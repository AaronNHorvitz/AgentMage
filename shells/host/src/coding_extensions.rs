//! Extensions through the development catalog host (Decision 0132).
//!
//! The catalog host keeps one extension catalog for its state root, as an
//! owner state in its operational store. Extensions are kept per workspace
//! scope: each scope holds the keys the person trusts in it, the extensions
//! installed in it and the revocation list it accepted last. Each operation
//! decodes the stored catalog, verifies every key, package and list again
//! through the capability package component, applies one change and commits
//! the next state under the revision it read. An extension is active unless
//! the scope's accepted list revokes it or it depends on an inactive one.
//! Nothing crosses scopes. Every refusal is closed and content-free and
//! changes nothing. An installed extension provides nothing to a coding run:
//! nothing it declares is granted or run, and nothing here opens a network
//! connection.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_engine::owner_state_store::{
    DurableOwnerStates, OwnerStateName, OwnerStateStoreError,
};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::capability_package::{
    CapabilityCompatibility, CapabilityPackageAdmission, CapabilityPackageError,
    CapabilityPackageManifest, CapabilityRevocationAdmission, CapabilityRevocationList,
    CapabilitySideEffect, VerifiedCapabilityRevocations, deactivate_revoked_entries,
    verify_capability_package, verify_capability_revocations,
};
use crate::capability_package_runtime::{
    CatalogCapabilityKind, PackageCatalogCandidate, PackageCatalogEntry, build_package_catalog,
};

const STATE_SCHEMA_VERSION: u16 = 1;
const MAX_SCOPES: usize = 64;
const MAX_KEYS_PER_SCOPE: usize = 32;
const MAX_EXTENSIONS_PER_SCOPE: usize = 256;
const MAX_STATE_BYTES: usize = 16 * 1024 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;
/// Largest trust statement file the CLI reads.
pub const MAX_EXTENSION_TRUST_BYTES: u64 = 4 * 1024;
/// Largest package or revocation list file the CLI reads.
pub const MAX_EXTENSION_FILE_BYTES: u64 = 1024 * 1024;
/// Largest package source file the CLI hashes.
pub const MAX_EXTENSION_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
/// The package file of an extension directory.
pub const EXTENSION_PACKAGE_FILE: &str = "package.json";
/// The source file of an extension directory.
pub const EXTENSION_SOURCE_FILE: &str = "source";
const INSTALLED_REASON: &str = "capability-package.installed";
const DEPENDENCY_REASON: &str = "capability-package.dependency-inactive";

/// An optional member that must still be present, as `null` when absent.
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// The versions of the development catalog host's extension contract an
/// extension must declare exactly.
#[must_use]
pub fn extension_host_compatibility() -> CapabilityCompatibility {
    let version = || "1.0.0".to_owned();
    CapabilityCompatibility {
        kernel_version: version(),
        tool_protocol_version: version(),
        configuration_version: version(),
        memory_version: version(),
        storage_version: version(),
        shell_version: version(),
        policy_version: version(),
    }
}

/// The one role a trusted key has in a scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionKeyRole {
    /// Signs packages.
    PackageSigner,
    /// Issues revocation lists.
    RevocationIssuer,
}

impl ExtensionKeyRole {
    const fn name(self) -> &'static str {
        match self {
            Self::PackageSigner => "package-signer",
            Self::RevocationIssuer => "revocation-issuer",
        }
    }
}

/// A key the person trusts: its role, its identity and its Ed25519 public key
/// in lowercase hexadecimal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionTrustStatement {
    /// The key's role.
    pub role: ExtensionKeyRole,
    /// The identity a package's signer or a list's issuer names.
    pub key_id: String,
    /// The 32-byte public key, as 64 lowercase hexadecimal characters.
    pub public_key: String,
}

/// A sealed package manifest and its signature, as a package file holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedExtensionPackage {
    /// The sealed manifest.
    pub manifest: CapabilityPackageManifest,
    /// The signer's 64-byte signature, as 128 lowercase hexadecimal
    /// characters.
    pub signature: String,
}

/// A sealed revocation list and its signature, as a list file holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedRevocationList {
    /// The sealed list.
    pub list: CapabilityRevocationList,
    /// The issuer's 64-byte signature, as 128 lowercase hexadecimal
    /// characters.
    pub signature: String,
}

/// One extension request of a catalog client. Every member is required and
/// no other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExtensionRequest {
    /// Trusts one key in a scope.
    Trust {
        /// The scope.
        workspace_id: String,
        /// The key.
        statement: ExtensionTrustStatement,
    },
    /// Removes one trusted key from a scope.
    Distrust {
        /// The scope.
        workspace_id: String,
        /// SHA-256 of the key.
        key_sha256: String,
    },
    /// Installs one signed package into a scope.
    Install {
        /// The scope.
        workspace_id: String,
        /// The signed package.
        package: Box<SignedExtensionPackage>,
        /// SHA-256 of the source file the client read.
        observed_source_sha256: String,
        /// The one license the person allows for it.
        allowed_license: String,
    },
    /// Removes one extension from a scope.
    Uninstall {
        /// The scope.
        workspace_id: String,
        /// The extension's package identity.
        package_id: String,
    },
    /// Applies a signed revocation list to a scope.
    ApplyRevocations {
        /// The scope.
        workspace_id: String,
        /// The signed list.
        list: Box<SignedRevocationList>,
    },
    /// Every scope, or one.
    List {
        /// One scope, or every scope.
        #[serde(deserialize_with = "required_option")]
        workspace_id: Option<String>,
    },
}

/// A trusted key as the person sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionKeyView {
    /// Its role.
    pub role: ExtensionKeyRole,
    /// Its identity.
    pub key_id: String,
    /// SHA-256 of the public key.
    pub key_sha256: String,
}

/// Why a catalog entry is active or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionEntryReason {
    /// Installed and not revoked.
    Installed,
    /// The accepted list revokes its package, manifest or signer key.
    Revoked,
    /// Its manifest was not checked against the accepted list.
    RevocationUnchecked,
    /// An extension it depends on is inactive.
    DependencyInactive,
}

impl ExtensionEntryReason {
    fn of(reason: &str) -> Self {
        match reason {
            "capability-package.revoked" => Self::Revoked,
            "capability-package.revocation-unchecked" => Self::RevocationUnchecked,
            DEPENDENCY_REASON => Self::DependencyInactive,
            _ => Self::Installed,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Revoked => "revoked",
            Self::RevocationUnchecked => "revocation-unchecked",
            Self::DependencyInactive => "dependency-inactive",
        }
    }
}

/// One tool or skill an extension offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionEntryView {
    /// The tool or skill identity.
    pub capability_id: String,
    /// Tool or skill.
    pub kind: CatalogCapabilityKind,
    /// Whether it is active.
    pub active: bool,
    /// Why.
    pub reason: ExtensionEntryReason,
}

/// One installed extension as the person sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionView {
    /// Package identity.
    pub package_id: String,
    /// Package name, as the manifest states it.
    pub package_name: String,
    /// Version.
    pub version: String,
    /// Manifest digest.
    pub manifest_sha256: String,
    /// Signer identity.
    pub signer_id: String,
    /// License.
    pub license: String,
    /// Side effects the manifest requests; none is granted.
    pub side_effects: Vec<CapabilitySideEffect>,
    /// Whether the extension is active.
    pub active: bool,
    /// Why.
    pub reason: ExtensionEntryReason,
    /// Its tools and skills.
    pub entries: Vec<ExtensionEntryView>,
    /// When it was installed.
    pub installed_at: String,
}

/// The revocation list a scope accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionRevocationsView {
    /// List identity.
    pub list_id: String,
    /// List sequence.
    pub sequence: u64,
    /// List digest.
    pub list_sha256: String,
    /// SHA-256 of its issuer key.
    pub issuer_key_sha256: String,
}

/// One scope as the person sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionScopeView {
    /// The scope's workspace label.
    pub workspace_id: String,
    /// Its trusted keys.
    pub keys: Vec<ExtensionKeyView>,
    /// Its extensions.
    pub extensions: Vec<ExtensionView>,
    /// The list it accepted last.
    #[serde(deserialize_with = "required_option")]
    pub revocations: Option<ExtensionRevocationsView>,
}

/// The receipt of one change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionTransitionView {
    /// Catalog revision after the change.
    pub catalog_revision: u64,
    /// SHA-256 of the catalog state after the change.
    pub state_sha256: String,
    /// Digest of the person's decision.
    pub decision_sha256: String,
}

/// Content-free reason an extension request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionRefusal {
    /// A value is malformed or out of bounds.
    InvalidInput,
    /// A file is missing, linked, not a regular file, oversized or malformed.
    FileInvalid,
    /// The key is already trusted in the scope.
    AlreadyTrusted,
    /// The key's identity is already used for its role in the scope.
    KeyConflict,
    /// The key signs an installed extension or issued the accepted list.
    KeyInUse,
    /// No such key, extension or scope.
    NotFound,
    /// The package is already installed in the scope.
    AlreadyInstalled,
    /// No trusted signer key of the scope matches the package.
    UntrustedSigner,
    /// The package's signature does not verify.
    SignatureDenied,
    /// The manifest or source digest does not match.
    DigestMismatch,
    /// The license is not the one the person allowed.
    LicenseDenied,
    /// A dependency is not installed and active at its exact version.
    DependencyDenied,
    /// Another extension depends on it.
    DependencyInUse,
    /// The package declares another contract version.
    CompatibilityDenied,
    /// The scope's accepted list revokes the package.
    Revoked,
    /// Two extensions would offer the same tool or skill.
    CapabilityConflict,
    /// The scope does not trust the list's issuer key.
    RevocationsUntrusted,
    /// The list's shape, digest or signature does not verify.
    RevocationsInvalid,
    /// The list is older than, forked from or not the scope's accepted list.
    RevocationsStale,
    /// The catalog would exceed its bound.
    ResourceLimit,
    /// The host has no clock reading.
    ClockUnavailable,
    /// The store could not be opened, read or written.
    StoreUnavailable,
    /// The catalog changed underneath this operation.
    StoreConflict,
    /// The stored catalog failed its checks.
    StoreIntegrity,
}

impl ExtensionRefusal {
    /// Stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "extension.invalid-input",
            Self::FileInvalid => "extension.file-invalid",
            Self::AlreadyTrusted => "extension.already-trusted",
            Self::KeyConflict => "extension.key-conflict",
            Self::KeyInUse => "extension.key-in-use",
            Self::NotFound => "extension.not-found",
            Self::AlreadyInstalled => "extension.already-installed",
            Self::UntrustedSigner => "extension.untrusted-signer",
            Self::SignatureDenied => "extension.signature-denied",
            Self::DigestMismatch => "extension.digest-mismatch",
            Self::LicenseDenied => "extension.license-denied",
            Self::DependencyDenied => "extension.dependency-denied",
            Self::DependencyInUse => "extension.dependency-in-use",
            Self::CompatibilityDenied => "extension.compatibility-denied",
            Self::Revoked => "extension.revoked",
            Self::CapabilityConflict => "extension.capability-conflict",
            Self::RevocationsUntrusted => "extension.revocations-untrusted",
            Self::RevocationsInvalid => "extension.revocations-invalid",
            Self::RevocationsStale => "extension.revocations-stale",
            Self::ResourceLimit => "extension.resource-limit",
            Self::ClockUnavailable => "extension.clock-unavailable",
            Self::StoreUnavailable => "extension.store-unavailable",
            Self::StoreConflict => "extension.store-conflict",
            Self::StoreIntegrity => "extension.store-integrity",
        }
    }

    const fn of_admission(error: CapabilityPackageError) -> Self {
        match error {
            CapabilityPackageError::HashMismatch => Self::DigestMismatch,
            CapabilityPackageError::SignatureDenied => Self::SignatureDenied,
            CapabilityPackageError::LicenseDenied => Self::LicenseDenied,
            CapabilityPackageError::ProvenanceDenied => Self::UntrustedSigner,
            CapabilityPackageError::DependencyDenied => Self::DependencyDenied,
            CapabilityPackageError::CompatibilityDenied => Self::CompatibilityDenied,
            CapabilityPackageError::Revoked => Self::Revoked,
            CapabilityPackageError::InvalidManifest
            | CapabilityPackageError::ApprovalDenied
            | CapabilityPackageError::ScopeDenied
            | CapabilityPackageError::RevocationDenied => Self::InvalidInput,
        }
    }

    const fn of_store(error: OwnerStateStoreError) -> Self {
        match error {
            OwnerStateStoreError::Stale => Self::StoreConflict,
            OwnerStateStoreError::Integrity => Self::StoreIntegrity,
            OwnerStateStoreError::ResourceLimit => Self::ResourceLimit,
            OwnerStateStoreError::InvalidInput
            | OwnerStateStoreError::Storage
            | OwnerStateStoreError::Unavailable => Self::StoreUnavailable,
        }
    }
}

/// One extension answer of the catalog host. Every member is required and no
/// other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ExtensionAnswer {
    /// The key is trusted in the scope.
    Trusted {
        /// The scope.
        workspace_id: String,
        /// The key.
        key: ExtensionKeyView,
        /// The change.
        receipt: ExtensionTransitionView,
    },
    /// The key is no longer trusted in the scope.
    Distrusted {
        /// The scope.
        workspace_id: String,
        /// The key.
        key: ExtensionKeyView,
        /// The change.
        receipt: ExtensionTransitionView,
    },
    /// The extension is installed in the scope.
    Installed {
        /// The scope.
        workspace_id: String,
        /// The extension.
        extension: ExtensionView,
        /// The change.
        receipt: ExtensionTransitionView,
    },
    /// The extension left the scope.
    Uninstalled {
        /// The scope.
        workspace_id: String,
        /// The extension's package identity.
        package_id: String,
        /// The change.
        receipt: ExtensionTransitionView,
    },
    /// The scope accepted the list.
    RevocationsApplied {
        /// The scope.
        workspace_id: String,
        /// The accepted list.
        revocations: ExtensionRevocationsView,
        /// Every extension of the scope the list made inactive.
        deactivated: Vec<String>,
        /// The change.
        receipt: ExtensionTransitionView,
    },
    /// The list is the scope's accepted list already; nothing changed.
    RevocationsUnchanged {
        /// The scope.
        workspace_id: String,
        /// The accepted list.
        revocations: ExtensionRevocationsView,
    },
    /// The requested scopes.
    Listed {
        /// The scopes, by workspace label.
        scopes: Vec<ExtensionScopeView>,
        /// Catalog revision.
        catalog_revision: u64,
    },
    /// The request was refused and changed nothing.
    Refused {
        /// Why.
        refusal: ExtensionRefusal,
    },
}

/// One installed extension as the catalog state keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledState {
    package: SignedExtensionPackage,
    installed_at: String,
    decision_sha256: String,
}

/// One scope as the catalog state keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeState {
    workspace_id: String,
    keys: Vec<ExtensionTrustStatement>,
    extensions: Vec<InstalledState>,
    #[serde(deserialize_with = "required_option")]
    accepted: Option<SignedRevocationList>,
}

/// The catalog as its owner state keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogState {
    schema_version: u16,
    scopes: Vec<ScopeState>,
}

/// A trusted key, decoded.
#[derive(Clone, Debug)]
struct TrustedKey {
    statement: ExtensionTrustStatement,
    key_sha256: String,
    bytes: [u8; 32],
}

/// One scope, every part verified.
#[derive(Debug)]
struct Scope {
    keys: Vec<TrustedKey>,
    extensions: BTreeMap<String, InstalledState>,
    accepted: Option<SignedRevocationList>,
    revocations: VerifiedCapabilityRevocations,
}

impl Scope {
    fn empty() -> Result<Self, ExtensionRefusal> {
        Ok(Self {
            keys: Vec::new(),
            extensions: BTreeMap::new(),
            accepted: None,
            revocations: VerifiedCapabilityRevocations::never_accepted(None)
                .map_err(|_| ExtensionRefusal::StoreIntegrity)?,
        })
    }

    fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.extensions.is_empty() && self.accepted.is_none()
    }

    fn key(&self, role: ExtensionKeyRole, key_sha256: &str) -> Option<&TrustedKey> {
        self.keys
            .iter()
            .find(|key| key.statement.role == role && key.key_sha256 == key_sha256)
    }

    fn trusted_ids(&self, role: ExtensionKeyRole) -> BTreeSet<String> {
        self.keys
            .iter()
            .filter(|key| key.statement.role == role)
            .map(|key| key.statement.key_id.clone())
            .collect()
    }

    fn trusted_digests(&self, role: ExtensionKeyRole) -> BTreeSet<String> {
        self.keys
            .iter()
            .filter(|key| key.statement.role == role)
            .map(|key| key.key_sha256.clone())
            .collect()
    }

    /// Verifies one package against this scope's keys, the host's
    /// compatibility and the given revocations; `observed` is the source
    /// digest the client read.
    fn admit(
        &self,
        package: &SignedExtensionPackage,
        observed: &str,
        allowed_license: &str,
        revocations: &VerifiedCapabilityRevocations,
    ) -> Result<(), ExtensionRefusal> {
        let manifest = &package.manifest;
        let signature =
            decode_signature(&package.signature).ok_or(ExtensionRefusal::InvalidInput)?;
        let signer = self
            .keys
            .iter()
            .find(|key| {
                key.statement.role == ExtensionKeyRole::PackageSigner
                    && key.statement.key_id == manifest.signer_id
                    && key.key_sha256 == manifest.signer_public_key_sha256
            })
            .ok_or(ExtensionRefusal::UntrustedSigner)?;
        // Each dependency is locked to the installed extension of the scope
        // at its exact version and manifest digest.
        let locks = manifest
            .dependencies
            .iter()
            .filter(|dependency| {
                self.extensions
                    .get(&dependency.package_id)
                    .is_some_and(|installed| {
                        installed.package.manifest.version == dependency.exact_version
                            && installed.package.manifest.manifest_sha256
                                == dependency.manifest_sha256
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        verify_capability_package(CapabilityPackageAdmission {
            manifest: manifest.clone(),
            signature: &signature,
            public_key: &signer.bytes,
            expected_compatibility: &extension_host_compatibility(),
            allowed_licenses: &BTreeSet::from([allowed_license.to_owned()]),
            trusted_signers: &self.trusted_ids(ExtensionKeyRole::PackageSigner),
            observed_source_sha256: observed,
            dependency_locks: &locks,
            revocations,
        })
        .map(|_| ())
        .map_err(ExtensionRefusal::of_admission)
    }

    /// The component's catalog entries of every extension, with revoked
    /// entries and entries of extensions whose dependency is inactive marked
    /// inactive, and each extension's own state.
    fn activity(
        &self,
    ) -> Result<
        (
            Vec<PackageCatalogEntry>,
            BTreeMap<String, ExtensionEntryReason>,
        ),
        ExtensionRefusal,
    > {
        let candidates = self
            .extensions
            .values()
            .flat_map(|installed| {
                let manifest = &installed.package.manifest;
                manifest
                    .tools
                    .iter()
                    .map(|tool| (tool, CatalogCapabilityKind::Tool))
                    .chain(
                        manifest
                            .skills
                            .iter()
                            .map(|skill| (skill, CatalogCapabilityKind::Skill)),
                    )
                    .map(|(capability_id, kind)| PackageCatalogCandidate {
                        package_id: manifest.package_id.clone(),
                        manifest_sha256: manifest.manifest_sha256.clone(),
                        capability_id: capability_id.clone(),
                        kind,
                        verified: true,
                        enabled: true,
                        removal_pending: false,
                        activation_reason: INSTALLED_REASON.to_owned(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut entries = build_package_catalog(candidates, false)
            .map_err(|_| ExtensionRefusal::CapabilityConflict)?;
        let manifests = self
            .extensions
            .values()
            .map(|installed| installed.package.manifest.clone())
            .collect::<Vec<_>>();
        deactivate_revoked_entries(&mut entries, &manifests, &self.revocations);
        let mut states = self
            .extensions
            .values()
            .map(|installed| {
                let manifest = &installed.package.manifest;
                let reason = if self.revocations.revokes(manifest) {
                    ExtensionEntryReason::Revoked
                } else {
                    ExtensionEntryReason::Installed
                };
                (manifest.package_id.clone(), reason)
            })
            .collect::<BTreeMap<_, _>>();
        // An extension whose dependency is inactive is inactive too, until no
        // state changes; each pass marks at least one more, so this ends.
        loop {
            let inactive = states
                .iter()
                .filter(|(_, reason)| **reason != ExtensionEntryReason::Installed)
                .map(|(package_id, _)| package_id.clone())
                .collect::<BTreeSet<_>>();
            let newly = self
                .extensions
                .values()
                .filter(|installed| {
                    let manifest = &installed.package.manifest;
                    !inactive.contains(&manifest.package_id)
                        && manifest
                            .dependencies
                            .iter()
                            .any(|dependency| inactive.contains(&dependency.package_id))
                })
                .map(|installed| installed.package.manifest.package_id.clone())
                .collect::<Vec<_>>();
            if newly.is_empty() {
                break;
            }
            for package_id in newly {
                states.insert(package_id, ExtensionEntryReason::DependencyInactive);
            }
        }
        for entry in &mut entries {
            if entry.active
                && states.get(&entry.package_id) == Some(&ExtensionEntryReason::DependencyInactive)
            {
                entry.active = false;
                DEPENDENCY_REASON.clone_into(&mut entry.activation_reason);
            }
        }
        Ok((entries, states))
    }

    fn revocations_view(&self) -> Option<ExtensionRevocationsView> {
        self.accepted
            .as_ref()
            .map(|accepted| revocations_view(&accepted.list))
    }

    fn view(&self, workspace_id: &str) -> Result<ExtensionScopeView, ExtensionRefusal> {
        let (entries, states) = self.activity()?;
        let extensions = self
            .extensions
            .values()
            .map(|installed| {
                let manifest = &installed.package.manifest;
                let reason = states
                    .get(&manifest.package_id)
                    .copied()
                    .unwrap_or(ExtensionEntryReason::Installed);
                ExtensionView {
                    package_id: manifest.package_id.clone(),
                    package_name: manifest.package_name.clone(),
                    version: manifest.version.clone(),
                    manifest_sha256: manifest.manifest_sha256.clone(),
                    signer_id: manifest.signer_id.clone(),
                    license: manifest.license.clone(),
                    side_effects: manifest.side_effects.clone(),
                    active: reason == ExtensionEntryReason::Installed,
                    reason,
                    entries: entries
                        .iter()
                        .filter(|entry| entry.package_id == manifest.package_id)
                        .map(|entry| ExtensionEntryView {
                            capability_id: entry.capability_id.clone(),
                            kind: entry.kind,
                            active: entry.active,
                            reason: ExtensionEntryReason::of(&entry.activation_reason),
                        })
                        .collect(),
                    installed_at: installed.installed_at.clone(),
                }
            })
            .collect();
        Ok(ExtensionScopeView {
            workspace_id: workspace_id.to_owned(),
            keys: self.keys.iter().map(key_view).collect(),
            extensions,
            revocations: self.revocations_view(),
        })
    }
}

fn key_view(key: &TrustedKey) -> ExtensionKeyView {
    ExtensionKeyView {
        role: key.statement.role,
        key_id: key.statement.key_id.clone(),
        key_sha256: key.key_sha256.clone(),
    }
}

fn revocations_view(list: &CapabilityRevocationList) -> ExtensionRevocationsView {
    ExtensionRevocationsView {
        list_id: list.list_id.clone(),
        sequence: list.sequence,
        list_sha256: list.list_sha256.clone(),
        issuer_key_sha256: list.issuer_public_key_sha256.clone(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// Whether a value has the shape `YYYY-MM-DDTHH:MM:SSZ` with each part in
/// range.
fn timestamp_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    let number = |range: std::ops::Range<usize>| {
        bytes.get(range).and_then(|part| {
            part.iter().all(u8::is_ascii_digit).then(|| {
                part.iter()
                    .fold(0_u32, |total, digit| total * 10 + u32::from(digit - b'0'))
            })
        })
    };
    bytes.len() == 20
        && [
            (4, b'-'),
            (7, b'-'),
            (10, b'T'),
            (13, b':'),
            (16, b':'),
            (19, b'Z'),
        ]
        .iter()
        .all(|(index, separator)| bytes[*index] == *separator)
        && number(0..4).is_some_and(|year| year >= 1)
        && number(5..7).is_some_and(|month| (1..=12).contains(&month))
        && number(8..10).is_some_and(|day| (1..=31).contains(&day))
        && number(11..13).is_some_and(|hour| hour < 24)
        && number(14..16).is_some_and(|minute| minute < 60)
        && number(17..19).is_some_and(|second| second < 60)
}

fn lowercase_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn decode_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    if !lowercase_hex(value, N * 2) {
        return None;
    }
    let mut bytes = [0_u8; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

fn decode_signature(value: &str) -> Option<[u8; 64]> {
    decode_hex::<64>(value)
}

/// Whether a value is a SHA-256 digest in lowercase hexadecimal.
#[must_use]
pub fn extension_sha256(value: &str) -> bool {
    lowercase_hex(value, 64)
}

/// Whether a value is an identity under the capability package contract:
/// ASCII letters, digits, dots, underscores, colons and hyphens, at most 128
/// bytes.
#[must_use]
pub fn extension_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

/// The SHA-256 of a trust statement's public key, when it names a valid
/// Ed25519 public key.
#[must_use]
pub fn extension_key_sha256(statement: &ExtensionTrustStatement) -> Option<String> {
    decode_key(statement).map(|(_, digest)| digest)
}

fn decode_key(statement: &ExtensionTrustStatement) -> Option<([u8; 32], String)> {
    let bytes = decode_hex::<32>(&statement.public_key)?;
    VerifyingKey::from_bytes(&bytes).ok()?;
    extension_identifier(&statement.key_id).then(|| (bytes, sha256_hex(&bytes)))
}

/// The digest of the person's decision: the SHA-256 of the exact request in
/// its wire form.
#[must_use]
pub fn extension_decision_sha256(request: &ExtensionRequest) -> Option<String> {
    serde_json::to_vec(request)
        .ok()
        .map(|bytes| sha256_hex(&bytes))
}

fn workspace_of(request: &ExtensionRequest) -> Option<&str> {
    match request {
        ExtensionRequest::Trust { workspace_id, .. }
        | ExtensionRequest::Distrust { workspace_id, .. }
        | ExtensionRequest::Install { workspace_id, .. }
        | ExtensionRequest::Uninstall { workspace_id, .. }
        | ExtensionRequest::ApplyRevocations { workspace_id, .. } => Some(workspace_id),
        ExtensionRequest::List { workspace_id } => workspace_id.as_deref(),
    }
}

/// Whether an answer acknowledges exactly the request that was sent: the
/// matching kind, the decision digest of that request, its scope, and the
/// key, extension or list it named.
#[must_use]
pub fn extension_answer_acknowledges(request: &ExtensionRequest, answer: &ExtensionAnswer) -> bool {
    let decision = extension_decision_sha256(request);
    let decided =
        |receipt: &ExtensionTransitionView| Some(&receipt.decision_sha256) == decision.as_ref();
    let scope = workspace_of(request);
    match (request, answer) {
        (_, ExtensionAnswer::Refused { .. }) => true,
        (
            ExtensionRequest::Trust { statement, .. },
            ExtensionAnswer::Trusted {
                workspace_id,
                key,
                receipt,
            },
        ) => {
            decided(receipt)
                && scope == Some(workspace_id.as_str())
                && key.role == statement.role
                && key.key_id == statement.key_id
                && extension_key_sha256(statement).as_ref() == Some(&key.key_sha256)
        }
        (
            ExtensionRequest::Distrust { key_sha256, .. },
            ExtensionAnswer::Distrusted {
                workspace_id,
                key,
                receipt,
            },
        ) => {
            decided(receipt)
                && scope == Some(workspace_id.as_str())
                && key.key_sha256 == *key_sha256
        }
        (
            ExtensionRequest::Install { package, .. },
            ExtensionAnswer::Installed {
                workspace_id,
                extension,
                receipt,
            },
        ) => {
            decided(receipt)
                && scope == Some(workspace_id.as_str())
                && extension.package_id == package.manifest.package_id
                && extension.version == package.manifest.version
                && extension.manifest_sha256 == package.manifest.manifest_sha256
                && extension.active
        }
        (
            ExtensionRequest::Uninstall { package_id, .. },
            ExtensionAnswer::Uninstalled {
                workspace_id,
                package_id: removed,
                receipt,
            },
        ) => decided(receipt) && scope == Some(workspace_id.as_str()) && removed == package_id,
        (
            ExtensionRequest::ApplyRevocations { list, .. },
            ExtensionAnswer::RevocationsApplied {
                workspace_id,
                revocations,
                receipt,
                ..
            },
        ) => {
            decided(receipt)
                && scope == Some(workspace_id.as_str())
                && *revocations == revocations_view(&list.list)
        }
        (
            ExtensionRequest::ApplyRevocations { list, .. },
            ExtensionAnswer::RevocationsUnchanged {
                workspace_id,
                revocations,
            },
        ) => scope == Some(workspace_id.as_str()) && *revocations == revocations_view(&list.list),
        (ExtensionRequest::List { workspace_id }, ExtensionAnswer::Listed { scopes, .. }) => {
            workspace_id.as_ref().is_none_or(|workspace| {
                scopes.len() <= 1 && scopes.iter().all(|scope| scope.workspace_id == *workspace)
            })
        }
        _ => false,
    }
}

/// Decodes and verifies a stored catalog. The state must be the exact
/// encoding this owner writes.
fn decode_catalog(bytes: &[u8]) -> Result<BTreeMap<String, Scope>, ExtensionRefusal> {
    let integrity = ExtensionRefusal::StoreIntegrity;
    let state = serde_json::from_slice::<CatalogState>(bytes).map_err(|_| integrity)?;
    if serde_json::to_vec(&state).ok().as_deref() != Some(bytes)
        || state.schema_version != STATE_SCHEMA_VERSION
        || state.scopes.len() > MAX_SCOPES
        || state
            .scopes
            .windows(2)
            .any(|pair| pair[0].workspace_id >= pair[1].workspace_id)
    {
        return Err(integrity);
    }
    state
        .scopes
        .into_iter()
        .map(|scope| {
            let workspace_id = scope.workspace_id.clone();
            Ok((workspace_id, verify_scope(scope)?))
        })
        .collect()
}

fn verify_scope(state: ScopeState) -> Result<Scope, ExtensionRefusal> {
    let integrity = ExtensionRefusal::StoreIntegrity;
    if !crate::coding_memory::portable_memory_label(&state.workspace_id)
        || state.keys.len() > MAX_KEYS_PER_SCOPE
        || state.extensions.len() > MAX_EXTENSIONS_PER_SCOPE
    {
        return Err(integrity);
    }
    let mut scope = Scope::empty()?;
    for statement in state.keys {
        let (bytes, key_sha256) = decode_key(&statement).ok_or(integrity)?;
        if scope
            .keys
            .last()
            .is_some_and(|last| last.key_sha256 >= key_sha256)
            || scope.keys.iter().any(|key| {
                key.statement.role == statement.role && key.statement.key_id == statement.key_id
            })
        {
            return Err(integrity);
        }
        scope.keys.push(TrustedKey {
            statement,
            key_sha256,
            bytes,
        });
    }
    if let Some(accepted) = state.accepted {
        scope.revocations = verify_list(&scope, &accepted, None).map_err(|_| integrity)?;
        scope.accepted = Some(accepted);
    }
    // Each extension verifies again as installed: its own signature, signer,
    // license, source digest, compatibility and the locks of its
    // dependencies. A later list may revoke it; revocation decides whether
    // it is active, not whether it is kept.
    let mut previous: Option<String> = None;
    for installed in state.extensions {
        let package_id = installed.package.manifest.package_id.clone();
        if previous
            .as_ref()
            .is_some_and(|previous| *previous >= package_id)
            || !timestamp_shape(&installed.installed_at)
            || !extension_sha256(&installed.decision_sha256)
        {
            return Err(integrity);
        }
        previous = Some(package_id.clone());
        scope.extensions.insert(package_id, installed);
    }
    let unrevoked = VerifiedCapabilityRevocations::never_accepted(None).map_err(|_| integrity)?;
    for installed in scope.extensions.values() {
        let manifest = &installed.package.manifest;
        scope
            .admit(
                &installed.package,
                &manifest.source_sha256,
                &manifest.license,
                &unrevoked,
            )
            .map_err(|_| integrity)?;
    }
    // A dependency lock names the exact manifest digest of an installed
    // extension, and a digest covers the locks, so dependencies cannot form a
    // cycle.
    scope.activity().map_err(|_| integrity)?;
    if scope.is_empty() {
        return Err(integrity);
    }
    Ok(scope)
}

/// Verifies a list against the scope's issuer key and, when given, the
/// scope's accepted list.
fn verify_list(
    scope: &Scope,
    list: &SignedRevocationList,
    last_accepted: Option<&SignedRevocationList>,
) -> Result<VerifiedCapabilityRevocations, ExtensionRefusal> {
    let issuer = scope
        .key(
            ExtensionKeyRole::RevocationIssuer,
            &list.list.issuer_public_key_sha256,
        )
        .ok_or(ExtensionRefusal::RevocationsUntrusted)?;
    let signature =
        decode_signature(&list.signature).ok_or(ExtensionRefusal::RevocationsInvalid)?;
    let checkpoint =
        last_accepted.map(
            |accepted| crate::capability_package::CapabilityRevocationCheckpoint {
                list_id: accepted.list.list_id.clone(),
                sequence: accepted.list.sequence,
                list_sha256: accepted.list.list_sha256.clone(),
            },
        );
    let trusted = scope.trusted_digests(ExtensionKeyRole::RevocationIssuer);
    let admit = |last| {
        verify_capability_revocations(CapabilityRevocationAdmission {
            list: list.list.clone(),
            signature: &signature,
            issuer_public_key: &issuer.bytes,
            trusted_issuer_key_sha256s: &trusted,
            last_accepted: last,
        })
    };
    // The list must verify on its own before it is compared with the
    // accepted one, so a stale list is told apart from an invalid one.
    admit(None).map_err(|_| ExtensionRefusal::RevocationsInvalid)?;
    admit(checkpoint.as_ref()).map_err(|_| ExtensionRefusal::RevocationsStale)
}

fn encode_catalog(scopes: &BTreeMap<String, Scope>) -> Result<Vec<u8>, ExtensionRefusal> {
    let state = CatalogState {
        schema_version: STATE_SCHEMA_VERSION,
        scopes: scopes
            .iter()
            .filter(|(_, scope)| !scope.is_empty())
            .map(|(workspace_id, scope)| ScopeState {
                workspace_id: workspace_id.clone(),
                keys: scope.keys.iter().map(|key| key.statement.clone()).collect(),
                extensions: scope.extensions.values().cloned().collect(),
                accepted: scope.accepted.clone(),
            })
            .collect(),
    };
    if state.scopes.len() > MAX_SCOPES {
        return Err(ExtensionRefusal::ResourceLimit);
    }
    let bytes = serde_json::to_vec(&state).map_err(|_| ExtensionRefusal::InvalidInput)?;
    if bytes.len() > MAX_STATE_BYTES {
        return Err(ExtensionRefusal::ResourceLimit);
    }
    Ok(bytes)
}

/// Answers one extension request. `open` opens the owner states for the
/// operation; `now_epoch_ms` is the host's clock, if it has one.
pub fn answer_extension(
    request: &ExtensionRequest,
    open: &mut dyn FnMut() -> Result<DurableOwnerStates, ExtensionRefusal>,
    now_epoch_ms: Option<u64>,
) -> ExtensionAnswer {
    match decide(request, open, now_epoch_ms) {
        Ok(answer) => answer,
        Err(refusal) => ExtensionAnswer::Refused { refusal },
    }
}

fn decide(
    request: &ExtensionRequest,
    open: &mut dyn FnMut() -> Result<DurableOwnerStates, ExtensionRefusal>,
    now_epoch_ms: Option<u64>,
) -> Result<ExtensionAnswer, ExtensionRefusal> {
    validate(request)?;
    let decision_sha256 =
        extension_decision_sha256(request).ok_or(ExtensionRefusal::InvalidInput)?;
    let installed_at = match request {
        ExtensionRequest::Install { .. } => Some(
            now_epoch_ms
                .and_then(crate::coding_memory::memory_timestamp)
                .ok_or(ExtensionRefusal::ClockUnavailable)?,
        ),
        _ => None,
    };
    let states = open()?;
    let stored = states
        .load(OwnerStateName::ExtensionCatalog)
        .map_err(ExtensionRefusal::of_store)?;
    let mut scopes = if stored.revision == 0 {
        BTreeMap::new()
    } else {
        decode_catalog(&stored.state)?
    };
    if let ExtensionRequest::List { workspace_id } = request {
        let scopes = scopes
            .iter()
            .filter(|(workspace, _)| {
                workspace_id
                    .as_ref()
                    .is_none_or(|requested| *workspace == requested)
            })
            .map(|(workspace, scope)| scope.view(workspace))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(ExtensionAnswer::Listed {
            scopes,
            catalog_revision: stored.revision,
        });
    }
    let workspace_id = workspace_of(request)
        .ok_or(ExtensionRefusal::InvalidInput)?
        .to_owned();
    if !scopes.contains_key(&workspace_id) {
        if scopes.len() >= MAX_SCOPES {
            return Err(ExtensionRefusal::ResourceLimit);
        }
        scopes.insert(workspace_id.clone(), Scope::empty()?);
    }
    let scope = scopes
        .get_mut(&workspace_id)
        .ok_or(ExtensionRefusal::StoreUnavailable)?;
    let change = change_scope(request, scope, installed_at)?;
    let Some(change) = change else {
        let revocations = scope
            .revocations_view()
            .ok_or(ExtensionRefusal::StoreUnavailable)?;
        return Ok(ExtensionAnswer::RevocationsUnchanged {
            workspace_id,
            revocations,
        });
    };
    let view = match &change {
        Change::Installed(package_id) => Some(
            scope
                .view(&workspace_id)?
                .extensions
                .into_iter()
                .find(|extension| extension.package_id == *package_id)
                .ok_or(ExtensionRefusal::StoreUnavailable)?,
        ),
        _ => None,
    };
    let state = encode_catalog(&scopes)?;
    let catalog_revision = states
        .commit(OwnerStateName::ExtensionCatalog, stored.revision, &state)
        .map_err(ExtensionRefusal::of_store)?;
    let receipt = ExtensionTransitionView {
        catalog_revision,
        state_sha256: sha256_hex(&state),
        decision_sha256,
    };
    Ok(match change {
        Change::Trusted(key) => ExtensionAnswer::Trusted {
            workspace_id,
            key,
            receipt,
        },
        Change::Distrusted(key) => ExtensionAnswer::Distrusted {
            workspace_id,
            key,
            receipt,
        },
        Change::Installed(_) => ExtensionAnswer::Installed {
            workspace_id,
            extension: view.ok_or(ExtensionRefusal::StoreUnavailable)?,
            receipt,
        },
        Change::Uninstalled(package_id) => ExtensionAnswer::Uninstalled {
            workspace_id,
            package_id,
            receipt,
        },
        Change::Applied(revocations, deactivated) => ExtensionAnswer::RevocationsApplied {
            workspace_id,
            revocations,
            deactivated,
            receipt,
        },
    })
}

/// What one change did.
enum Change {
    Trusted(ExtensionKeyView),
    Distrusted(ExtensionKeyView),
    Installed(String),
    Uninstalled(String),
    Applied(ExtensionRevocationsView, Vec<String>),
}

/// Applies one change to one scope; none when the list is the scope's
/// accepted list already.
fn change_scope(
    request: &ExtensionRequest,
    scope: &mut Scope,
    installed_at: Option<String>,
) -> Result<Option<Change>, ExtensionRefusal> {
    let decision_sha256 =
        extension_decision_sha256(request).ok_or(ExtensionRefusal::InvalidInput)?;
    match request {
        ExtensionRequest::Trust { statement, .. } => {
            let (bytes, key_sha256) =
                decode_key(statement).ok_or(ExtensionRefusal::InvalidInput)?;
            if scope.keys.iter().any(|key| key.key_sha256 == key_sha256) {
                return Err(ExtensionRefusal::AlreadyTrusted);
            }
            if scope.keys.iter().any(|key| {
                key.statement.role == statement.role && key.statement.key_id == statement.key_id
            }) {
                return Err(ExtensionRefusal::KeyConflict);
            }
            if scope.keys.len() >= MAX_KEYS_PER_SCOPE {
                return Err(ExtensionRefusal::ResourceLimit);
            }
            let key = TrustedKey {
                statement: statement.clone(),
                key_sha256,
                bytes,
            };
            let view = key_view(&key);
            let position = scope
                .keys
                .partition_point(|kept| kept.key_sha256 < key.key_sha256);
            scope.keys.insert(position, key);
            Ok(Some(Change::Trusted(view)))
        }
        ExtensionRequest::Distrust { key_sha256, .. } => {
            let position = scope
                .keys
                .iter()
                .position(|key| key.key_sha256 == *key_sha256)
                .ok_or(ExtensionRefusal::NotFound)?;
            let key = &scope.keys[position];
            let signs = key.statement.role == ExtensionKeyRole::PackageSigner
                && scope.extensions.values().any(|installed| {
                    installed.package.manifest.signer_public_key_sha256 == *key_sha256
                });
            let issued = key.statement.role == ExtensionKeyRole::RevocationIssuer
                && scope
                    .accepted
                    .as_ref()
                    .is_some_and(|accepted| accepted.list.issuer_public_key_sha256 == *key_sha256);
            if signs || issued {
                return Err(ExtensionRefusal::KeyInUse);
            }
            let removed = scope.keys.remove(position);
            Ok(Some(Change::Distrusted(key_view(&removed))))
        }
        ExtensionRequest::Install {
            package,
            observed_source_sha256,
            allowed_license,
            ..
        } => {
            let manifest = &package.manifest;
            if scope.extensions.contains_key(&manifest.package_id) {
                return Err(ExtensionRefusal::AlreadyInstalled);
            }
            if scope.extensions.len() >= MAX_EXTENSIONS_PER_SCOPE {
                return Err(ExtensionRefusal::ResourceLimit);
            }
            // A dependency must be active, not only installed.
            let (_, states) = scope.activity()?;
            if manifest.dependencies.iter().any(|dependency| {
                states.get(&dependency.package_id) != Some(&ExtensionEntryReason::Installed)
            }) {
                return Err(ExtensionRefusal::DependencyDenied);
            }
            scope.admit(
                package,
                observed_source_sha256,
                allowed_license,
                &scope.revocations,
            )?;
            scope.extensions.insert(
                manifest.package_id.clone(),
                InstalledState {
                    package: (**package).clone(),
                    installed_at: installed_at.ok_or(ExtensionRefusal::ClockUnavailable)?,
                    decision_sha256,
                },
            );
            // Two extensions may not offer the same tool or skill. A refusal
            // discards this decoded copy; nothing is committed.
            scope.activity()?;
            Ok(Some(Change::Installed(manifest.package_id.clone())))
        }
        ExtensionRequest::Uninstall { package_id, .. } => {
            if !scope.extensions.contains_key(package_id) {
                return Err(ExtensionRefusal::NotFound);
            }
            if scope.extensions.values().any(|installed| {
                installed
                    .package
                    .manifest
                    .dependencies
                    .iter()
                    .any(|dependency| dependency.package_id == *package_id)
            }) {
                return Err(ExtensionRefusal::DependencyInUse);
            }
            scope.extensions.remove(package_id);
            Ok(Some(Change::Uninstalled(package_id.clone())))
        }
        ExtensionRequest::ApplyRevocations { list, .. } => {
            let verified = verify_list(scope, list, scope.accepted.as_ref())?;
            if scope.accepted.as_ref().is_some_and(|accepted| {
                accepted.list.list_sha256 == list.list.list_sha256
                    && accepted.list.sequence == list.list.sequence
            }) {
                return Ok(None);
            }
            let (_, before) = scope.activity()?;
            scope.revocations = verified;
            scope.accepted = Some((**list).clone());
            let (_, after) = scope.activity()?;
            let deactivated = after
                .iter()
                .filter(|(package_id, reason)| {
                    **reason != ExtensionEntryReason::Installed
                        && before.get(*package_id) == Some(&ExtensionEntryReason::Installed)
                })
                .map(|(package_id, _)| package_id.clone())
                .collect();
            Ok(Some(Change::Applied(
                revocations_view(&list.list),
                deactivated,
            )))
        }
        ExtensionRequest::List { .. } => Err(ExtensionRefusal::InvalidInput),
    }
}

/// Shapes and bounds of a request, before any store is opened.
fn validate(request: &ExtensionRequest) -> Result<(), ExtensionRefusal> {
    let label = crate::coding_memory::portable_memory_label;
    let valid = match request {
        ExtensionRequest::Trust {
            workspace_id,
            statement,
        } => label(workspace_id) && decode_key(statement).is_some(),
        ExtensionRequest::Distrust {
            workspace_id,
            key_sha256,
        } => label(workspace_id) && extension_sha256(key_sha256),
        ExtensionRequest::Install {
            workspace_id,
            package,
            observed_source_sha256,
            allowed_license,
        } => {
            label(workspace_id)
                && decode_signature(&package.signature).is_some()
                && extension_identifier(&package.manifest.package_id)
                && extension_sha256(observed_source_sha256)
                && extension_identifier(allowed_license)
        }
        ExtensionRequest::Uninstall {
            workspace_id,
            package_id,
        } => label(workspace_id) && extension_identifier(package_id),
        ExtensionRequest::ApplyRevocations { workspace_id, list } => {
            label(workspace_id) && decode_signature(&list.signature).is_some()
        }
        ExtensionRequest::List { workspace_id } => workspace_id.as_deref().is_none_or(label),
    };
    if valid {
        Ok(())
    } else {
        Err(ExtensionRefusal::InvalidInput)
    }
}

/// Text of a manifest's free text for a terminal, escaped.
fn escaped(text: &str) -> String {
    text.escape_debug().to_string()
}

fn short(digest: &str) -> &str {
    &digest[..digest.len().min(12)]
}

fn render_key(workspace_id: &str, key: &ExtensionKeyView) -> String {
    format!(
        "extension key {} {} ({}) in {workspace_id}\n",
        key.role.name(),
        key.key_id,
        short(&key.key_sha256)
    )
}

fn render_extension(workspace_id: &str, extension: &ExtensionView, json: bool) -> String {
    if json {
        return format!(
            "{}\n",
            serde_json::json!({
                "type": "extension",
                "workspace_id": workspace_id,
                "extension": extension,
            })
        );
    }
    let mut output = format!(
        "extension {} {} [{}] in {workspace_id}; signer {}; license {}; manifest {}; installed {}\n",
        extension.package_id,
        extension.version,
        if extension.active {
            "active".to_owned()
        } else {
            format!("inactive: {}", extension.reason.name())
        },
        extension.signer_id,
        extension.license,
        short(&extension.manifest_sha256),
        extension.installed_at
    );
    let _ = writeln!(output, "  | {}", escaped(&extension.package_name));
    for entry in &extension.entries {
        let _ = writeln!(
            output,
            "  {} {} [{}]",
            match entry.kind {
                CatalogCapabilityKind::Tool => "tool",
                CatalogCapabilityKind::Skill => "skill",
            },
            entry.capability_id,
            if entry.active {
                "active"
            } else {
                entry.reason.name()
            }
        );
    }
    if !extension.side_effects.is_empty() {
        let effects = extension
            .side_effects
            .iter()
            .map(|effect| {
                serde_json::to_value(effect)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        let _ = writeln!(output, "  requests (not granted): {}", effects.join(", "));
    }
    output
}

fn render_revocations(workspace_id: &str, revocations: &ExtensionRevocationsView) -> String {
    format!(
        "extension revocations {} sequence {} ({}) in {workspace_id}; issuer {}\n",
        revocations.list_id,
        revocations.sequence,
        short(&revocations.list_sha256),
        short(&revocations.issuer_key_sha256)
    )
}

fn render_receipt(kind: &str, workspace_id: &str, receipt: &ExtensionTransitionView) -> String {
    format!(
        "extension {kind} in {workspace_id}; catalog revision {}; decision {}\n",
        receipt.catalog_revision,
        short(&receipt.decision_sha256)
    )
}

/// Standard output for one answer, in text lines or JSON rows. A refusal is
/// written to standard error by the caller.
#[must_use]
pub fn render_extension_answer(answer: &ExtensionAnswer, json: bool) -> String {
    if json {
        return match answer {
            ExtensionAnswer::Listed {
                scopes,
                catalog_revision,
            } => {
                let mut output = String::new();
                for scope in scopes {
                    let _ = writeln!(
                        output,
                        "{}",
                        serde_json::json!({"type": "extension_scope", "scope": scope})
                    );
                }
                let _ = writeln!(
                    output,
                    "{}",
                    serde_json::json!({
                        "type": "extension_list",
                        "scopes": scopes.len(),
                        "catalog_revision": catalog_revision,
                    })
                );
                output
            }
            ExtensionAnswer::Refused { .. } => String::new(),
            _ => format!(
                "{}\n",
                serde_json::json!({"type": "extension_answer", "answer": answer})
            ),
        };
    }
    match answer {
        ExtensionAnswer::Trusted {
            workspace_id,
            key,
            receipt,
        } => render_key(workspace_id, key) + &render_receipt("trusted", workspace_id, receipt),
        ExtensionAnswer::Distrusted {
            workspace_id,
            key,
            receipt,
        } => render_key(workspace_id, key) + &render_receipt("distrusted", workspace_id, receipt),
        ExtensionAnswer::Installed {
            workspace_id,
            extension,
            receipt,
        } => {
            render_extension(workspace_id, extension, false)
                + &render_receipt("installed", workspace_id, receipt)
        }
        ExtensionAnswer::Uninstalled {
            workspace_id,
            package_id,
            receipt,
        } => render_receipt(&format!("{package_id} uninstalled"), workspace_id, receipt),
        ExtensionAnswer::RevocationsApplied {
            workspace_id,
            revocations,
            deactivated,
            receipt,
        } => {
            let mut output = render_revocations(workspace_id, revocations);
            let _ = writeln!(
                output,
                "extension revocations deactivated: {}",
                if deactivated.is_empty() {
                    "none".to_owned()
                } else {
                    deactivated.join(", ")
                }
            );
            output + &render_receipt("revocations applied", workspace_id, receipt)
        }
        ExtensionAnswer::RevocationsUnchanged {
            workspace_id,
            revocations,
        } => {
            render_revocations(workspace_id, revocations)
                + "extension revocations unchanged: the scope accepted this list already\n"
        }
        ExtensionAnswer::Listed {
            scopes,
            catalog_revision,
        } => {
            let mut output = String::new();
            for scope in scopes {
                for key in &scope.keys {
                    output.push_str(&render_key(&scope.workspace_id, key));
                }
                for extension in &scope.extensions {
                    output.push_str(&render_extension(&scope.workspace_id, extension, false));
                }
                match &scope.revocations {
                    Some(revocations) => {
                        output.push_str(&render_revocations(&scope.workspace_id, revocations));
                    }
                    None => {
                        let _ = writeln!(
                            output,
                            "extension revocations: none accepted in {}",
                            scope.workspace_id
                        );
                    }
                }
            }
            let _ = writeln!(
                output,
                "extensions: {} scopes; catalog revision {catalog_revision}",
                scopes.len()
            );
            output
        }
        ExtensionAnswer::Refused { .. } => String::new(),
    }
}

/// One refusal line for standard error.
#[must_use]
pub fn render_extension_refusal(refusal: ExtensionRefusal, json: bool) -> String {
    if json {
        format!(
            "{}\n",
            serde_json::json!({"type": "extension_refused", "code": refusal.code()})
        )
    } else {
        format!("extension refused: {}\n", refusal.code())
    }
}

/// The closed process exit class of a refusal.
#[must_use]
pub const fn extension_refusal_exit(refusal: ExtensionRefusal) -> crate::headless::ClientExitCode {
    use crate::headless::ClientExitCode;
    match refusal {
        ExtensionRefusal::InvalidInput | ExtensionRefusal::FileInvalid => {
            ClientExitCode::InvalidInput
        }
        ExtensionRefusal::UntrustedSigner
        | ExtensionRefusal::SignatureDenied
        | ExtensionRefusal::DigestMismatch
        | ExtensionRefusal::RevocationsUntrusted
        | ExtensionRefusal::RevocationsInvalid
        | ExtensionRefusal::RevocationsStale => ClientExitCode::AuthorityDenied,
        ExtensionRefusal::AlreadyTrusted
        | ExtensionRefusal::KeyConflict
        | ExtensionRefusal::KeyInUse
        | ExtensionRefusal::NotFound
        | ExtensionRefusal::AlreadyInstalled
        | ExtensionRefusal::LicenseDenied
        | ExtensionRefusal::DependencyDenied
        | ExtensionRefusal::DependencyInUse
        | ExtensionRefusal::CompatibilityDenied
        | ExtensionRefusal::Revoked
        | ExtensionRefusal::CapabilityConflict => ClientExitCode::PolicyDenied,
        ExtensionRefusal::ResourceLimit => ClientExitCode::ResourceBound,
        ExtensionRefusal::ClockUnavailable
        | ExtensionRefusal::StoreUnavailable
        | ExtensionRefusal::StoreConflict
        | ExtensionRefusal::StoreIntegrity => ClientExitCode::ServiceUnavailable,
    }
}

/// An extension operation as the command line names it, before any file is
/// read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtensionCommand {
    /// Trusts the key of a trust statement file.
    Trust {
        /// The scope.
        workspace_id: String,
        /// Absolute path of the trust statement.
        file: std::path::PathBuf,
    },
    /// Removes a trusted key.
    Distrust {
        /// The scope.
        workspace_id: String,
        /// SHA-256 of the key.
        key_sha256: String,
    },
    /// Installs the package of a directory.
    Install {
        /// The scope.
        workspace_id: String,
        /// Absolute path of the package directory.
        directory: std::path::PathBuf,
        /// The one license the person allows.
        allowed_license: String,
    },
    /// Removes an extension.
    Uninstall {
        /// The scope.
        workspace_id: String,
        /// The package identity.
        package_id: String,
    },
    /// Applies the list of a revocation list file.
    ApplyRevocations {
        /// The scope.
        workspace_id: String,
        /// Absolute path of the list file.
        file: std::path::PathBuf,
    },
    /// Lists every scope or one.
    List {
        /// One scope, or every scope.
        workspace_id: Option<String>,
    },
}

/// The request of a command, reading its files without following links.
#[cfg(target_os = "linux")]
pub fn extension_request(command: &ExtensionCommand) -> Result<ExtensionRequest, ExtensionRefusal> {
    use source::{read_file, read_package};
    Ok(match command {
        ExtensionCommand::Trust { workspace_id, file } => ExtensionRequest::Trust {
            workspace_id: workspace_id.clone(),
            statement: serde_json::from_slice(&read_file(file, MAX_EXTENSION_TRUST_BYTES)?)
                .map_err(|_| ExtensionRefusal::FileInvalid)?,
        },
        ExtensionCommand::Distrust {
            workspace_id,
            key_sha256,
        } => ExtensionRequest::Distrust {
            workspace_id: workspace_id.clone(),
            key_sha256: key_sha256.clone(),
        },
        ExtensionCommand::Install {
            workspace_id,
            directory,
            allowed_license,
        } => {
            let (package, observed_source_sha256) = read_package(directory)?;
            ExtensionRequest::Install {
                workspace_id: workspace_id.clone(),
                package: Box::new(package),
                observed_source_sha256,
                allowed_license: allowed_license.clone(),
            }
        }
        ExtensionCommand::Uninstall {
            workspace_id,
            package_id,
        } => ExtensionRequest::Uninstall {
            workspace_id: workspace_id.clone(),
            package_id: package_id.clone(),
        },
        ExtensionCommand::ApplyRevocations { workspace_id, file } => {
            ExtensionRequest::ApplyRevocations {
                workspace_id: workspace_id.clone(),
                list: Box::new(
                    serde_json::from_slice(&read_file(file, MAX_EXTENSION_FILE_BYTES)?)
                        .map_err(|_| ExtensionRefusal::FileInvalid)?,
                ),
            }
        }
        ExtensionCommand::List { workspace_id } => ExtensionRequest::List {
            workspace_id: workspace_id.clone(),
        },
    })
}

#[cfg(target_os = "linux")]
mod source {
    use std::io::Read as _;
    use std::os::fd::OwnedFd;
    use std::path::Path;

    use rustix::fs::{Mode, OFlags, openat};
    use sha2::{Digest, Sha256};

    use super::{
        EXTENSION_PACKAGE_FILE, EXTENSION_SOURCE_FILE, ExtensionRefusal, MAX_EXTENSION_FILE_BYTES,
        MAX_EXTENSION_SOURCE_BYTES, SignedExtensionPackage,
    };

    const INVALID: ExtensionRefusal = ExtensionRefusal::FileInvalid;

    /// The length of an open regular file within `maximum` bytes.
    fn regular_length(file: &OwnedFd, maximum: u64) -> Result<u64, ExtensionRefusal> {
        let metadata = rustix::fs::fstat(file).map_err(|_| INVALID)?;
        if rustix::fs::FileType::from_raw_mode(metadata.st_mode)
            != rustix::fs::FileType::RegularFile
        {
            return Err(INVALID);
        }
        u64::try_from(metadata.st_size)
            .ok()
            .filter(|length| *length > 0 && *length <= maximum)
            .ok_or(INVALID)
    }

    /// Reads exactly the file's length; a file that changed length is
    /// refused.
    fn read_open(file: OwnedFd, maximum: u64) -> Result<Vec<u8>, ExtensionRefusal> {
        let length = regular_length(&file, maximum)?;
        let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(0));
        std::fs::File::from(file)
            .take(length + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| INVALID)?;
        if bytes.len() as u64 != length {
            return Err(INVALID);
        }
        Ok(bytes)
    }

    fn open_at(directory: &OwnedFd, name: &str) -> Result<OwnedFd, ExtensionRefusal> {
        // A FIFO or device opens without blocking and is then refused as
        // not a regular file.
        openat(
            directory,
            name,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| INVALID)
    }

    /// Reads one absolute file without following a link at its last
    /// component.
    pub(super) fn read_file(path: &Path, maximum: u64) -> Result<Vec<u8>, ExtensionRefusal> {
        if !path.is_absolute() {
            return Err(INVALID);
        }
        let file = rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| INVALID)?;
        read_open(file, maximum)
    }

    /// Reads a package directory: its package file, and the SHA-256 of its
    /// source file, which is hashed in parts and never kept.
    pub(super) fn read_package(
        directory: &Path,
    ) -> Result<(SignedExtensionPackage, String), ExtensionRefusal> {
        if !directory.is_absolute() {
            return Err(INVALID);
        }
        let root = rustix::fs::open(
            directory,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| INVALID)?;
        let package = serde_json::from_slice::<SignedExtensionPackage>(&read_open(
            open_at(&root, EXTENSION_PACKAGE_FILE)?,
            MAX_EXTENSION_FILE_BYTES,
        )?)
        .map_err(|_| INVALID)?;
        let source = open_at(&root, EXTENSION_SOURCE_FILE)?;
        let length = regular_length(&source, MAX_EXTENSION_SOURCE_BYTES)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0_u8; 64 * 1024];
        let mut total = 0_u64;
        let mut file = std::fs::File::from(source).take(length + 1);
        loop {
            let read = file.read(&mut buffer).map_err(|_| INVALID)?;
            if read == 0 {
                break;
            }
            total += read as u64;
            hasher.update(&buffer[..read]);
        }
        if total != length {
            return Err(INVALID);
        }
        let digest =
            hasher
                .finalize()
                .iter()
                .fold(String::with_capacity(64), |mut output, byte| {
                    use std::fmt::Write as _;
                    let _ = write!(output, "{byte:02x}");
                    output
                });
        Ok((package, digest))
    }
}

#[cfg(all(
    test,
    target_os = "linux",
    feature = "source-artifacts",
    feature = "workflow-supervisor"
))]
#[path = "coding_extensions_tests.rs"]
mod tests;

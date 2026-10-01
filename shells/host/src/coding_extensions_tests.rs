// Real encrypted operational stores in private temporary directories and a
// synthetic sample signed with keys derived from fixed, published seeds; no
// process, transport, network or model.
use std::collections::BTreeMap;

use agentmage_kernel_engine::operational_store::DurableAuthorityRuntime;
use ed25519_dalek::{Signer, SigningKey};

use super::*;
use crate::capability_package::{
    CapabilityDependency, CapabilityEntryPoint, CapabilityPackageScope, CapabilityRevocationEntry,
    seal_capability_manifest, seal_capability_revocations,
};
use crate::runtime_start_tests::JobLedgerStore;

const SIGNATURE_DOMAIN: &[u8] = b"agentmage.capability-package.v1\0";
const REVOCATION_DOMAIN: &[u8] = b"agentmage.capability-revocation.v1\0";
const SIGNER_SEED: [u8; 32] = [0x51; 32];
const ISSUER_SEED: [u8; 32] = [0x1d; 32];
const FOREIGN_SEED: [u8; 32] = [0x7f; 32];
const SIGNER_ID: &str = "agentmage-sample-signer";
const ISSUER_ID: &str = "agentmage-sample-issuer";
const FOREIGN_ID: &str = "agentmage-sample-foreign-issuer";
const LICENSE: &str = "LicenseRef-agentmage-sample";
const LIST_ID: &str = "agentmage-sample-revocations";
const FORMATTER: &str = "sample-formatter";
const LINTER: &str = "sample-linter";
const REPORT: &str = "sample-report";
// 2026-10-01T00:00:00Z
const NOW: u64 = 1_790_812_800_000;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        let _ = write!(output, "{byte:02x}");
        output
    })
}

fn signing(seed: [u8; 32]) -> SigningKey {
    SigningKey::from_bytes(&seed)
}

fn statement(role: ExtensionKeyRole, key_id: &str, seed: [u8; 32]) -> ExtensionTrustStatement {
    ExtensionTrustStatement {
        role,
        key_id: key_id.to_owned(),
        public_key: hex(signing(seed).verifying_key().as_bytes()),
    }
}

fn signer() -> ExtensionTrustStatement {
    statement(ExtensionKeyRole::PackageSigner, SIGNER_ID, SIGNER_SEED)
}

fn issuer() -> ExtensionTrustStatement {
    statement(ExtensionKeyRole::RevocationIssuer, ISSUER_ID, ISSUER_SEED)
}

fn foreign_issuer() -> ExtensionTrustStatement {
    statement(ExtensionKeyRole::RevocationIssuer, FOREIGN_ID, FOREIGN_SEED)
}

fn key_digest(seed: [u8; 32]) -> String {
    sha256_hex(signing(seed).verifying_key().as_bytes())
}

fn source_text(package_id: &str) -> String {
    format!("Synthetic {package_id} source for the AgentMage extension sample; it is never run.\n")
}

/// The fields a sample package varies.
struct Shape {
    package_id: &'static str,
    package_name: &'static str,
    tools: &'static [&'static str],
    skills: &'static [&'static str],
    side_effects: &'static [CapabilitySideEffect],
    dependencies: Vec<CapabilityDependency>,
    compatibility: CapabilityCompatibility,
    signer_seed: [u8; 32],
}

impl Shape {
    fn new(package_id: &'static str, package_name: &'static str) -> Self {
        Self {
            package_id,
            package_name,
            tools: &[],
            skills: &[],
            side_effects: &[CapabilitySideEffect::WorkspaceRead],
            dependencies: Vec::new(),
            compatibility: extension_host_compatibility(),
            signer_seed: SIGNER_SEED,
        }
    }

    fn signed(&self) -> SignedExtensionPackage {
        let source = source_text(self.package_id);
        let strings = |values: &[&str]| values.iter().map(|&value| value.to_owned()).collect();
        let manifest = seal_capability_manifest(CapabilityPackageManifest {
            schema_version: 1,
            package_id: self.package_id.to_owned(),
            package_name: self.package_name.to_owned(),
            version: "1.0.0".to_owned(),
            source_id: format!("{}-source", self.package_id),
            source_sha256: sha256_hex(source.as_bytes()),
            signer_id: SIGNER_ID.to_owned(),
            signer_public_key_sha256: key_digest(self.signer_seed),
            license: LICENSE.to_owned(),
            compatibility: self.compatibility.clone(),
            tools: strings(self.tools),
            skills: strings(self.skills),
            hooks: Vec::new(),
            requested_scope: CapabilityPackageScope {
                filesystem_roots: Vec::new(),
                commands: Vec::new(),
                network_domains: Vec::new(),
                credential_ids: Vec::new(),
                connector_ids: Vec::new(),
                publication_targets: Vec::new(),
                max_memory_bytes: 64 * 1024 * 1024,
                max_cpu_milliseconds: 1_000,
                retention_seconds: 0,
            },
            dependencies: self.dependencies.clone(),
            entry_points: vec![CapabilityEntryPoint {
                entry_id: "main".to_owned(),
                relative_path: "bin/main".to_owned(),
                content_sha256: sha256_hex(source.as_bytes()),
            }],
            side_effects: self.side_effects.to_vec(),
            manifest_sha256: String::new(),
        })
        .unwrap();
        let mut unsealed = manifest.clone();
        unsealed.manifest_sha256.clear();
        let signed = [SIGNATURE_DOMAIN, &serde_json::to_vec(&unsealed).unwrap()].concat();
        SignedExtensionPackage {
            manifest,
            signature: hex(&signing(self.signer_seed).sign(&signed).to_bytes()),
        }
    }
}

fn formatter() -> SignedExtensionPackage {
    let mut shape = Shape::new(FORMATTER, "Sample formatter");
    shape.tools = &["sample-format-check"];
    shape.signed()
}

fn linter() -> SignedExtensionPackage {
    let mut shape = Shape::new(LINTER, "Sample linter");
    shape.tools = &["sample-lint-check"];
    shape.skills = &["sample-lint-guide"];
    shape.signed()
}

fn report() -> SignedExtensionPackage {
    let formatter = formatter();
    let mut shape = Shape::new(REPORT, "Sample report");
    shape.tools = &["sample-report"];
    shape.side_effects = &[
        CapabilitySideEffect::WorkspaceRead,
        CapabilitySideEffect::WorkspaceWrite,
    ];
    shape.dependencies = vec![CapabilityDependency {
        package_id: FORMATTER.to_owned(),
        exact_version: "1.0.0".to_owned(),
        manifest_sha256: formatter.manifest.manifest_sha256,
    }];
    shape.signed()
}

fn list(
    list_id: &str,
    sequence: u64,
    entries: Vec<CapabilityRevocationEntry>,
    issuer: (&str, [u8; 32]),
    signed_by: [u8; 32],
) -> SignedRevocationList {
    let list = seal_capability_revocations(CapabilityRevocationList {
        schema_version: 1,
        list_id: list_id.to_owned(),
        sequence,
        issuer_id: issuer.0.to_owned(),
        issuer_public_key_sha256: key_digest(issuer.1),
        issued_at_epoch_ms: NOW + sequence,
        entries,
        list_sha256: String::new(),
    })
    .unwrap();
    let mut unsealed = list.clone();
    unsealed.list_sha256.clear();
    let signed = [REVOCATION_DOMAIN, &serde_json::to_vec(&unsealed).unwrap()].concat();
    SignedRevocationList {
        list,
        signature: hex(&signing(signed_by).sign(&signed).to_bytes()),
    }
}

fn package_entry(package_id: &str) -> CapabilityRevocationEntry {
    CapabilityRevocationEntry::Package {
        package_id: package_id.to_owned(),
    }
}

/// Revokes the formatter's package.
fn first_list() -> SignedRevocationList {
    list(
        LIST_ID,
        1,
        vec![package_entry(FORMATTER)],
        (ISSUER_ID, ISSUER_SEED),
        ISSUER_SEED,
    )
}

/// Also revokes the linter's exact manifest.
fn second_list() -> SignedRevocationList {
    list(
        LIST_ID,
        2,
        vec![
            package_entry(FORMATTER),
            CapabilityRevocationEntry::Manifest {
                manifest_sha256: linter().manifest.manifest_sha256,
            },
        ],
        (ISSUER_ID, ISSUER_SEED),
        ISSUER_SEED,
    )
}

/// Another list with the second list's sequence.
fn forked_list() -> SignedRevocationList {
    list(
        LIST_ID,
        2,
        vec![package_entry(LINTER)],
        (ISSUER_ID, ISSUER_SEED),
        ISSUER_SEED,
    )
}

/// A list of a key the sample's scopes do not trust by default.
fn foreign_list() -> SignedRevocationList {
    list(
        "agentmage-sample-foreign-revocations",
        1,
        vec![package_entry(LINTER)],
        (FOREIGN_ID, FOREIGN_SEED),
        FOREIGN_SEED,
    )
}

/// Signed by the trusted issuer key under another identity than the one the
/// scope trusts it as (review F2 of `e197fe95`).
fn renamed_list() -> SignedRevocationList {
    list(
        LIST_ID,
        3,
        vec![package_entry(FORMATTER), package_entry(LINTER)],
        ("agentmage-sample-renamed-issuer", ISSUER_SEED),
        ISSUER_SEED,
    )
}

/// Names the trusted issuer but is signed by another key.
fn unsigned_list() -> SignedRevocationList {
    list(
        LIST_ID,
        3,
        vec![package_entry(FORMATTER), package_entry(REPORT)],
        (ISSUER_ID, ISSUER_SEED),
        FOREIGN_SEED,
    )
}

fn pretty<T: Serialize>(value: &T) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}

/// Every file of the committed sample, by path, regenerated from its seeds.
fn sample_files() -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    files.insert("signer-trust.json".to_owned(), pretty(&signer()));
    files.insert("issuer-trust.json".to_owned(), pretty(&issuer()));
    files.insert(
        "foreign-issuer-trust.json".to_owned(),
        pretty(&foreign_issuer()),
    );
    for (package_id, package) in [
        (FORMATTER, formatter()),
        (LINTER, linter()),
        (REPORT, report()),
    ] {
        files.insert(
            format!("packages/{package_id}/{EXTENSION_PACKAGE_FILE}"),
            pretty(&package),
        );
        files.insert(
            format!("packages/{package_id}/{EXTENSION_SOURCE_FILE}"),
            source_text(package_id).into_bytes(),
        );
    }
    for (name, list) in [
        ("1", first_list()),
        ("2", second_list()),
        ("2-fork", forked_list()),
        ("foreign", foreign_list()),
        ("unsigned", unsigned_list()),
        ("renamed", renamed_list()),
    ] {
        files.insert(format!("revocations/{name}.json"), pretty(&list));
    }
    files
}

fn sample_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/extension-sample")
}

fn committed_files(root: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            let kind = std::fs::symlink_metadata(&path).unwrap().file_type();
            if kind.is_dir() {
                pending.push(path);
            } else {
                assert!(kind.is_file(), "{path:?}");
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();
                files.insert(relative, std::fs::read(&path).unwrap());
            }
        }
    }
    files
}

/// The extension owner over one real store, as the catalog host opens it.
struct Harness {
    store: JobLedgerStore,
    runtime: Option<DurableAuthorityRuntime>,
    clock: Option<u64>,
}

impl Harness {
    fn new(tag: &str) -> Self {
        let store = JobLedgerStore::new(tag);
        let runtime = store.try_runtime();
        assert!(runtime.is_some());
        Self {
            store,
            runtime,
            clock: Some(NOW),
        }
    }

    fn answer(&mut self, request: &ExtensionRequest) -> ExtensionAnswer {
        let runtime = self.runtime.as_ref().unwrap();
        let answer = answer_extension(request, &mut || Ok(runtime.owner_states()), self.clock);
        assert!(
            extension_answer_acknowledges(request, &answer),
            "{request:?} {answer:?}"
        );
        answer
    }

    fn revision(&self) -> u64 {
        self.stored().revision
    }

    fn stored(&self) -> agentmage_kernel_engine::owner_state_store::StoredOwnerState {
        self.runtime
            .as_ref()
            .unwrap()
            .owner_states()
            .load(OwnerStateName::ExtensionCatalog)
            .unwrap()
    }

    fn commit_raw(&self, state: &[u8]) {
        let revision = self.revision();
        self.runtime
            .as_ref()
            .unwrap()
            .owner_states()
            .commit(OwnerStateName::ExtensionCatalog, revision, state)
            .unwrap();
    }

    fn reopen(&mut self) {
        self.runtime = None;
        self.runtime = self.store.try_runtime();
        assert!(self.runtime.is_some());
    }

    fn trust(&mut self, workspace: &str, statement: ExtensionTrustStatement) -> ExtensionAnswer {
        self.answer(&ExtensionRequest::Trust {
            workspace_id: workspace.to_owned(),
            statement,
        })
    }

    fn install(&mut self, workspace: &str, package: SignedExtensionPackage) -> ExtensionAnswer {
        let observed_source_sha256 = package.manifest.source_sha256.clone();
        self.answer(&install(workspace, package, &observed_source_sha256))
    }

    fn apply(&mut self, workspace: &str, list: SignedRevocationList) -> ExtensionAnswer {
        self.answer(&ExtensionRequest::ApplyRevocations {
            workspace_id: workspace.to_owned(),
            list: Box::new(list),
        })
    }

    fn scope(&mut self, workspace: &str) -> ExtensionScopeView {
        match self.answer(&ExtensionRequest::List {
            workspace_id: Some(workspace.to_owned()),
        }) {
            ExtensionAnswer::Listed { mut scopes, .. } => {
                assert_eq!(scopes.len(), 1);
                scopes.remove(0)
            }
            other => panic!("listed: {other:?}"),
        }
    }

    /// Each extension of a scope: whether it is active and why.
    fn states(&mut self, workspace: &str) -> Vec<(String, bool, ExtensionEntryReason)> {
        self.scope(workspace)
            .extensions
            .into_iter()
            .map(|extension| (extension.package_id, extension.active, extension.reason))
            .collect()
    }

    /// Trusts the sample's signer and issuer and installs its three packages.
    fn populate(&mut self, workspace: &str) {
        assert!(matches!(
            self.trust(workspace, signer()),
            ExtensionAnswer::Trusted { .. }
        ));
        assert!(matches!(
            self.trust(workspace, issuer()),
            ExtensionAnswer::Trusted { .. }
        ));
        for package in [formatter(), linter(), report()] {
            assert!(matches!(
                self.install(workspace, package),
                ExtensionAnswer::Installed { .. }
            ));
        }
    }
}

fn install(
    workspace: &str,
    package: SignedExtensionPackage,
    observed_source_sha256: &str,
) -> ExtensionRequest {
    ExtensionRequest::Install {
        workspace_id: workspace.to_owned(),
        package: Box::new(package),
        observed_source_sha256: observed_source_sha256.to_owned(),
        allowed_license: LICENSE.to_owned(),
    }
}

/// The first 32-byte encoding, by its leading byte, that names no curve
/// point.
fn not_a_point() -> [u8; 32] {
    (0_u8..=u8::MAX)
        .map(|leading| {
            let mut bytes = [0_u8; 32];
            bytes[0] = leading;
            bytes
        })
        .find(|bytes| VerifyingKey::from_bytes(bytes).is_err())
        .unwrap()
}

fn refused(answer: &ExtensionAnswer) -> Option<ExtensionRefusal> {
    match answer {
        ExtensionAnswer::Refused { refusal } => Some(*refusal),
        _ => None,
    }
}

fn active(package_id: &str) -> (String, bool, ExtensionEntryReason) {
    (package_id.to_owned(), true, ExtensionEntryReason::Installed)
}

fn inactive(
    package_id: &str,
    reason: ExtensionEntryReason,
) -> (String, bool, ExtensionEntryReason) {
    (package_id.to_owned(), false, reason)
}

#[test]
fn the_committed_sample_is_regenerated_from_its_published_seeds() {
    // Decision 0132: the sample's keys come from fixed, published seeds, so
    // every committed file is reproduced byte for byte; nothing else is in
    // the sample directory.
    assert_eq!(committed_files(&sample_root()), sample_files());
    // The seeds give three distinct keys, and the sample's lists and
    // packages verify only as the decision describes.
    let digests = [SIGNER_SEED, ISSUER_SEED, FOREIGN_SEED].map(key_digest);
    assert!(digests[0] != digests[1] && digests[1] != digests[2] && digests[0] != digests[2]);
    assert_eq!(
        extension_key_sha256(&signer()).as_deref(),
        Some(digests[0].as_str())
    );
}

#[test]
fn signed_extensions_install_under_a_trusted_signer_and_survive_reopening() {
    let mut harness = Harness::new("extension-install");
    // Nothing is kept before the first change.
    assert_eq!(
        harness.answer(&ExtensionRequest::List { workspace_id: None }),
        ExtensionAnswer::Listed {
            scopes: Vec::new(),
            catalog_revision: 0
        }
    );
    let ExtensionAnswer::Trusted { key, receipt, .. } = harness.trust("workspace-a", signer())
    else {
        panic!("trusted");
    };
    assert_eq!(
        key,
        ExtensionKeyView {
            role: ExtensionKeyRole::PackageSigner,
            key_id: SIGNER_ID.to_owned(),
            key_sha256: key_digest(SIGNER_SEED),
        }
    );
    assert_eq!(receipt.catalog_revision, 1);
    assert_eq!(receipt.state_sha256, sha256_hex(&harness.stored().state));
    let ExtensionAnswer::Installed {
        extension, receipt, ..
    } = harness.install("workspace-a", formatter())
    else {
        panic!("installed");
    };
    assert_eq!(receipt.catalog_revision, 2);
    assert_eq!(
        (
            extension.package_id.as_str(),
            extension.version.as_str(),
            extension.active,
            extension.installed_at.as_str()
        ),
        (FORMATTER, "1.0.0", true, "2026-10-01T00:00:00Z")
    );
    assert_eq!(
        extension.entries,
        [ExtensionEntryView {
            capability_id: "sample-format-check".to_owned(),
            kind: CatalogCapabilityKind::Tool,
            active: true,
            reason: ExtensionEntryReason::Installed,
        }]
    );
    assert_eq!(
        extension.side_effects,
        [CapabilitySideEffect::WorkspaceRead]
    );
    for package in [linter(), report()] {
        assert!(matches!(
            harness.install("workspace-a", package),
            ExtensionAnswer::Installed { .. }
        ));
    }
    harness.reopen();
    assert_eq!(harness.revision(), 4);
    let scope = harness.scope("workspace-a");
    assert_eq!(scope.keys.len(), 1);
    assert!(scope.revocations.is_none());
    assert_eq!(
        harness.states("workspace-a"),
        [active(FORMATTER), active(LINTER), active(REPORT)]
    );
    // Another scope holds nothing.
    assert_eq!(
        harness.answer(&ExtensionRequest::List {
            workspace_id: Some("workspace-b".to_owned())
        }),
        ExtensionAnswer::Listed {
            scopes: Vec::new(),
            catalog_revision: 4
        }
    );
    // Uninstalling the report, then the formatter it depended on; the scope
    // keeps the linter.
    assert_eq!(
        refused(&harness.answer(&ExtensionRequest::Uninstall {
            workspace_id: "workspace-a".to_owned(),
            package_id: FORMATTER.to_owned(),
        })),
        Some(ExtensionRefusal::DependencyInUse)
    );
    for package_id in [REPORT, FORMATTER] {
        assert!(matches!(
            harness.answer(&ExtensionRequest::Uninstall {
                workspace_id: "workspace-a".to_owned(),
                package_id: package_id.to_owned(),
            }),
            ExtensionAnswer::Uninstalled { .. }
        ));
    }
    assert_eq!(harness.states("workspace-a"), [active(LINTER)]);
    assert_eq!(harness.revision(), 6);
}

#[test]
fn a_revocation_list_deactivates_only_what_it_reaches_within_its_scope() {
    let mut harness = Harness::new("extension-revocations");
    harness.populate("workspace-a");
    harness.populate("workspace-b");
    let revision = harness.revision();
    // The first list revokes the formatter's package; the report depends on
    // it and goes inactive too. The other scope does not change.
    let ExtensionAnswer::RevocationsApplied {
        revocations,
        deactivated,
        receipt,
        ..
    } = harness.apply("workspace-a", first_list())
    else {
        panic!("applied");
    };
    assert_eq!(
        revocations,
        ExtensionRevocationsView {
            list_id: LIST_ID.to_owned(),
            sequence: 1,
            list_sha256: first_list().list.list_sha256,
            issuer_id: ISSUER_ID.to_owned(),
            issuer_key_sha256: key_digest(ISSUER_SEED),
        }
    );
    assert_eq!(deactivated, [FORMATTER, REPORT]);
    assert_eq!(receipt.catalog_revision, revision + 1);
    assert_eq!(
        harness.states("workspace-a"),
        [
            inactive(FORMATTER, ExtensionEntryReason::Revoked),
            active(LINTER),
            inactive(REPORT, ExtensionEntryReason::DependencyInactive),
        ]
    );
    let scope = harness.scope("workspace-a");
    let entries = scope
        .extensions
        .iter()
        .flat_map(|extension| extension.entries.iter())
        .map(|entry| (entry.capability_id.as_str(), entry.active, entry.reason))
        .collect::<Vec<_>>();
    assert_eq!(
        entries,
        [
            ("sample-format-check", false, ExtensionEntryReason::Revoked),
            ("sample-lint-check", true, ExtensionEntryReason::Installed),
            ("sample-lint-guide", true, ExtensionEntryReason::Installed),
            (
                "sample-report",
                false,
                ExtensionEntryReason::DependencyInactive
            ),
        ]
    );
    assert_eq!(
        harness.states("workspace-b"),
        [active(FORMATTER), active(LINTER), active(REPORT)]
    );
    // The same list again changes nothing.
    assert_eq!(
        harness.apply("workspace-a", first_list()),
        ExtensionAnswer::RevocationsUnchanged {
            workspace_id: "workspace-a".to_owned(),
            revocations,
        }
    );
    assert_eq!(harness.revision(), revision + 1);
    // The next list also revokes the linter's exact manifest.
    let ExtensionAnswer::RevocationsApplied { deactivated, .. } =
        harness.apply("workspace-a", second_list())
    else {
        panic!("applied");
    };
    assert_eq!(deactivated, [LINTER]);
    // Older, forked, foreign and badly signed lists are refused, as is a
    // newer list the trusted key signed under another identity than the one
    // the scope trusts it as; the scope keeps the second list.
    for (list, refusal) in [
        (first_list(), ExtensionRefusal::RevocationsStale),
        (forked_list(), ExtensionRefusal::RevocationsStale),
        (foreign_list(), ExtensionRefusal::RevocationsUntrusted),
        (unsigned_list(), ExtensionRefusal::RevocationsInvalid),
        (renamed_list(), ExtensionRefusal::RevocationsUntrusted),
    ] {
        assert_eq!(refused(&harness.apply("workspace-a", list)), Some(refusal));
    }
    assert_eq!(harness.revision(), revision + 2);
    assert_eq!(
        harness.scope("workspace-a").revocations.unwrap().sequence,
        2
    );
    // A revoked package cannot be installed again in the scope that revoked
    // it, but can be in a scope whose accepted list does not reach it.
    for package_id in [REPORT, FORMATTER] {
        harness.answer(&ExtensionRequest::Uninstall {
            workspace_id: "workspace-a".to_owned(),
            package_id: package_id.to_owned(),
        });
    }
    assert_eq!(
        refused(&harness.install("workspace-a", formatter())),
        Some(ExtensionRefusal::Revoked)
    );
    // A scope that trusts another issuer accepts that issuer's list, which
    // reaches only that scope.
    assert!(matches!(
        harness.trust("workspace-b", foreign_issuer()),
        ExtensionAnswer::Trusted { .. }
    ));
    let ExtensionAnswer::RevocationsApplied { deactivated, .. } =
        harness.apply("workspace-b", foreign_list())
    else {
        panic!("applied");
    };
    assert_eq!(deactivated, [LINTER]);
    assert_eq!(
        harness.states("workspace-b"),
        [
            active(FORMATTER),
            inactive(LINTER, ExtensionEntryReason::Revoked),
            active(REPORT),
        ]
    );
    // Once a scope accepted one issuer's list, another list identity is
    // stale there.
    assert_eq!(
        refused(&harness.apply("workspace-b", first_list())),
        Some(ExtensionRefusal::RevocationsStale)
    );
    harness.reopen();
    assert_eq!(
        harness.states("workspace-a"),
        [inactive(LINTER, ExtensionEntryReason::Revoked)]
    );
    assert_eq!(
        harness.scope("workspace-b").revocations.unwrap().list_id,
        "agentmage-sample-foreign-revocations"
    );
}

#[test]
fn every_refusal_changes_nothing() {
    let mut harness = Harness::new("extension-refusals");
    let workspace = "workspace-a";
    // Without a trusted signer, nothing installs.
    assert_eq!(
        refused(&harness.install(workspace, formatter())),
        Some(ExtensionRefusal::UntrustedSigner)
    );
    assert_eq!(harness.revision(), 0);
    harness.trust(workspace, signer());
    harness.trust(workspace, issuer());
    let revision = harness.revision();
    let mut forged = formatter();
    forged.signature = forged.signature.replacen(&forged.signature[..2], "00", 1);
    if forged.signature == formatter().signature {
        forged.signature = forged.signature.replacen("00", "11", 1);
    }
    let mut other_compatibility = Shape::new(FORMATTER, "Sample formatter");
    other_compatibility.compatibility.kernel_version = "2.0.0".to_owned();
    other_compatibility.tools = &["sample-format-check"];
    let mut foreign_signer = Shape::new(FORMATTER, "Sample formatter");
    foreign_signer.signer_seed = FOREIGN_SEED;
    let mut duplicate_tool = Shape::new("sample-duplicate", "Sample duplicate");
    duplicate_tool.tools = &["sample-format-check"];
    let mut self_dependent = Shape::new("sample-self", "Sample self");
    self_dependent.dependencies = vec![CapabilityDependency {
        package_id: "sample-self".to_owned(),
        exact_version: "1.0.0".to_owned(),
        manifest_sha256: "a".repeat(64),
    }];
    let observed = |package: &SignedExtensionPackage| package.manifest.source_sha256.clone();
    let cases: Vec<(&str, ExtensionRequest, ExtensionRefusal)> = vec![
        (
            "a malformed scope",
            ExtensionRequest::List {
                workspace_id: Some("Workspace A".to_owned()),
            },
            ExtensionRefusal::InvalidInput,
        ),
        (
            "a malformed key",
            ExtensionRequest::Trust {
                workspace_id: workspace.to_owned(),
                statement: ExtensionTrustStatement {
                    public_key: "zz".repeat(32),
                    ..signer()
                },
            },
            ExtensionRefusal::InvalidInput,
        ),
        (
            "a key that is not a curve point",
            ExtensionRequest::Trust {
                workspace_id: workspace.to_owned(),
                statement: ExtensionTrustStatement {
                    public_key: hex(&not_a_point()),
                    ..signer()
                },
            },
            ExtensionRefusal::InvalidInput,
        ),
        (
            "the same key again",
            ExtensionRequest::Trust {
                workspace_id: workspace.to_owned(),
                statement: ExtensionTrustStatement {
                    key_id: "another-name".to_owned(),
                    ..signer()
                },
            },
            ExtensionRefusal::AlreadyTrusted,
        ),
        (
            "an identity already used for the role",
            ExtensionRequest::Trust {
                workspace_id: workspace.to_owned(),
                statement: statement(ExtensionKeyRole::PackageSigner, SIGNER_ID, FOREIGN_SEED),
            },
            ExtensionRefusal::KeyConflict,
        ),
        (
            "an unknown key",
            ExtensionRequest::Distrust {
                workspace_id: workspace.to_owned(),
                key_sha256: key_digest(FOREIGN_SEED),
            },
            ExtensionRefusal::NotFound,
        ),
        (
            "a malformed key digest",
            ExtensionRequest::Distrust {
                workspace_id: workspace.to_owned(),
                key_sha256: "A".repeat(64),
            },
            ExtensionRefusal::InvalidInput,
        ),
        (
            "a forged signature",
            install(workspace, forged.clone(), &observed(&forged)),
            ExtensionRefusal::SignatureDenied,
        ),
        (
            "a malformed signature",
            install(
                workspace,
                SignedExtensionPackage {
                    signature: "ab".to_owned(),
                    ..formatter()
                },
                &observed(&formatter()),
            ),
            ExtensionRefusal::InvalidInput,
        ),
        (
            "another source",
            install(workspace, formatter(), &"b".repeat(64)),
            ExtensionRefusal::DigestMismatch,
        ),
        (
            "an unsealed manifest",
            {
                let mut changed = formatter();
                changed.manifest.package_name = "Changed".to_owned();
                install(workspace, changed, &observed(&formatter()))
            },
            ExtensionRefusal::DigestMismatch,
        ),
        (
            "another license",
            ExtensionRequest::Install {
                workspace_id: workspace.to_owned(),
                package: Box::new(formatter()),
                observed_source_sha256: observed(&formatter()),
                allowed_license: "MIT".to_owned(),
            },
            ExtensionRefusal::LicenseDenied,
        ),
        (
            "an untrusted signer key",
            install(
                workspace,
                foreign_signer.signed(),
                &observed(&foreign_signer.signed()),
            ),
            ExtensionRefusal::UntrustedSigner,
        ),
        (
            "another contract version",
            install(
                workspace,
                other_compatibility.signed(),
                &observed(&other_compatibility.signed()),
            ),
            ExtensionRefusal::CompatibilityDenied,
        ),
        (
            "a dependency not installed",
            install(workspace, report(), &observed(&report())),
            ExtensionRefusal::DependencyDenied,
        ),
        (
            "a dependency on itself",
            install(
                workspace,
                self_dependent.signed(),
                &observed(&self_dependent.signed()),
            ),
            ExtensionRefusal::DependencyDenied,
        ),
        (
            "an unknown extension",
            ExtensionRequest::Uninstall {
                workspace_id: workspace.to_owned(),
                package_id: FORMATTER.to_owned(),
            },
            ExtensionRefusal::NotFound,
        ),
        (
            "a malformed list signature",
            ExtensionRequest::ApplyRevocations {
                workspace_id: workspace.to_owned(),
                list: Box::new(SignedRevocationList {
                    signature: String::new(),
                    ..first_list()
                }),
            },
            ExtensionRefusal::InvalidInput,
        ),
        (
            "an unsealed list",
            ExtensionRequest::ApplyRevocations {
                workspace_id: workspace.to_owned(),
                list: Box::new({
                    let mut changed = first_list();
                    changed.list.sequence = 9;
                    changed
                }),
            },
            ExtensionRefusal::RevocationsInvalid,
        ),
    ];
    for (name, request, refusal) in cases {
        assert_eq!(refused(&harness.answer(&request)), Some(refusal), "{name}");
        assert_eq!(harness.revision(), revision, "{name}");
    }
    // An installation needs the host's clock.
    harness.clock = None;
    assert_eq!(
        refused(&harness.install(workspace, formatter())),
        Some(ExtensionRefusal::ClockUnavailable)
    );
    harness.clock = Some(NOW);
    assert_eq!(harness.revision(), revision);
    harness.install(workspace, formatter());
    let revision = harness.revision();
    for (name, request, refusal) in [
        (
            "the same package again",
            install(workspace, formatter(), &observed(&formatter())),
            ExtensionRefusal::AlreadyInstalled,
        ),
        (
            "a tool another extension offers",
            install(
                workspace,
                duplicate_tool.signed(),
                &observed(&duplicate_tool.signed()),
            ),
            ExtensionRefusal::CapabilityConflict,
        ),
        (
            "the signer of an installed extension",
            ExtensionRequest::Distrust {
                workspace_id: workspace.to_owned(),
                key_sha256: key_digest(SIGNER_SEED),
            },
            ExtensionRefusal::KeyInUse,
        ),
    ] {
        assert_eq!(refused(&harness.answer(&request)), Some(refusal), "{name}");
        assert_eq!(harness.revision(), revision, "{name}");
    }
    // The issuer of the accepted list stays trusted; a dependency that the
    // list made inactive refuses a new dependent.
    harness.apply(workspace, first_list());
    let revision = harness.revision();
    for (name, request, refusal) in [
        (
            "the issuer of the accepted list",
            ExtensionRequest::Distrust {
                workspace_id: workspace.to_owned(),
                key_sha256: key_digest(ISSUER_SEED),
            },
            ExtensionRefusal::KeyInUse,
        ),
        (
            "an inactive dependency",
            install(workspace, report(), &observed(&report())),
            ExtensionRefusal::DependencyDenied,
        ),
    ] {
        assert_eq!(refused(&harness.answer(&request)), Some(refusal), "{name}");
        assert_eq!(harness.revision(), revision, "{name}");
    }
    // A key no extension and no list uses can be removed; an emptied scope
    // is no longer listed.
    let mut fresh = Harness::new("extension-distrust");
    fresh.trust("workspace-c", issuer());
    let ExtensionAnswer::Distrusted { key, .. } = fresh.answer(&ExtensionRequest::Distrust {
        workspace_id: "workspace-c".to_owned(),
        key_sha256: key_digest(ISSUER_SEED),
    }) else {
        panic!("distrusted");
    };
    assert_eq!(key.role, ExtensionKeyRole::RevocationIssuer);
    assert_eq!(
        fresh.answer(&ExtensionRequest::List { workspace_id: None }),
        ExtensionAnswer::Listed {
            scopes: Vec::new(),
            catalog_revision: 2
        }
    );
}

#[test]
fn a_stored_catalog_is_verified_again_at_every_operation() {
    let mut harness = Harness::new("extension-integrity");
    harness.populate("workspace-a");
    harness.apply("workspace-a", first_list());
    let state = harness.stored().state;
    let mut decoded = serde_json::from_slice::<CatalogState>(&state).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), state);
    // Each change below is a state a key holder could write but no operation
    // produces; each is refused, and the next operation still refuses it.
    type StateChange = Box<dyn Fn(&mut CatalogState)>;
    let changes: Vec<(&str, StateChange)> = vec![
        ("another schema", Box::new(|state| state.schema_version = 2)),
        (
            "a forged package signature",
            Box::new(|state| {
                let signature = &mut state.scopes[0].extensions[0].package.signature;
                *signature = format!("00{}", &signature[2..]);
                if *signature == formatter().signature {
                    *signature = format!("11{}", &signature[2..]);
                }
            }),
        ),
        (
            "a forged list signature",
            Box::new(|state| {
                state.scopes[0].accepted.as_mut().unwrap().signature = unsigned_list().signature;
            }),
        ),
        (
            "an extension whose signer key is gone",
            Box::new(|state| {
                state.scopes[0]
                    .keys
                    .retain(|key| key.role != ExtensionKeyRole::PackageSigner);
            }),
        ),
        (
            "a list whose issuer the scope trusts under another identity",
            Box::new(|state| {
                for key in &mut state.scopes[0].keys {
                    if key.role == ExtensionKeyRole::RevocationIssuer {
                        key.key_id = "agentmage-sample-renamed-issuer".to_owned();
                    }
                }
            }),
        ),
        (
            "a list whose issuer key is gone",
            Box::new(|state| {
                state.scopes[0]
                    .keys
                    .retain(|key| key.role != ExtensionKeyRole::RevocationIssuer);
            }),
        ),
        (
            "an extension whose dependency is gone",
            Box::new(|state| {
                state.scopes[0]
                    .extensions
                    .retain(|installed| installed.package.manifest.package_id != FORMATTER);
            }),
        ),
        (
            "extensions out of order",
            Box::new(|state| state.scopes[0].extensions.swap(0, 1)),
        ),
        (
            "keys out of order",
            Box::new(|state| state.scopes[0].keys.swap(0, 1)),
        ),
        (
            "two keys with one identity for a role",
            Box::new(|state| {
                let keys = &mut state.scopes[0].keys;
                keys.push(statement(
                    ExtensionKeyRole::PackageSigner,
                    SIGNER_ID,
                    FOREIGN_SEED,
                ));
                keys.sort_by_key(|key| extension_key_sha256(key).unwrap());
            }),
        ),
        (
            "a malformed installation time",
            Box::new(|state| {
                state.scopes[0].extensions[0].installed_at = "2026-10-01T24:00:00Z".to_owned();
            }),
        ),
        (
            "a malformed decision",
            Box::new(|state| state.scopes[0].extensions[0].decision_sha256 = "x".to_owned()),
        ),
        (
            "an empty scope",
            Box::new(|state| {
                let mut empty = state.scopes[0].clone();
                empty.workspace_id = "workspace-z".to_owned();
                empty.keys.clear();
                empty.extensions.clear();
                empty.accepted = None;
                state.scopes.push(empty);
            }),
        ),
        (
            "a scope twice",
            Box::new(|state| {
                let copy = state.scopes[0].clone();
                state.scopes.push(copy);
            }),
        ),
        (
            "scopes out of order",
            Box::new(|state| {
                let mut other = state.scopes[0].clone();
                other.workspace_id = "workspace-0".to_owned();
                state.scopes.push(other);
            }),
        ),
        (
            "a scope that is not portable",
            Box::new(|state| state.scopes[0].workspace_id = "Workspace".to_owned()),
        ),
    ];
    let base = decoded.clone();
    for (name, change) in changes {
        decoded = base.clone();
        change(&mut decoded);
        assert_eq!(
            decode_catalog(&serde_json::to_vec(&decoded).unwrap()).err(),
            Some(ExtensionRefusal::StoreIntegrity),
            "{name}"
        );
    }
    // A state that is not this owner's exact encoding is refused, and the
    // operations refuse it without changing it.
    let pretty = serde_json::to_vec_pretty(&base).unwrap();
    assert_eq!(
        decode_catalog(&pretty).err(),
        Some(ExtensionRefusal::StoreIntegrity)
    );
    assert!(decode_catalog(&state).is_ok());
    harness.commit_raw(&pretty);
    let revision = harness.revision();
    for request in [
        ExtensionRequest::List { workspace_id: None },
        ExtensionRequest::Uninstall {
            workspace_id: "workspace-a".to_owned(),
            package_id: LINTER.to_owned(),
        },
    ] {
        assert_eq!(
            refused(&harness.answer(&request)),
            Some(ExtensionRefusal::StoreIntegrity)
        );
    }
    assert_eq!(harness.revision(), revision);
    // Store failures map to closed refusals.
    for (error, refusal) in [
        (OwnerStateStoreError::Stale, ExtensionRefusal::StoreConflict),
        (
            OwnerStateStoreError::Integrity,
            ExtensionRefusal::StoreIntegrity,
        ),
        (
            OwnerStateStoreError::ResourceLimit,
            ExtensionRefusal::ResourceLimit,
        ),
        (
            OwnerStateStoreError::Storage,
            ExtensionRefusal::StoreUnavailable,
        ),
        (
            OwnerStateStoreError::Unavailable,
            ExtensionRefusal::StoreUnavailable,
        ),
        (
            OwnerStateStoreError::InvalidInput,
            ExtensionRefusal::StoreUnavailable,
        ),
    ] {
        assert_eq!(ExtensionRefusal::of_store(error), refusal);
    }
}

#[test]
fn a_stored_dependency_must_lock_the_installed_manifest() {
    // An extension is stored only with the exact manifest of each dependency
    // installed beside it; a lock naming another digest is refused, as is a
    // pair that names each other.
    let mut first = Shape::new("sample-pair-a", "Sample pair a");
    first.dependencies = vec![CapabilityDependency {
        package_id: "sample-pair-b".to_owned(),
        exact_version: "1.0.0".to_owned(),
        manifest_sha256: "0".repeat(64),
    }];
    let mut second = Shape::new("sample-pair-b", "Sample pair b");
    second.dependencies = vec![CapabilityDependency {
        package_id: "sample-pair-a".to_owned(),
        exact_version: "1.0.0".to_owned(),
        manifest_sha256: first.signed().manifest.manifest_sha256,
    }];
    let stored = |packages: Vec<SignedExtensionPackage>| {
        serde_json::to_vec(&CatalogState {
            schema_version: STATE_SCHEMA_VERSION,
            scopes: vec![ScopeState {
                workspace_id: "workspace-a".to_owned(),
                keys: vec![signer()],
                extensions: packages
                    .into_iter()
                    .map(|package| InstalledState {
                        package,
                        installed_at: "2026-10-01T00:00:00Z".to_owned(),
                        decision_sha256: "a".repeat(64),
                    })
                    .collect(),
                accepted: None,
            }],
        })
        .unwrap()
    };
    assert!(decode_catalog(&stored(vec![formatter(), report()])).is_ok());
    let mut other_version = Shape::new(FORMATTER, "Sample formatter");
    other_version.tools = &["sample-format-check"];
    other_version.side_effects = &[];
    for packages in [
        vec![first.signed(), second.signed()],
        vec![report()],
        vec![other_version.signed(), report()],
    ] {
        assert_eq!(
            decode_catalog(&stored(packages)).err(),
            Some(ExtensionRefusal::StoreIntegrity)
        );
    }
}

#[test]
fn requests_and_answers_are_parsed_closed() {
    let requests = [
        ExtensionRequest::Trust {
            workspace_id: "workspace-a".to_owned(),
            statement: signer(),
        },
        ExtensionRequest::Distrust {
            workspace_id: "workspace-a".to_owned(),
            key_sha256: key_digest(SIGNER_SEED),
        },
        install("workspace-a", formatter(), &"a".repeat(64)),
        ExtensionRequest::Uninstall {
            workspace_id: "workspace-a".to_owned(),
            package_id: FORMATTER.to_owned(),
        },
        ExtensionRequest::ApplyRevocations {
            workspace_id: "workspace-a".to_owned(),
            list: Box::new(first_list()),
        },
        ExtensionRequest::List { workspace_id: None },
    ];
    for request in requests {
        let text = serde_json::to_string(&request).unwrap();
        assert_eq!(
            serde_json::from_str::<ExtensionRequest>(&text).unwrap(),
            request
        );
        let extra = text.replacen('{', "{\"extra\":1,", 1);
        assert!(serde_json::from_str::<ExtensionRequest>(&extra).is_err());
        let renamed = text.replacen("\"operation\":\"", "\"operation\":\"x-", 1);
        assert!(serde_json::from_str::<ExtensionRequest>(&renamed).is_err());
    }
    // Nested records are closed as well, and an absent optional member must
    // still be present.
    for (text, name) in [
        (
            serde_json::to_string(&ExtensionRequest::Trust {
                workspace_id: "workspace-a".to_owned(),
                statement: signer(),
            })
            .unwrap()
            .replacen("\"role\":", "\"extra\":1,\"role\":", 1),
            "statement",
        ),
        (
            serde_json::to_string(&install("workspace-a", formatter(), &"a".repeat(64)))
                .unwrap()
                .replacen("\"signature\":", "\"extra\":1,\"signature\":", 1),
            "package",
        ),
        (
            "{\"operation\":\"list\"}".to_owned(),
            "absent optional member",
        ),
    ] {
        assert!(
            serde_json::from_str::<ExtensionRequest>(&text).is_err(),
            "{name}"
        );
    }
    let answers = [
        ExtensionAnswer::Refused {
            refusal: ExtensionRefusal::RevocationsStale,
        },
        ExtensionAnswer::Listed {
            scopes: vec![ExtensionScopeView {
                workspace_id: "workspace-a".to_owned(),
                keys: Vec::new(),
                extensions: Vec::new(),
                revocations: None,
            }],
            catalog_revision: 3,
        },
        ExtensionAnswer::RevocationsUnchanged {
            workspace_id: "workspace-a".to_owned(),
            revocations: revocations_view(&first_list().list),
        },
    ];
    for answer in answers {
        let text = serde_json::to_string(&answer).unwrap();
        assert_eq!(
            serde_json::from_str::<ExtensionAnswer>(&text).unwrap(),
            answer
        );
        let extra = text.replacen('{', "{\"extra\":1,", 1);
        assert!(serde_json::from_str::<ExtensionAnswer>(&extra).is_err());
    }
    assert!(
        serde_json::from_str::<ExtensionAnswer>(
            "{\"answer\":\"refused\",\"refusal\":\"extension.revoked\"}"
        )
        .is_err()
    );
}

#[test]
fn only_an_answer_to_the_sent_request_is_acknowledged() {
    let mut harness = Harness::new("extension-acknowledge");
    harness.trust("workspace-a", signer());
    let request = install(
        "workspace-a",
        formatter(),
        &formatter().manifest.source_sha256,
    );
    let answer = harness.answer(&request);
    assert!(extension_answer_acknowledges(&request, &answer));
    let ExtensionAnswer::Installed {
        workspace_id,
        extension,
        receipt,
    } = answer
    else {
        panic!("installed");
    };
    let installed =
        |workspace_id: &str, extension: ExtensionView, receipt: ExtensionTransitionView| {
            ExtensionAnswer::Installed {
                workspace_id: workspace_id.to_owned(),
                extension,
                receipt,
            }
        };
    for (name, answer) in [
        (
            "another scope",
            installed("workspace-b", extension.clone(), receipt.clone()),
        ),
        (
            "another package",
            installed(
                &workspace_id,
                ExtensionView {
                    package_id: LINTER.to_owned(),
                    ..extension.clone()
                },
                receipt.clone(),
            ),
        ),
        (
            "another manifest",
            installed(
                &workspace_id,
                ExtensionView {
                    manifest_sha256: "c".repeat(64),
                    ..extension.clone()
                },
                receipt.clone(),
            ),
        ),
        (
            "an inactive extension",
            installed(
                &workspace_id,
                ExtensionView {
                    active: false,
                    ..extension.clone()
                },
                receipt.clone(),
            ),
        ),
        (
            "another decision",
            installed(
                &workspace_id,
                extension.clone(),
                ExtensionTransitionView {
                    decision_sha256: "d".repeat(64),
                    ..receipt.clone()
                },
            ),
        ),
        (
            "another kind",
            ExtensionAnswer::Uninstalled {
                workspace_id: workspace_id.clone(),
                package_id: FORMATTER.to_owned(),
                receipt: receipt.clone(),
            },
        ),
    ] {
        assert!(!extension_answer_acknowledges(&request, &answer), "{name}");
    }
    // A list answer names only the requested scope; a key answer names the
    // sent key.
    let list = ExtensionRequest::List {
        workspace_id: Some("workspace-a".to_owned()),
    };
    let scope = |workspace_id: &str| ExtensionScopeView {
        workspace_id: workspace_id.to_owned(),
        keys: Vec::new(),
        extensions: Vec::new(),
        revocations: None,
    };
    assert!(!extension_answer_acknowledges(
        &list,
        &ExtensionAnswer::Listed {
            scopes: vec![scope("workspace-b")],
            catalog_revision: 1
        }
    ));
    assert!(!extension_answer_acknowledges(
        &list,
        &ExtensionAnswer::Listed {
            scopes: vec![scope("workspace-a"), scope("workspace-a")],
            catalog_revision: 1
        }
    ));
    let trust = ExtensionRequest::Trust {
        workspace_id: "workspace-a".to_owned(),
        statement: issuer(),
    };
    let receipt = ExtensionTransitionView {
        decision_sha256: extension_decision_sha256(&trust).unwrap(),
        ..receipt
    };
    let trusted = |key: ExtensionKeyView| ExtensionAnswer::Trusted {
        workspace_id: "workspace-a".to_owned(),
        key,
        receipt: receipt.clone(),
    };
    let key = ExtensionKeyView {
        role: ExtensionKeyRole::RevocationIssuer,
        key_id: ISSUER_ID.to_owned(),
        key_sha256: key_digest(ISSUER_SEED),
    };
    assert!(extension_answer_acknowledges(&trust, &trusted(key.clone())));
    for other in [
        ExtensionKeyView {
            role: ExtensionKeyRole::PackageSigner,
            ..key.clone()
        },
        ExtensionKeyView {
            key_id: SIGNER_ID.to_owned(),
            ..key.clone()
        },
        ExtensionKeyView {
            key_sha256: key_digest(SIGNER_SEED),
            ..key
        },
    ] {
        assert!(!extension_answer_acknowledges(&trust, &trusted(other)));
    }
    // A list answer names the sent list.
    let apply = ExtensionRequest::ApplyRevocations {
        workspace_id: "workspace-a".to_owned(),
        list: Box::new(first_list()),
    };
    assert!(!extension_answer_acknowledges(
        &apply,
        &ExtensionAnswer::RevocationsUnchanged {
            workspace_id: "workspace-a".to_owned(),
            revocations: revocations_view(&second_list().list),
        }
    ));
    let applied = |workspace_id: &str, list: &SignedRevocationList, decision: Option<String>| {
        ExtensionAnswer::RevocationsApplied {
            workspace_id: workspace_id.to_owned(),
            revocations: revocations_view(&list.list),
            deactivated: Vec::new(),
            receipt: ExtensionTransitionView {
                decision_sha256: decision.unwrap_or_else(|| "e".repeat(64)),
                ..receipt.clone()
            },
        }
    };
    let decision = extension_decision_sha256(&apply);
    assert!(extension_answer_acknowledges(
        &apply,
        &applied("workspace-a", &first_list(), decision.clone())
    ));
    for answer in [
        applied("workspace-b", &first_list(), decision.clone()),
        applied("workspace-a", &second_list(), decision.clone()),
        applied("workspace-a", &first_list(), None),
    ] {
        assert!(!extension_answer_acknowledges(&apply, &answer));
    }
    // A key or extension answer names the sent key or package.
    let distrust = ExtensionRequest::Distrust {
        workspace_id: "workspace-a".to_owned(),
        key_sha256: key_digest(ISSUER_SEED),
    };
    let distrusted = |key_sha256: String| ExtensionAnswer::Distrusted {
        workspace_id: "workspace-a".to_owned(),
        key: ExtensionKeyView {
            role: ExtensionKeyRole::RevocationIssuer,
            key_id: ISSUER_ID.to_owned(),
            key_sha256,
        },
        receipt: ExtensionTransitionView {
            decision_sha256: extension_decision_sha256(&distrust).unwrap(),
            ..receipt.clone()
        },
    };
    assert!(extension_answer_acknowledges(
        &distrust,
        &distrusted(key_digest(ISSUER_SEED))
    ));
    assert!(!extension_answer_acknowledges(
        &distrust,
        &distrusted(key_digest(SIGNER_SEED))
    ));
    let uninstall = ExtensionRequest::Uninstall {
        workspace_id: "workspace-a".to_owned(),
        package_id: FORMATTER.to_owned(),
    };
    let uninstalled = |package_id: &str| ExtensionAnswer::Uninstalled {
        workspace_id: "workspace-a".to_owned(),
        package_id: package_id.to_owned(),
        receipt: ExtensionTransitionView {
            decision_sha256: extension_decision_sha256(&uninstall).unwrap(),
            ..receipt.clone()
        },
    };
    assert!(extension_answer_acknowledges(
        &uninstall,
        &uninstalled(FORMATTER)
    ));
    assert!(!extension_answer_acknowledges(
        &uninstall,
        &uninstalled(LINTER)
    ));
}

#[test]
fn a_scope_keeps_at_most_its_bound_of_keys() {
    let mut harness = Harness::new("extension-key-bound");
    for index in 0..MAX_KEYS_PER_SCOPE {
        let seed = [u8::try_from(index).unwrap() + 1; 32];
        assert!(
            matches!(
                harness.trust(
                    "workspace-a",
                    statement(
                        ExtensionKeyRole::RevocationIssuer,
                        &format!("issuer-{index}"),
                        seed
                    )
                ),
                ExtensionAnswer::Trusted { .. }
            ),
            "{index}"
        );
    }
    let revision = harness.revision();
    assert_eq!(
        refused(&harness.trust(
            "workspace-a",
            statement(
                ExtensionKeyRole::RevocationIssuer,
                "issuer-extra",
                [0xee; 32]
            )
        )),
        Some(ExtensionRefusal::ResourceLimit)
    );
    assert_eq!(harness.revision(), revision);
    // Another scope has its own bound.
    assert!(matches!(
        harness.trust("workspace-b", issuer()),
        ExtensionAnswer::Trusted { .. }
    ));
}

#[test]
fn answers_render_as_escaped_text_lines_or_json_rows() {
    let mut harness = Harness::new("extension-render");
    harness.trust("workspace-a", signer());
    let mut shape = Shape::new("sample-escaped", "Sample\u{1b}[2J\u{202e}name");
    shape.tools = &["sample-escaped-tool"];
    let ExtensionAnswer::Installed { .. } = harness.install("workspace-a", shape.signed()) else {
        panic!("installed");
    };
    let listed = harness.answer(&ExtensionRequest::List { workspace_id: None });
    let text = render_extension_answer(&listed, false);
    assert!(!text.contains('\u{1b}') && !text.contains('\u{202e}'));
    assert!(text.contains("Sample\\u{1b}[2J\\u{202e}name"));
    assert!(text.contains("tool sample-escaped-tool [active]"));
    assert!(text.contains("requests (not granted): workspace_read"));
    assert!(text.contains("extension revocations: none accepted in workspace-a"));
    let json = render_extension_answer(&listed, true);
    let rows = json.lines().collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    for row in rows {
        serde_json::from_str::<serde_json::Value>(row).unwrap();
    }
    assert_eq!(
        render_extension_refusal(ExtensionRefusal::RevocationsStale, false),
        "extension refused: extension.revocations-stale\n"
    );
    assert_eq!(
        render_extension_refusal(ExtensionRefusal::Revoked, true),
        "{\"code\":\"extension.revoked\",\"type\":\"extension_refused\"}\n"
    );
    assert_eq!(
        render_extension_answer(
            &ExtensionAnswer::Refused {
                refusal: ExtensionRefusal::Revoked
            },
            false
        ),
        ""
    );
    use crate::headless::ClientExitCode;
    for (refusal, exit) in [
        (ExtensionRefusal::FileInvalid, ClientExitCode::InvalidInput),
        (
            ExtensionRefusal::RevocationsStale,
            ClientExitCode::AuthorityDenied,
        ),
        (
            ExtensionRefusal::SignatureDenied,
            ClientExitCode::AuthorityDenied,
        ),
        (ExtensionRefusal::Revoked, ClientExitCode::PolicyDenied),
        (
            ExtensionRefusal::ResourceLimit,
            ClientExitCode::ResourceBound,
        ),
        (
            ExtensionRefusal::StoreIntegrity,
            ClientExitCode::ServiceUnavailable,
        ),
    ] {
        assert_eq!(extension_refusal_exit(refusal), exit);
    }
}

#[test]
fn timestamps_digests_and_identities_parse_exactly() {
    for value in [
        "2026-10-01T00:00:00Z",
        "0001-01-01T23:59:59Z",
        "2026-10-31T00:00:00Z",
        "2024-02-29T00:00:00Z",
        "2000-02-29T00:00:00Z",
    ] {
        assert!(timestamp_shape(value), "{value}");
    }
    for value in [
        "2026-10-01T00:00:00",
        "2026-13-01T00:00:00Z",
        "2026-10-00T00:00:00Z",
        "2026-10-01T24:00:00Z",
        "2026-10-01T00:60:00Z",
        "2026-10-01 00:00:00Z",
        "0000-10-01T00:00:00Z",
        "２026-10-01T00:00:00Z",
        // Review N2 of `e197fe95`: a day its month does not have.
        "2026-09-31T00:00:00Z",
        "2026-02-29T00:00:00Z",
        "1900-02-29T00:00:00Z",
        "2026-10-32T00:00:00Z",
    ] {
        assert!(!timestamp_shape(value), "{value}");
    }
    assert!(extension_sha256(&"a".repeat(64)));
    assert!(!extension_sha256(&"A".repeat(64)) && !extension_sha256(&"a".repeat(63)));
    assert!(extension_identifier("LicenseRef-agentmage-sample"));
    assert!(!extension_identifier("") && !extension_identifier("a b"));
    assert!(!extension_identifier(&"a".repeat(129)));
    assert_eq!(
        crate::coding_memory::memory_timestamp(NOW).as_deref(),
        Some("2026-10-01T00:00:00Z")
    );
}

#[test]
fn files_are_read_only_as_bounded_regular_files() {
    let directory = fixture::Directory::new("extension-files");
    let package = directory.package(&formatter(), FORMATTER);
    let (read, observed) = source::read_package(&package).unwrap();
    assert_eq!(read, formatter());
    assert_eq!(observed, formatter().manifest.source_sha256);
    let trust = directory.file("trust.json", &pretty(&signer()));
    assert_eq!(
        source::read_file(&trust, MAX_EXTENSION_TRUST_BYTES).unwrap(),
        pretty(&signer())
    );
    // A request built from a command reads the same files.
    assert_eq!(
        extension_request(&ExtensionCommand::Install {
            workspace_id: "workspace-a".to_owned(),
            directory: package.clone(),
            allowed_license: LICENSE.to_owned(),
        }),
        Ok(install(
            "workspace-a",
            formatter(),
            &formatter().manifest.source_sha256
        ))
    );
    for (name, path, maximum) in [
        (
            "relative",
            std::path::PathBuf::from("trust.json"),
            MAX_EXTENSION_TRUST_BYTES,
        ),
        (
            "missing",
            directory.path("missing.json"),
            MAX_EXTENSION_TRUST_BYTES,
        ),
        ("too long", trust.clone(), 8),
        (
            "linked",
            directory.link("linked.json", &trust),
            MAX_EXTENSION_TRUST_BYTES,
        ),
        (
            "a FIFO",
            directory.fifo("fifo.json"),
            MAX_EXTENSION_TRUST_BYTES,
        ),
        (
            "empty",
            directory.file("empty.json", b""),
            MAX_EXTENSION_TRUST_BYTES,
        ),
        ("a directory", package.clone(), MAX_EXTENSION_TRUST_BYTES),
    ] {
        assert_eq!(
            source::read_file(&path, maximum),
            Err(ExtensionRefusal::FileInvalid),
            "{name}"
        );
    }
    for (name, change) in fixture::package_changes() {
        let changed = directory.package(&formatter(), name);
        change(&changed);
        assert_eq!(
            source::read_package(&changed).map(|_| ()),
            Err(ExtensionRefusal::FileInvalid),
            "{name}"
        );
    }
    assert_eq!(
        source::read_package(&directory.link("linked-package", &package)).map(|_| ()),
        Err(ExtensionRefusal::FileInvalid)
    );
    // A malformed trust statement or list is refused before any host.
    let malformed = directory.file("malformed.json", b"{\"role\":\"package-signer\"}");
    for command in [
        ExtensionCommand::Trust {
            workspace_id: "workspace-a".to_owned(),
            file: malformed.clone(),
        },
        ExtensionCommand::ApplyRevocations {
            workspace_id: "workspace-a".to_owned(),
            file: malformed,
        },
    ] {
        assert_eq!(
            extension_request(&command),
            Err(ExtensionRefusal::FileInvalid)
        );
    }
}

// The file fixture creates, links and replaces files, so it is a test module
// of its own for the effect boundary scan.
#[cfg(test)]
mod fixture {
    use std::path::{Path, PathBuf};

    use super::{EXTENSION_PACKAGE_FILE, EXTENSION_SOURCE_FILE, SignedExtensionPackage, pretty};

    pub(super) struct Directory {
        root: PathBuf,
    }

    impl Directory {
        pub(super) fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "agentmage-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&root).unwrap();
            Self { root }
        }

        pub(super) fn path(&self, name: &str) -> PathBuf {
            self.root.join(name)
        }

        pub(super) fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.root.join(name);
            std::fs::write(&path, bytes).unwrap();
            path
        }

        pub(super) fn link(&self, name: &str, target: &Path) -> PathBuf {
            let path = self.root.join(name);
            std::os::unix::fs::symlink(target, &path).unwrap();
            path
        }

        pub(super) fn fifo(&self, name: &str) -> PathBuf {
            let path = self.root.join(name);
            rustix::fs::mknodat(
                rustix::fs::CWD,
                path.as_path(),
                rustix::fs::FileType::Fifo,
                rustix::fs::Mode::from_raw_mode(0o600),
                0,
            )
            .unwrap();
            path
        }

        pub(super) fn package(&self, package: &SignedExtensionPackage, name: &str) -> PathBuf {
            let directory = self.root.join(name);
            std::fs::create_dir(&directory).unwrap();
            std::fs::write(directory.join(EXTENSION_PACKAGE_FILE), pretty(package)).unwrap();
            std::fs::write(
                directory.join(EXTENSION_SOURCE_FILE),
                super::source_text(&package.manifest.package_id),
            )
            .unwrap();
            directory
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    type Change = fn(&Path);

    /// Each change of a written package directory; each is refused.
    pub(super) fn package_changes() -> Vec<(&'static str, Change)> {
        vec![
            ("extension-missing-source", |directory| {
                std::fs::remove_file(directory.join(EXTENSION_SOURCE_FILE)).unwrap();
            }),
            ("extension-empty-source", |directory| {
                std::fs::write(directory.join(EXTENSION_SOURCE_FILE), b"").unwrap();
            }),
            ("extension-linked-source", |directory| {
                let moved = directory.with_extension("moved-source");
                std::fs::rename(directory.join(EXTENSION_SOURCE_FILE), &moved).unwrap();
                std::os::unix::fs::symlink(&moved, directory.join(EXTENSION_SOURCE_FILE)).unwrap();
            }),
            ("extension-linked-package", |directory| {
                let moved = directory.with_extension("moved-package");
                std::fs::rename(directory.join(EXTENSION_PACKAGE_FILE), &moved).unwrap();
                std::os::unix::fs::symlink(&moved, directory.join(EXTENSION_PACKAGE_FILE)).unwrap();
            }),
            ("extension-malformed-package", |directory| {
                std::fs::write(directory.join(EXTENSION_PACKAGE_FILE), b"{}").unwrap();
            }),
            ("extension-open-package", |directory| {
                let path = directory.join(EXTENSION_PACKAGE_FILE);
                let text = std::fs::read_to_string(&path).unwrap().replacen(
                    "\"signature\"",
                    "\"extra\": 1,\n  \"signature\"",
                    1,
                );
                std::fs::write(path, text).unwrap();
            }),
        ]
    }
}

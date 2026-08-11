//! Canonical operation and authority-class taxonomy.

use serde::{Deserialize, Deserializer, Serialize};

/// Current version of the canonical operation-to-authority mapping.
pub const OPERATION_TAXONOMY_VERSION: u16 = 1;

/// Independent authority class required by one operation.
///
/// Classes do not inherit from one another. Holding authority for one class never
/// implies authority for any other class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorityClass {
    /// Observe state without changing it.
    Observe,
    /// Produce a non-authoritative draft for later review.
    Draft,
    /// Change state held on the local machine.
    LocalWrite,
    /// Change state owned by a remote service.
    RemoteWrite,
    /// Execute a bounded process or active operation.
    Execute,
    /// Change a deployment or runtime environment.
    Deploy,
    /// Access a secret through an approved broker.
    Secrets,
    /// Change administrative policy or authority.
    Admin,
}

impl AuthorityClass {
    /// Every authority class in stable taxonomy order.
    pub const ALL: [Self; 8] = [
        Self::Observe,
        Self::Draft,
        Self::LocalWrite,
        Self::RemoteWrite,
        Self::Execute,
        Self::Deploy,
        Self::Secrets,
        Self::Admin,
    ];
}

/// Closed canonical operation identity authorized by an exact grant.
///
/// There is deliberately no wildcard, custom, or approve-all variant. The
/// authority class for each operation is fixed by the versioned taxonomy below.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantOperation {
    /// Observe content beneath one approved workspace.
    WorkspaceRead,
    /// Change exact workspace content in a controlled-write release.
    WorkspaceWrite,
    /// Delete exact workspace content in a controlled-write release.
    WorkspaceDelete,
    /// Execute one exact command.
    CommandExecute,
    /// Contact one exact network destination.
    NetworkAccess,
    /// Clone one exact repository without checking out or executing repository content.
    GitClone,
    /// Fetch one exact remote ref into an AgentMage-owned local namespace.
    GitFetch,
    /// Create one exact AgentMage-owned isolated worktree.
    GitWorktreeCreate,
    /// Remove one exact clean AgentMage-owned isolated worktree.
    GitWorktreeRemove,
    /// Advance one exact local branch by compare-and-swap fast-forward only.
    GitBranchFastForward,
    /// Create one exact local source-control commit.
    GitCommit,
    /// Push one exact source-control state to a remote.
    GitPush,
    /// Publish one exact artifact to a remote.
    Publish,
    /// Send one exact message to a remote recipient.
    Send,
    /// Upload one exact artifact to a remote destination.
    Upload,
    /// Deploy one exact artifact or release.
    Deploy,
    /// Read one exact database scope.
    DatabaseRead,
    /// Change one exact remote database scope.
    DatabaseWrite,
    /// Access one exact credential through an approved broker.
    CredentialAccess,
    /// Invoke one exact local model profile without giving the model authority.
    ModelInference,
    /// Produce one exact non-authoritative draft.
    DraftCreate,
    /// Change one exact administrative setting or policy.
    Administration,
}

impl GrantOperation {
    /// Every canonical operation in stable taxonomy order.
    pub const ALL: [Self; 22] = [
        Self::WorkspaceRead,
        Self::WorkspaceWrite,
        Self::WorkspaceDelete,
        Self::CommandExecute,
        Self::NetworkAccess,
        Self::GitClone,
        Self::GitFetch,
        Self::GitWorktreeCreate,
        Self::GitWorktreeRemove,
        Self::GitBranchFastForward,
        Self::GitCommit,
        Self::GitPush,
        Self::Publish,
        Self::Send,
        Self::Upload,
        Self::Deploy,
        Self::DatabaseRead,
        Self::DatabaseWrite,
        Self::CredentialAccess,
        Self::ModelInference,
        Self::DraftCreate,
        Self::Administration,
    ];

    /// Returns the one non-inheriting authority class fixed for this operation.
    #[must_use]
    pub const fn authority_class(self) -> AuthorityClass {
        match self {
            Self::WorkspaceRead | Self::DatabaseRead => AuthorityClass::Observe,
            Self::DraftCreate => AuthorityClass::Draft,
            Self::WorkspaceWrite
            | Self::WorkspaceDelete
            | Self::GitClone
            | Self::GitFetch
            | Self::GitWorktreeCreate
            | Self::GitWorktreeRemove
            | Self::GitBranchFastForward
            | Self::GitCommit => AuthorityClass::LocalWrite,
            Self::GitPush | Self::Publish | Self::Send | Self::Upload | Self::DatabaseWrite => {
                AuthorityClass::RemoteWrite
            }
            Self::CommandExecute | Self::NetworkAccess | Self::ModelInference => {
                AuthorityClass::Execute
            }
            Self::Deploy => AuthorityClass::Deploy,
            Self::CredentialAccess => AuthorityClass::Secrets,
            Self::Administration => AuthorityClass::Admin,
        }
    }
}

/// Stable reason an operation binding cannot be admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationTaxonomyError {
    /// The record names a taxonomy version this build does not implement.
    UnsupportedVersion,
    /// The authority class does not equal the canonical mapping for the operation.
    AuthorityClassMismatch,
}

impl OperationTaxonomyError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "operation.taxonomy.version_unsupported",
            Self::AuthorityClassMismatch => "operation.taxonomy.authority_mismatch",
        }
    }
}

/// Versioned exact operation identity and its orthogonal authority class.
///
/// Construction and deserialization reject any caller-selected class that differs
/// from the canonical mapping. Legacy bare operation values are accepted only by
/// the explicit non-broadening version-zero migration function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationBinding {
    taxonomy_version: u16,
    operation: GrantOperation,
    authority_class: AuthorityClass,
}

impl OperationBinding {
    /// Creates the current canonical binding for one operation.
    #[must_use]
    pub const fn new(operation: GrantOperation) -> Self {
        Self {
            taxonomy_version: OPERATION_TAXONOMY_VERSION,
            operation,
            authority_class: operation.authority_class(),
        }
    }

    /// Explicitly migrates a legacy bare operation from the named taxonomy version.
    pub const fn migrate_legacy(
        taxonomy_version: u16,
        operation: GrantOperation,
    ) -> Result<Self, OperationTaxonomyError> {
        if taxonomy_version == 0 {
            Ok(Self::new(operation))
        } else {
            Err(OperationTaxonomyError::UnsupportedVersion)
        }
    }

    /// Validates and constructs one fully specified binding.
    pub fn from_parts(
        taxonomy_version: u16,
        operation: GrantOperation,
        authority_class: AuthorityClass,
    ) -> Result<Self, OperationTaxonomyError> {
        if taxonomy_version != OPERATION_TAXONOMY_VERSION {
            return Err(OperationTaxonomyError::UnsupportedVersion);
        }
        if authority_class != operation.authority_class() {
            return Err(OperationTaxonomyError::AuthorityClassMismatch);
        }
        Ok(Self {
            taxonomy_version,
            operation,
            authority_class,
        })
    }

    /// Returns the exact taxonomy version.
    #[must_use]
    pub const fn taxonomy_version(self) -> u16 {
        self.taxonomy_version
    }

    /// Returns the exact canonical operation.
    #[must_use]
    pub const fn operation(self) -> GrantOperation {
        self.operation
    }

    /// Returns the operation's non-inheriting authority class.
    #[must_use]
    pub const fn authority_class(self) -> AuthorityClass {
        self.authority_class
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentOperationBinding {
    taxonomy_version: u16,
    operation: GrantOperation,
    authority_class: AuthorityClass,
}

impl<'de> Deserialize<'de> for OperationBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let current = CurrentOperationBinding::deserialize(deserializer)?;
        Self::from_parts(
            current.taxonomy_version,
            current.operation,
            current.authority_class,
        )
        .map_err(|error| serde::de::Error::custom(error.code()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        AuthorityClass, GrantOperation, OPERATION_TAXONOMY_VERSION, OperationBinding,
        OperationTaxonomyError,
    };

    #[test]
    fn every_operation_has_one_exact_authority_class_and_all_classes_are_represented() {
        let classes = GrantOperation::ALL
            .into_iter()
            .map(GrantOperation::authority_class)
            .collect::<BTreeSet<_>>();
        assert_eq!(classes, BTreeSet::from(AuthorityClass::ALL));
        assert_eq!(GrantOperation::ALL.len(), 22);
    }

    #[test]
    fn current_bindings_round_trip_and_reject_version_or_class_drift() {
        for operation in GrantOperation::ALL {
            let binding = OperationBinding::new(operation);
            let bytes = serde_json::to_vec(&binding).expect("binding must encode");
            let decoded: OperationBinding =
                serde_json::from_slice(&bytes).expect("binding must decode");
            assert_eq!(decoded, binding);

            for authority_class in AuthorityClass::ALL {
                let candidate = OperationBinding::from_parts(
                    OPERATION_TAXONOMY_VERSION,
                    operation,
                    authority_class,
                );
                assert_eq!(
                    candidate.is_ok(),
                    authority_class == operation.authority_class()
                );
            }
        }

        let wrong_class = serde_json::json!({
            "taxonomy_version": OPERATION_TAXONOMY_VERSION,
            "operation": "workspace_read",
            "authority_class": "admin"
        });
        assert!(serde_json::from_value::<OperationBinding>(wrong_class).is_err());

        let wrong_version = serde_json::json!({
            "taxonomy_version": OPERATION_TAXONOMY_VERSION + 1,
            "operation": "workspace_read",
            "authority_class": "observe"
        });
        assert!(serde_json::from_value::<OperationBinding>(wrong_version).is_err());
        assert!(
            serde_json::from_str::<OperationBinding>(
                r#"{"taxonomy_version":1,"operation":"unknown","authority_class":"observe"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn legacy_migration_is_explicit_exact_and_non_broadening() {
        assert!(serde_json::from_str::<OperationBinding>("\"git_push\"").is_err());
        let migrated = OperationBinding::migrate_legacy(0, GrantOperation::GitPush)
            .expect("exact legacy operation must migrate explicitly");
        assert_eq!(migrated, OperationBinding::new(GrantOperation::GitPush));
        assert_eq!(migrated.authority_class(), AuthorityClass::RemoteWrite);
        assert_eq!(
            OperationBinding::migrate_legacy(1, GrantOperation::GitPush),
            Err(OperationTaxonomyError::UnsupportedVersion)
        );
        assert!(serde_json::from_str::<OperationBinding>("\"all\"").is_err());
    }

    #[test]
    fn destructive_or_implicit_git_operations_are_not_representable() {
        for operation in [
            "git_pull",
            "git_restore_discard",
            "git_reset",
            "git_clean",
            "git_force_push",
            "git_force_with_lease_push",
            "git_mirror_push",
            "git_checkout",
            "git_merge",
            "git_rebase",
            "git_stash",
            "git_notes_write",
            "git_tag_update",
            "git_branch_delete",
            "git_ref_update",
            "git_remote_configure",
            "git_hook_execute",
            "git_filter_execute",
        ] {
            let candidate = serde_json::json!({
                "taxonomy_version": OPERATION_TAXONOMY_VERSION,
                "operation": operation,
                "authority_class": "local-write"
            });
            assert!(serde_json::from_value::<OperationBinding>(candidate).is_err());
        }
    }
}

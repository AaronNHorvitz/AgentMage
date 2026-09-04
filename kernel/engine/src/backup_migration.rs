//! Complete backup-domain manifests and non-mutating migration admission.

use std::collections::BTreeSet;

/// Closed backup data class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BackupDomain {
    /// Versioned product configuration.
    Configuration,
    /// Separately keyed encrypted canonical operational store.
    EncryptedOperationalState,
    /// User-approved knowledge sources and derived identities.
    ApprovedKnowledge,
    /// Rebuildable indexes.
    Indexes,
    /// Exact model, package, and runtime manifests.
    Manifests,
    /// Admitted local packages.
    Packages,
    /// Retained conversations.
    Conversations,
    /// Audit-chain anchors without secret payloads.
    AuditAnchors,
}

/// One exact encrypted backup set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupManifest {
    /// Stable backup identity.
    pub backup_id: String,
    /// Exact canonical product identity.
    pub product_identity_sha256: String,
    /// Exact source schema identity.
    pub schema_sha256: String,
    /// Exact encryption-key identity, never the key.
    pub backup_key_id_sha256: String,
    /// Digest of the encrypted archive.
    pub encrypted_archive_sha256: String,
    /// Exact included closed domains.
    pub domains: BTreeSet<BackupDomain>,
    /// Every content object has an integrity digest.
    pub content_hashes_complete: bool,
    /// Backup was verified by restoring into a fresh candidate.
    pub fresh_restore_verified: bool,
    /// Any secret-store value was exported.
    pub secret_value_exported: bool,
    /// Any plaintext canonical content was written outside the encrypted archive.
    pub plaintext_content_exported: bool,
}

/// Exact cross-version/platform migration proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationPlan {
    /// Digest of the verified backup manifest.
    pub backup_manifest_sha256: String,
    /// Source machine identity.
    pub source_machine_sha256: String,
    /// Destination machine identity.
    pub destination_machine_sha256: String,
    /// Exact source platform adapter.
    pub source_platform_adapter_sha256: String,
    /// Exact destination platform adapter.
    pub destination_platform_adapter_sha256: String,
    /// Ordered schema migration digests.
    pub schema_migrations: Vec<String>,
    /// Exact model-manifest migration digests.
    pub model_manifests: Vec<String>,
    /// Exact package-version migration digests.
    pub package_versions: Vec<String>,
    /// Destination compatibility was verified.
    pub compatibility_verified: bool,
    /// User approved the exact migration preview.
    pub explicitly_approved: bool,
    /// Live destination replacement was requested by this plan.
    pub live_replacement_requested: bool,
}

/// Stable backup/migration refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackupMigrationError {
    /// Identity, digest, or domain closure is malformed.
    InvalidManifest,
    /// A secret or plaintext export was attempted.
    SecretExport,
    /// Restore verification or content closure is incomplete.
    UnverifiedBackup,
    /// Compatibility, explicit approval, or exact migration identities are absent.
    MigrationBlocked,
    /// This planning layer attempted to replace live authority.
    AuthorityViolation,
}

/// Return the exact complete backup domain set.
#[must_use]
pub fn complete_backup_domains() -> BTreeSet<BackupDomain> {
    [
        BackupDomain::Configuration,
        BackupDomain::EncryptedOperationalState,
        BackupDomain::ApprovedKnowledge,
        BackupDomain::Indexes,
        BackupDomain::Manifests,
        BackupDomain::Packages,
        BackupDomain::Conversations,
        BackupDomain::AuditAnchors,
    ]
    .into_iter()
    .collect()
}

/// Validate a complete encrypted backup without granting restore authority.
pub fn validate_backup(value: &BackupManifest) -> Result<(), BackupMigrationError> {
    if !valid_id(&value.backup_id)
        || ![
            &value.product_identity_sha256,
            &value.schema_sha256,
            &value.backup_key_id_sha256,
            &value.encrypted_archive_sha256,
        ]
        .into_iter()
        .all(|digest| valid_sha256(digest))
        || value.domains != complete_backup_domains()
    {
        return Err(BackupMigrationError::InvalidManifest);
    }
    if value.secret_value_exported || value.plaintext_content_exported {
        return Err(BackupMigrationError::SecretExport);
    }
    if !value.content_hashes_complete || !value.fresh_restore_verified {
        return Err(BackupMigrationError::UnverifiedBackup);
    }
    Ok(())
}

/// Validate an exact migration proposal without replacing live destination state.
pub fn validate_migration(
    backup: &BackupManifest,
    plan: &MigrationPlan,
) -> Result<(), BackupMigrationError> {
    validate_backup(backup)?;
    if plan.live_replacement_requested {
        return Err(BackupMigrationError::AuthorityViolation);
    }
    if ![
        &plan.backup_manifest_sha256,
        &plan.source_machine_sha256,
        &plan.destination_machine_sha256,
        &plan.source_platform_adapter_sha256,
        &plan.destination_platform_adapter_sha256,
    ]
    .into_iter()
    .all(|digest| valid_sha256(digest))
        || plan.source_machine_sha256 == plan.destination_machine_sha256
        || plan.schema_migrations.is_empty()
        || plan.model_manifests.is_empty()
        || plan.package_versions.is_empty()
        || plan
            .schema_migrations
            .iter()
            .chain(&plan.model_manifests)
            .chain(&plan.package_versions)
            .any(|digest| !valid_sha256(digest))
        || !plan.compatibility_verified
        || !plan.explicitly_approved
    {
        return Err(BackupMigrationError::MigrationBlocked);
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hash(seed: char) -> String {
        seed.to_string().repeat(64)
    }
    fn backup() -> BackupManifest {
        BackupManifest {
            backup_id: "backup-001".to_owned(),
            product_identity_sha256: hash('1'),
            schema_sha256: hash('2'),
            backup_key_id_sha256: hash('3'),
            encrypted_archive_sha256: hash('4'),
            domains: complete_backup_domains(),
            content_hashes_complete: true,
            fresh_restore_verified: true,
            secret_value_exported: false,
            plaintext_content_exported: false,
        }
    }
    fn migration() -> MigrationPlan {
        MigrationPlan {
            backup_manifest_sha256: hash('5'),
            source_machine_sha256: hash('6'),
            destination_machine_sha256: hash('7'),
            source_platform_adapter_sha256: hash('8'),
            destination_platform_adapter_sha256: hash('9'),
            schema_migrations: vec![hash('a')],
            model_manifests: vec![hash('b')],
            package_versions: vec![hash('c')],
            compatibility_verified: true,
            explicitly_approved: true,
            live_replacement_requested: false,
        }
    }
    #[test]
    fn complete_encrypted_backup_and_migration_are_admitted_without_activation() {
        let backup = backup();
        assert_eq!(validate_backup(&backup), Ok(()));
        assert_eq!(validate_migration(&backup, &migration()), Ok(()));
    }
    #[test]
    fn missing_domain_or_secret_export_fails_closed() {
        let mut value = backup();
        value.domains.remove(&BackupDomain::AuditAnchors);
        assert_eq!(
            validate_backup(&value),
            Err(BackupMigrationError::InvalidManifest)
        );
        value = backup();
        value.secret_value_exported = true;
        assert_eq!(
            validate_backup(&value),
            Err(BackupMigrationError::SecretExport)
        );
    }
    #[test]
    fn unverified_restore_and_live_migration_replacement_fail_closed() {
        let mut value = backup();
        value.fresh_restore_verified = false;
        assert_eq!(
            validate_backup(&value),
            Err(BackupMigrationError::UnverifiedBackup)
        );
        let backup = backup();
        let mut plan = migration();
        plan.live_replacement_requested = true;
        assert_eq!(
            validate_migration(&backup, &plan),
            Err(BackupMigrationError::AuthorityViolation)
        );
    }
}

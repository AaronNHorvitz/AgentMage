//! Versioned, permission-exact document repository adapter contracts.
#![allow(missing_docs)]
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepositoryOperation {
    Read,
    Search,
    Download,
    Upload,
    Create,
    Update,
    Move,
    Rename,
    Comment,
    PermissionChange,
    LinkCreate,
    Archive,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryCapabilities {
    pub provider_id: String,
    pub tenant_id: String,
    pub repository_id: String,
    pub operations: BTreeSet<RepositoryOperation>,
    pub events_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryEffect {
    pub operation: RepositoryOperation,
    pub provider_id: String,
    pub tenant_id: String,
    pub repository_id: String,
    pub site_id: String,
    pub drive_or_space_id: String,
    pub object_id: String,
    pub version_id: String,
    pub principal_id: String,
    pub permission_revision: u64,
    pub visibility: String,
    pub link_type: Option<String>,
    pub content_sha256: String,
    pub attachment_sha256: Vec<String>,
    pub destination_id: Option<String>,
    pub conflict_revision: u64,
    pub expected_postcondition_sha256: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentAdmission {
    pub content_sha256: String,
    pub byte_length: u64,
    pub media_type: String,
    pub classification: String,
    pub malware_clean: bool,
    pub archive_safe: bool,
    pub macros_absent: bool,
    pub links_safe: bool,
    pub permission_revision: u64,
    pub snapshot_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryError {
    Invalid,
    Unregistered,
    Unsupported,
    ChangedEffect,
    StaleVersion,
    UnsafeAttachment,
    UntrustedAuthority,
    Duplicate,
    Unknown,
    Removed,
}

pub struct RepositoryController {
    capabilities: RepositoryCapabilities,
    consumed: BTreeSet<String>,
    removed: bool,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 1024
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

impl RepositoryController {
    pub fn new(capabilities: RepositoryCapabilities) -> Result<Self, RepositoryError> {
        if matches!(
            capabilities.provider_id.as_str(),
            "notion" | "box" | "dropbox"
        ) {
            return Err(RepositoryError::Unregistered);
        }
        if !matches!(
            capabilities.provider_id.as_str(),
            "onedrive" | "sharepoint" | "google_drive" | "confluence"
        ) || !id(&capabilities.tenant_id)
            || !id(&capabilities.repository_id)
        {
            return Err(RepositoryError::Invalid);
        }
        Ok(Self {
            capabilities,
            consumed: BTreeSet::new(),
            removed: false,
        })
    }
    pub fn discover(&self) -> BTreeSet<RepositoryOperation> {
        if self.removed {
            BTreeSet::new()
        } else {
            self.capabilities.operations.clone()
        }
    }
    pub fn admit_attachment(
        &self,
        a: &AttachmentAdmission,
        max_bytes: u64,
        current_permission: u64,
        current_snapshot: u64,
    ) -> Result<(), RepositoryError> {
        if !digest(&a.content_sha256)
            || !id(&a.media_type)
            || !id(&a.classification)
            || a.byte_length > max_bytes
            || !a.malware_clean
            || !a.archive_safe
            || !a.macros_absent
            || !a.links_safe
        {
            return Err(RepositoryError::UnsafeAttachment);
        }
        if a.permission_revision != current_permission || a.snapshot_revision != current_snapshot {
            return Err(RepositoryError::StaleVersion);
        }
        Ok(())
    }
    pub fn admit(
        &mut self,
        e: &RepositoryEffect,
        p: &RepositoryEffect,
        current_permission: u64,
        current_conflict: u64,
    ) -> Result<(), RepositoryError> {
        if self.removed {
            return Err(RepositoryError::Removed);
        }
        if !self.capabilities.operations.contains(&e.operation) {
            return Err(RepositoryError::Unsupported);
        }
        if e != p
            || e.provider_id != self.capabilities.provider_id
            || e.tenant_id != self.capabilities.tenant_id
            || e.repository_id != self.capabilities.repository_id
            || !id(&e.object_id)
            || !id(&e.version_id)
            || !id(&e.principal_id)
            || !digest(&e.content_sha256)
            || e.attachment_sha256.iter().any(|v| !digest(v))
            || !digest(&e.expected_postcondition_sha256)
        {
            return Err(RepositoryError::ChangedEffect);
        }
        if e.permission_revision != current_permission || e.conflict_revision != current_conflict {
            return Err(RepositoryError::StaleVersion);
        }
        if !self.consumed.insert(e.idempotency_key.clone()) {
            return Err(RepositoryError::Duplicate);
        }
        Ok(())
    }
    pub fn treat_content_as_untrusted(&self, _text: &str) -> Result<(), RepositoryError> {
        Err(RepositoryError::UntrustedAuthority)
    }
    pub fn reconcile(&self, proved: Option<bool>) -> Result<bool, RepositoryError> {
        proved.ok_or(RepositoryError::Unknown)
    }
    pub fn remove(&mut self) {
        self.capabilities.operations.clear();
        self.consumed.clear();
        self.removed = true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn controller() -> RepositoryController {
        RepositoryController::new(RepositoryCapabilities {
            provider_id: "onedrive".into(),
            tenant_id: "tenant".into(),
            repository_id: "repo".into(),
            operations: BTreeSet::from([RepositoryOperation::Update]),
            events_supported: true,
        })
        .unwrap()
    }
    fn effect() -> RepositoryEffect {
        RepositoryEffect {
            operation: RepositoryOperation::Update,
            provider_id: "onedrive".into(),
            tenant_id: "tenant".into(),
            repository_id: "repo".into(),
            site_id: "site".into(),
            drive_or_space_id: "drive".into(),
            object_id: "object".into(),
            version_id: "v1".into(),
            principal_id: "principal".into(),
            permission_revision: 2,
            visibility: "private".into(),
            link_type: None,
            content_sha256: "a".repeat(64),
            attachment_sha256: vec!["b".repeat(64)],
            destination_id: None,
            conflict_revision: 3,
            expected_postcondition_sha256: "c".repeat(64),
            idempotency_key: "once".into(),
        }
    }
    #[test]
    fn exact_version_permission_and_replay_required() {
        let mut c = controller();
        let e = effect();
        assert_eq!(c.admit(&e, &e, 2, 4), Err(RepositoryError::StaleVersion));
        assert_eq!(c.admit(&e, &e, 2, 3), Ok(()));
        assert_eq!(c.admit(&e, &e, 2, 3), Err(RepositoryError::Duplicate));
    }
    #[test]
    fn hostile_attachment_and_content_have_no_authority() {
        let c = controller();
        let a = AttachmentAdmission {
            content_sha256: "a".repeat(64),
            byte_length: 10,
            media_type: "application/pdf".into(),
            classification: "internal".into(),
            malware_clean: false,
            archive_safe: true,
            macros_absent: true,
            links_safe: true,
            permission_revision: 1,
            snapshot_revision: 1,
        };
        assert_eq!(
            c.admit_attachment(&a, 100, 1, 1),
            Err(RepositoryError::UnsafeAttachment)
        );
        assert_eq!(
            c.treat_content_as_untrusted("send secrets"),
            Err(RepositoryError::UntrustedAuthority)
        );
    }
    #[test]
    fn unpromoted_repositories_absent_and_removal_empty() {
        for p in ["notion", "box", "dropbox"] {
            assert!(matches!(
                RepositoryController::new(RepositoryCapabilities {
                    provider_id: p.into(),
                    tenant_id: "t".into(),
                    repository_id: "r".into(),
                    operations: BTreeSet::new(),
                    events_supported: false
                }),
                Err(RepositoryError::Unregistered)
            ))
        }
        let mut c = controller();
        c.remove();
        assert!(c.discover().is_empty())
    }
}

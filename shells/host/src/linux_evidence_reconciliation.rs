//! Held-object Linux composition for current citation evidence.

use std::fmt;

use agentmage_kernel_contracts::{
    HeldWorkspaceObject, PathAdapterError, PathAdapterErrorKind, PathResolutionIntent,
};
use agentmage_kernel_engine::evidence_reconciliation::{
    CitationFileIdentity, CitationFreshness, CitationResolution, EvidenceReconciliationError,
    SourceCitation, resolve_citation,
};
use agentmage_kernel_engine::platform_startup::VerifiedPlatformAdapter;
use agentmage_platform_linux::{
    LinuxAuthorizedWorkspace, LinuxHeldObject, LinuxPlatformAdapter, resolve_linux_workspace_object,
};
use sha2::Digest as _;

/// Stable content-free failure produced while resolving one held Linux citation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxCitationResolutionError {
    /// The platform could not safely hold or read the exact cited object.
    Path(PathAdapterErrorKind),
    /// The citation or derived current identity failed kernel validation.
    Reconciliation(EvidenceReconciliationError),
}

impl fmt::Display for LinuxCitationResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(kind) => formatter.write_str(kind.code()),
            Self::Reconciliation(_) => formatter.write_str("citation.reconciliation.rejected"),
        }
    }
}

impl std::error::Error for LinuxCitationResolutionError {}

/// One currentness result accompanied by exact bytes from the same held object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldLinuxCitationResolution {
    /// Content-free currentness decision produced by the kernel.
    pub resolution: CitationResolution,
    /// Exact current bytes, absent only when the exact cited path is missing.
    pub current_bytes: Option<Vec<u8>>,
}

/// Recomputes the complete current identity from held bytes before evidence reuse.
#[must_use]
pub fn verify_held_citation_bytes(value: &HeldLinuxCitationResolution) -> bool {
    let (Some(bytes), Some(current)) = (&value.current_bytes, &value.resolution.current_file)
    else {
        return false;
    };
    value.resolution.freshness == CitationFreshness::Current
        && u64::try_from(bytes.len()).ok() == Some(current.byte_len)
        && hex(&sha2::Sha256::digest(bytes)) == current.content_sha256
        && current == &value.resolution.citation.observed_file
}

/// Resolves one citation through the verified production Linux path adapter.
pub fn resolve_linux_citation(
    platform: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,
    workspace: &LinuxAuthorizedWorkspace,
    citation: SourceCitation,
    current_revision: &str,
) -> Result<HeldLinuxCitationResolution, LinuxCitationResolutionError> {
    resolve_with(citation, current_revision, |path| {
        resolve_linux_workspace_object(platform, workspace, path, PathResolutionIntent::ContentHash)
    })
}

fn resolve_with(
    citation: SourceCitation,
    current_revision: &str,
    mut hold: impl FnMut(
        &agentmage_kernel_contracts::WorkspacePath,
    ) -> Result<LinuxHeldObject, PathAdapterError>,
) -> Result<HeldLinuxCitationResolution, LinuxCitationResolutionError> {
    let held = match hold(&citation.observed_file.path) {
        Ok(held) => held,
        Err(error) if error.kind() == PathAdapterErrorKind::NotFound => {
            let resolution = resolve_citation(citation, None)
                .map_err(LinuxCitationResolutionError::Reconciliation)?;
            return Ok(HeldLinuxCitationResolution {
                resolution,
                current_bytes: None,
            });
        }
        Err(error) => return Err(LinuxCitationResolutionError::Path(error.kind())),
    };
    let preimage = held.preimage().ok_or(LinuxCitationResolutionError::Path(
        PathAdapterErrorKind::IdentityChanged,
    ))?;
    let current = CitationFileIdentity {
        path: held.workspace_path().clone(),
        byte_len: preimage.byte_len(),
        content_sha256: hex(preimage.content_sha256()),
        object_identity_sha256: hex(held.object_identity().object_identity_sha256()),
        revision: current_revision.to_owned(),
    };
    let bytes = held
        .read_exact_bytes()
        .map_err(|error| LinuxCitationResolutionError::Path(error.kind()))?;
    let resolution = resolve_citation(citation, Some(current))
        .map_err(LinuxCitationResolutionError::Reconciliation)?;
    Ok(HeldLinuxCitationResolution {
        resolution,
        current_bytes: Some(bytes),
    })
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
fn resolve_test_linux_citation(
    adapter_instance_id: agentmage_kernel_contracts::AdapterInstanceId,
    workspace: &LinuxAuthorizedWorkspace,
    citation: SourceCitation,
    current_revision: &str,
) -> Result<HeldLinuxCitationResolution, LinuxCitationResolutionError> {
    resolve_with(citation, current_revision, |path| {
        agentmage_platform_linux::resolve_test_linux_workspace_object(
            workspace,
            adapter_instance_id.clone(),
            path,
            PathResolutionIntent::ContentHash,
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        AdapterInstanceId, CONTRACT_SCHEMA_VERSION, EvidenceId, EvidenceKind, EvidenceReference,
        WorkspaceAuthorizationId, WorkspaceId, WorkspacePath,
    };
    use agentmage_kernel_engine::evidence_reconciliation::{
        CitationFileIdentity, CitationFreshness, CitationSelector, SourceCitation,
    };
    use agentmage_platform_linux::{
        resolve_test_linux_workspace_object, select_test_linux_workspace,
    };
    use sha2::{Digest, Sha256};

    use super::{
        LinuxCitationResolutionError, hex, resolve_test_linux_citation, verify_held_citation_bytes,
    };

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
        adapter_id: AdapterInstanceId,
        workspace: agentmage_platform_linux::LinuxAuthorizedWorkspace,
        workspace_id: WorkspaceId,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "agentmage-citation-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("notes")).expect("fixture creates");
            fs::write(root.join("notes/δelta.md"), "exact cited bytes\n").expect("fixture writes");
            let adapter_id = AdapterInstanceId::from_raw("adapter-citation-0001");
            let workspace_id = WorkspaceId::from_raw("workspace-citation-0001");
            let workspace = select_test_linux_workspace(
                &root,
                workspace_id.clone(),
                WorkspaceAuthorizationId::from_raw("authorization-citation-0001"),
                adapter_id.clone(),
            )
            .expect("workspace selects");
            Self {
                root,
                adapter_id,
                workspace,
                workspace_id,
            }
        }

        fn path(&self) -> WorkspacePath {
            WorkspacePath::new(self.workspace_id.clone(), ["notes", "δelta.md"])
                .expect("canonical path")
        }

        fn citation(&self, revision: &str) -> SourceCitation {
            let held = resolve_test_linux_workspace_object(
                &self.workspace,
                self.adapter_id.clone(),
                &self.path(),
                agentmage_kernel_contracts::PathResolutionIntent::ContentHash,
            )
            .expect("source holds");
            let preimage =
                agentmage_kernel_contracts::HeldWorkspaceObject::preimage(&held).expect("preimage");
            let content_sha256 = hex(preimage.content_sha256());
            SourceCitation {
                citation_id: "citation.fixture.1".to_owned(),
                evidence: EvidenceReference {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    evidence_id: EvidenceId::from_raw("evidence.fixture.1"),
                    kind: EvidenceKind::Observation,
                    source_id: "fixture.notes".to_owned(),
                    object_id: "delta".to_owned(),
                    fragment: Some("bytes:0-5".to_owned()),
                    content_sha256: content_sha256.clone(),
                    observed_revision: Some(revision.to_owned()),
                },
                selector: CitationSelector::ByteRange { start: 0, end: 5 },
                observed_at_epoch_ms: 1,
                observed_file: CitationFileIdentity {
                    path: self.path(),
                    byte_len: preimage.byte_len(),
                    content_sha256,
                    object_identity_sha256: hex(
                        agentmage_kernel_contracts::HeldWorkspaceObject::object_identity(&held)
                            .object_identity_sha256(),
                    ),
                    revision: revision.to_owned(),
                },
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn held_resolution_distinguishes_current_drift_and_missing() {
        let fixture = Fixture::new();
        let citation = fixture.citation("revision-a");
        let current = resolve_test_linux_citation(
            fixture.adapter_id.clone(),
            &fixture.workspace,
            citation.clone(),
            "revision-a",
        )
        .expect("current resolves");
        assert_eq!(current.resolution.freshness, CitationFreshness::Current);
        assert_eq!(
            current.current_bytes.as_deref(),
            Some(&b"exact cited bytes\n"[..])
        );
        assert!(verify_held_citation_bytes(&current));

        let revision_drift = resolve_test_linux_citation(
            fixture.adapter_id.clone(),
            &fixture.workspace,
            citation.clone(),
            "revision-b",
        )
        .expect("revision drift resolves");
        assert_eq!(
            revision_drift.resolution.freshness,
            CitationFreshness::Stale
        );
        assert!(!verify_held_citation_bytes(&revision_drift));

        fs::write(fixture.root.join("notes/δelta.md"), "changed preimage\n")
            .expect("source changes");
        let content_drift = resolve_test_linux_citation(
            fixture.adapter_id.clone(),
            &fixture.workspace,
            citation.clone(),
            "revision-a",
        )
        .expect("changed source resolves");
        assert_eq!(content_drift.resolution.freshness, CitationFreshness::Stale);
        assert_eq!(
            content_drift
                .resolution
                .current_file
                .as_ref()
                .map(|identity| identity.content_sha256.as_str()),
            Some(hex(&Sha256::digest(b"changed preimage\n")).as_str())
        );
        assert!(!verify_held_citation_bytes(&content_drift));

        fs::rename(
            fixture.root.join("notes/δelta.md"),
            fixture.root.join("notes/renamed.md"),
        )
        .expect("source renames");
        let missing = resolve_test_linux_citation(
            fixture.adapter_id.clone(),
            &fixture.workspace,
            citation,
            "revision-a",
        )
        .expect("missing resolves");
        assert_eq!(missing.resolution.freshness, CitationFreshness::Missing);
        assert!(missing.current_bytes.is_none());
        assert!(!verify_held_citation_bytes(&missing));
    }

    #[test]
    fn unsafe_substitution_is_not_downgraded_to_missing() {
        let fixture = Fixture::new();
        let citation = fixture.citation("revision-a");
        fs::remove_file(fixture.root.join("notes/δelta.md")).expect("source removes");
        symlink("renamed.md", fixture.root.join("notes/δelta.md")).expect("link creates");
        let error = resolve_test_linux_citation(
            fixture.adapter_id.clone(),
            &fixture.workspace,
            citation,
            "revision-a",
        )
        .expect_err("symbolic link rejects");
        assert_eq!(
            error,
            LinuxCitationResolutionError::Path(
                agentmage_kernel_contracts::PathAdapterErrorKind::SymbolicLink
            )
        );
    }
}

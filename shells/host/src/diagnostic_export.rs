//! One-use reviewed local diagnostic export workflow.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write as IoWrite;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use agentmage_kernel_contracts::DoctorReport;
use rustix::fs::{AtFlags, Mode, OFlags, linkat, openat};
use rustix::process::getuid;
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_PENDING_EXPORTS: usize = 4;
const EXPORT_LIFETIME_MS: u64 = 60_000;
const MAX_DESTINATION_BYTES: usize = 4_096;
const MAX_REPORT_BYTES: usize = 256 * 1024;

/// Stable failure from a diagnostic export preview or publication attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticExportError {
    /// Destination is relative, noncanonical, linked, synchronized, remote, or unsafe.
    DestinationDenied,
    /// Report is malformed or exceeds the fixed export bound.
    ReportDenied,
    /// Too many unconsumed previews exist.
    PreviewLimit,
    /// Preview is absent, expired, cancelled, consumed, or mismatched.
    ApprovalDenied,
    /// Atomic local publication failed with no authorized destination replacement.
    WriteFailed,
}

impl DiagnosticExportError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DestinationDenied => "diagnostic.export.destination-denied",
            Self::ReportDenied => "diagnostic.export.report-denied",
            Self::PreviewLimit => "diagnostic.export.preview-limit",
            Self::ApprovalDenied => "diagnostic.export.approval-denied",
            Self::WriteFailed => "diagnostic.export.write-failed",
        }
    }
}

/// Redacted preview returned before a one-use export grant may be consumed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticExportPreview {
    /// Opaque memory-only preview identity.
    pub preview_id: String,
    /// Digest of the canonical destination rather than its raw path.
    pub destination_sha256: String,
    /// Digest of the exact bytes to be published.
    pub payload_sha256: String,
    /// Exact byte count to be published.
    pub payload_bytes: u64,
    /// Stable included field families.
    pub included_fields: Vec<String>,
    /// Stable prohibited-data redaction families.
    pub redactions: Vec<String>,
    /// Stable sensitivity label.
    pub sensitivity: String,
    /// Stable retention instruction.
    pub retention: String,
    /// Preview expiry.
    pub expires_at_epoch_ms: u64,
    /// Digest of the complete approval display contract.
    pub confirmation_sha256: String,
}

/// Terminal result of one authorized local diagnostic export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticExportReceipt {
    /// Digest of the exact published bytes.
    pub payload_sha256: String,
    /// Digest of the canonical destination.
    pub destination_sha256: String,
    /// Exact published byte count.
    pub payload_bytes: u64,
    /// Stable terminal outcome.
    pub outcome: String,
}

struct PendingExport {
    destination: PathBuf,
    parent_identity: ParentIdentity,
    payload: Vec<u8>,
    preview: DiagnosticExportPreview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParentIdentity {
    device: u64,
    inode: u64,
    owner: u32,
    mode: u32,
}

/// Memory-only owner of reviewed diagnostic export previews.
#[derive(Default)]
pub struct DiagnosticExportWorkflow {
    pending: BTreeMap<String, PendingExport>,
}

impl DiagnosticExportWorkflow {
    /// Creates an empty workflow with no ambient destination or report access.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
        }
    }

    /// Creates a memory-only preview for one exact report and local destination.
    pub fn preview(
        &mut self,
        preview_id: String,
        report: &DoctorReport,
        destination: &Path,
        now_epoch_ms: u64,
    ) -> Result<DiagnosticExportPreview, DiagnosticExportError> {
        self.expire(now_epoch_ms);
        if self.pending.len() >= MAX_PENDING_EXPORTS || !valid_identifier(&preview_id) {
            return Err(DiagnosticExportError::PreviewLimit);
        }
        if self.pending.contains_key(&preview_id) {
            return Err(DiagnosticExportError::ApprovalDenied);
        }
        let (destination, parent_identity) = inspect_destination(destination)?;
        let mut payload =
            serde_json::to_vec_pretty(report).map_err(|_| DiagnosticExportError::ReportDenied)?;
        payload.push(b'\n');
        if payload.is_empty()
            || payload.len() > MAX_REPORT_BYTES
            || !valid_sha256(&report.report_sha256)
        {
            return Err(DiagnosticExportError::ReportDenied);
        }
        let destination_sha256 = sha256_hex(destination.as_os_str().as_encoded_bytes());
        let payload_sha256 = sha256_hex(&payload);
        let expires_at_epoch_ms = now_epoch_ms
            .checked_add(EXPORT_LIFETIME_MS)
            .ok_or(DiagnosticExportError::ApprovalDenied)?;
        let included_fields = vec![
            "component".to_owned(),
            "state".to_owned(),
            "reason_code".to_owned(),
            "remediation_code".to_owned(),
            "identity_sha256".to_owned(),
            "report_sha256".to_owned(),
        ];
        let redactions = vec![
            "credentials".to_owned(),
            "environment".to_owned(),
            "file_content".to_owned(),
            "host_identity".to_owned(),
            "paths".to_owned(),
            "prompts".to_owned(),
        ];
        let sensitivity = "content-free-local-diagnostic".to_owned();
        let retention = "user-managed-local-file".to_owned();
        let confirmation_sha256 = sha256_serialized(&ApprovalMaterial {
            preview_id: &preview_id,
            destination_sha256: &destination_sha256,
            payload_sha256: &payload_sha256,
            payload_bytes: payload.len() as u64,
            included_fields: &included_fields,
            redactions: &redactions,
            sensitivity: &sensitivity,
            retention: &retention,
            expires_at_epoch_ms,
        })?;
        let preview = DiagnosticExportPreview {
            preview_id: preview_id.clone(),
            destination_sha256,
            payload_sha256,
            payload_bytes: payload.len() as u64,
            included_fields,
            redactions,
            sensitivity,
            retention,
            expires_at_epoch_ms,
            confirmation_sha256,
        };
        self.pending.insert(
            preview_id,
            PendingExport {
                destination,
                parent_identity,
                payload,
                preview: preview.clone(),
            },
        );
        Ok(preview)
    }

    /// Consumes one exact preview and atomically publishes its complete payload once.
    pub fn approve(
        &mut self,
        preview_id: &str,
        confirmation_sha256: &str,
        now_epoch_ms: u64,
    ) -> Result<DiagnosticExportReceipt, DiagnosticExportError> {
        self.expire(now_epoch_ms);
        let pending = self
            .pending
            .remove(preview_id)
            .ok_or(DiagnosticExportError::ApprovalDenied)?;
        if pending.preview.confirmation_sha256 != confirmation_sha256
            || pending.preview.expires_at_epoch_ms <= now_epoch_ms
            || inspect_parent(
                pending
                    .destination
                    .parent()
                    .ok_or(DiagnosticExportError::DestinationDenied)?,
            )? != pending.parent_identity
            || pending.destination.exists()
        {
            return Err(DiagnosticExportError::ApprovalDenied);
        }
        publish_create_new(&pending.destination, &pending.payload)?;
        Ok(DiagnosticExportReceipt {
            payload_sha256: pending.preview.payload_sha256,
            destination_sha256: pending.preview.destination_sha256,
            payload_bytes: pending.preview.payload_bytes,
            outcome: "succeeded".to_owned(),
        })
    }

    /// Cancels one pending preview without writing a file.
    pub fn cancel(&mut self, preview_id: &str) -> bool {
        self.pending.remove(preview_id).is_some()
    }

    /// Returns the number of memory-only unconsumed previews.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    fn expire(&mut self, now_epoch_ms: u64) {
        self.pending
            .retain(|_, pending| pending.preview.expires_at_epoch_ms > now_epoch_ms);
    }
}

#[derive(Serialize)]
struct ApprovalMaterial<'a> {
    preview_id: &'a str,
    destination_sha256: &'a str,
    payload_sha256: &'a str,
    payload_bytes: u64,
    included_fields: &'a [String],
    redactions: &'a [String],
    sensitivity: &'a str,
    retention: &'a str,
    expires_at_epoch_ms: u64,
}

fn inspect_destination(
    destination: &Path,
) -> Result<(PathBuf, ParentIdentity), DiagnosticExportError> {
    if destination.as_os_str().is_empty()
        || destination.as_os_str().as_encoded_bytes().len() > MAX_DESTINATION_BYTES
        || !destination.is_absolute()
        || destination.extension().and_then(|value| value.to_str()) != Some("json")
        || destination
            .components()
            .any(|component| !matches!(component, Component::RootDir | Component::Normal(_)))
        || destination.exists()
    {
        return Err(DiagnosticExportError::DestinationDenied);
    }
    let parent = destination
        .parent()
        .ok_or(DiagnosticExportError::DestinationDenied)?;
    let canonical_parent =
        fs::canonicalize(parent).map_err(|_| DiagnosticExportError::DestinationDenied)?;
    if canonical_parent != parent || synchronized_path(&canonical_parent) {
        return Err(DiagnosticExportError::DestinationDenied);
    }
    let name = destination
        .file_name()
        .ok_or(DiagnosticExportError::DestinationDenied)?;
    Ok((
        canonical_parent.join(name),
        inspect_parent(&canonical_parent)?,
    ))
}

fn inspect_parent(parent: &Path) -> Result<ParentIdentity, DiagnosticExportError> {
    let metadata =
        fs::symlink_metadata(parent).map_err(|_| DiagnosticExportError::DestinationDenied)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != getuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(DiagnosticExportError::DestinationDenied);
    }
    Ok(ParentIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
        mode: metadata.permissions().mode(),
    })
}

fn synchronized_path(path: &Path) -> bool {
    path.components().any(|component| {
        let Component::Normal(name) = component else {
            return false;
        };
        let normalized = name
            .to_string_lossy()
            .to_ascii_lowercase()
            .replace([' ', '-', '_'], "");
        [
            "dropbox",
            "googledrive",
            "icloud",
            "nextcloud",
            "onedrive",
            "owncloud",
            "protondrive",
            "syncthing",
        ]
        .iter()
        .any(|marker| normalized.contains(marker))
    })
}

fn publish_create_new(destination: &Path, payload: &[u8]) -> Result<(), DiagnosticExportError> {
    let parent = destination
        .parent()
        .ok_or(DiagnosticExportError::DestinationDenied)?;
    let name = destination
        .file_name()
        .ok_or(DiagnosticExportError::DestinationDenied)?;
    let directory = File::open(parent).map_err(|_| DiagnosticExportError::WriteFailed)?;
    let mut anonymous = create_unnamed_file(&directory)?;
    anonymous
        .write_all(payload)
        .and_then(|_| anonymous.sync_all())
        .map_err(|_| DiagnosticExportError::WriteFailed)?;
    linkat(&anonymous, "", &directory, name, AtFlags::EMPTY_PATH)
        .map_err(|_| DiagnosticExportError::WriteFailed)?;
    directory
        .sync_all()
        .map_err(|_| DiagnosticExportError::WriteFailed)
}

fn create_unnamed_file(directory: &File) -> Result<File, DiagnosticExportError> {
    openat(
        directory,
        ".",
        OFlags::WRONLY | OFlags::CLOEXEC | OFlags::TMPFILE,
        Mode::from(0o600),
    )
    .map(File::from)
    .map_err(|_| DiagnosticExportError::WriteFailed)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_serialized(value: &impl Serialize) -> Result<String, DiagnosticExportError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| DiagnosticExportError::ReportDenied)
}

fn sha256_hex(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        DiagnosticComponent, DiagnosticObservation, DiagnosticState, DoctorReport,
    };
    use agentmage_kernel_engine::diagnostics::build_doctor_report;

    use super::{DiagnosticExportError, DiagnosticExportWorkflow, create_unnamed_file, sha256_hex};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "agentmage-diagnostic-export-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&path).expect("directory");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("private directory");
        path
    }

    fn report() -> DoctorReport {
        build_doctor_report(vec![DiagnosticObservation {
            component: DiagnosticComponent::Package,
            state: DiagnosticState::Healthy,
            reason_code: "diagnostic.package.verified".to_owned(),
            identity_sha256: None,
            stale: false,
        }])
        .expect("report")
    }

    #[test]
    fn preview_approval_publishes_exactly_once_with_private_mode() {
        let root = directory("success");
        let destination = root.join("doctor.json");
        let mut workflow = DiagnosticExportWorkflow::new();
        let preview = workflow
            .preview("export-0001".to_owned(), &report(), &destination, 100)
            .expect("preview");
        assert!(!destination.exists());
        let receipt = workflow
            .approve("export-0001", &preview.confirmation_sha256, 101)
            .expect("approval");
        assert_eq!(receipt.outcome, "succeeded");
        assert_eq!(
            sha256_hex(&fs::read(&destination).expect("payload")),
            receipt.payload_sha256
        );
        assert_eq!(
            fs::metadata(&destination)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            workflow.approve("export-0001", &preview.confirmation_sha256, 102),
            Err(DiagnosticExportError::ApprovalDenied)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn crash_before_publication_leaves_no_artifact() {
        let root = directory("crash-parent");
        let status = Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "diagnostic_export::tests::crash_export_child",
                "--nocapture",
            ])
            .env("AGENTMAGE_DIAGNOSTIC_CRASH_DIRECTORY", &root)
            .status()
            .expect("crash child");
        assert!(!status.success());
        assert_eq!(fs::read_dir(&root).expect("read directory").count(), 0);
        fs::remove_dir(root).expect("cleanup");
    }

    #[test]
    fn crash_export_child() {
        let Ok(root) = std::env::var("AGENTMAGE_DIAGNOSTIC_CRASH_DIRECTORY") else {
            return;
        };
        let directory = fs::File::open(root).expect("crash directory");
        let mut anonymous = create_unnamed_file(&directory).expect("anonymous export");
        anonymous
            .write_all(b"partial export")
            .expect("partial write");
        anonymous.sync_all().expect("partial sync");
        std::process::abort();
    }

    #[test]
    fn cancellation_mismatch_expiry_and_existing_destination_write_nothing() {
        for case in ["cancel", "mismatch", "expiry", "existing"] {
            let root = directory(case);
            let destination = root.join("doctor.json");
            let mut workflow = DiagnosticExportWorkflow::new();
            let preview = workflow
                .preview(format!("export-{case}"), &report(), &destination, 100)
                .expect("preview");
            let result = match case {
                "cancel" => {
                    assert!(workflow.cancel(&preview.preview_id));
                    workflow.approve(&preview.preview_id, &preview.confirmation_sha256, 101)
                }
                "mismatch" => workflow.approve(&preview.preview_id, &"f".repeat(64), 101),
                "expiry" => workflow.approve(
                    &preview.preview_id,
                    &preview.confirmation_sha256,
                    preview.expires_at_epoch_ms,
                ),
                "existing" => {
                    fs::write(&destination, b"existing").expect("competing destination");
                    workflow.approve(&preview.preview_id, &preview.confirmation_sha256, 101)
                }
                _ => unreachable!(),
            };
            assert_eq!(result, Err(DiagnosticExportError::ApprovalDenied));
            if case != "existing" {
                assert!(!destination.exists());
            } else {
                assert_eq!(fs::read(&destination).expect("existing"), b"existing");
            }
            assert_eq!(workflow.pending_count(), 0);
            fs::remove_dir_all(root).expect("cleanup");
        }
    }

    #[test]
    fn unsafe_nonprivate_and_synchronized_destinations_are_denied() {
        let private = directory("private");
        for destination in [
            PathBuf::from("relative.json"),
            private.join("wrong.txt"),
            private.join("OneDrive").join("doctor.json"),
        ] {
            let mut workflow = DiagnosticExportWorkflow::new();
            assert_eq!(
                workflow.preview("export-unsafe".to_owned(), &report(), &destination, 100),
                Err(DiagnosticExportError::DestinationDenied)
            );
        }
        let public = directory("public");
        fs::set_permissions(&public, fs::Permissions::from_mode(0o755)).expect("public mode");
        let mut workflow = DiagnosticExportWorkflow::new();
        assert_eq!(
            workflow.preview(
                "export-public".to_owned(),
                &report(),
                &public.join("doctor.json"),
                100
            ),
            Err(DiagnosticExportError::DestinationDenied)
        );
        fs::remove_dir_all(private).expect("cleanup");
        fs::remove_dir_all(public).expect("cleanup");
    }

    #[test]
    fn prohibited_source_canaries_never_reach_preview_receipt_or_export() {
        let canaries = [
            "AM-S15-CANARY-00-prompt",
            "AM-S15-CANARY-01-file-content",
            "AM-S15-CANARY-02-credential",
            "AM-S15-CANARY-03-private-key",
            "AM-S15-CANARY-04-environment-value",
            "AM-S15-CANARY-05-absolute-path",
            "AM-S15-CANARY-06-hostname",
            "AM-S15-CANARY-07-username",
            "AM-S15-CANARY-08-device-identifier",
        ];
        let safe_report = report();
        let expected_semantics = serde_json::to_vec(&safe_report).expect("safe report");
        let root = directory("canary-matrix");
        let destination = root.join("doctor.json");
        let mut workflow = DiagnosticExportWorkflow::new();
        let preview = workflow
            .preview(
                "export-canary-matrix".to_owned(),
                &safe_report,
                &destination,
                100,
            )
            .expect("preview");
        let preview_surface = format!("{preview:?}");
        let receipt = workflow
            .approve(&preview.preview_id, &preview.confirmation_sha256, 101)
            .expect("approval");
        let receipt_surface = format!("{receipt:?}");
        let exported = fs::read(&destination).expect("exported report");
        let exported_report: DoctorReport =
            serde_json::from_slice(&exported).expect("closed exported report");

        assert_eq!(exported_report, safe_report);
        assert_eq!(
            serde_json::to_vec(&exported_report).expect("report"),
            expected_semantics
        );
        for canary in canaries {
            assert!(
                !expected_semantics
                    .windows(canary.len())
                    .any(|part| part == canary.as_bytes())
            );
            assert!(!preview_surface.contains(canary));
            assert!(!receipt_surface.contains(canary));
            assert!(
                !exported
                    .windows(canary.len())
                    .any(|part| part == canary.as_bytes())
            );
        }
        fs::remove_dir_all(root).expect("cleanup");
    }
}

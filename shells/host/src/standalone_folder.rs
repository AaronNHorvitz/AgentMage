//! Closed folder admission messages for the standalone evidence host (Decision 0150).
//!
//! These are data only. The host alone reads the selected folder and decides
//! every entry; a client sends one absolute folder and displays the answer.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Current folder admission view schema.
pub const FOLDER_ADMISSION_SCHEMA_VERSION: u16 = 1;
/// Deepest directory level enumerated below the selected folder.
pub const MAX_FOLDER_DEPTH: u32 = 8;
/// Most entries, files and directories together, enumerated in one folder.
pub const MAX_FOLDER_ENTRIES: u32 = 200;
/// Largest accepted file.
pub const MAX_FOLDER_FILE_BYTES: u64 = 128 * 1024;
/// Largest total of accepted file bytes.
pub const MAX_FOLDER_TOTAL_BYTES: u64 = 1024 * 1024;
/// Longest accepted line, so every citation can name one exact line.
pub const MAX_FOLDER_LINE_BYTES: u32 = 4096;
/// Longest selected folder path.
pub const MAX_FOLDER_PATH_BYTES: usize = 4096;
/// Longest relative entry path reported, the Linux path limit.
pub const MAX_FOLDER_ENTRY_PATH_BYTES: usize = 4096;

/// One closed folder request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case", deny_unknown_fields)]
pub enum FolderRequest {
    /// Read and admit one folder as the current immutable snapshot.
    Admit {
        /// Absolute folder path chosen by the person.
        folder: String,
    },
}

/// The host's answer to one folder request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
pub enum FolderAnswer {
    /// The folder is the current snapshot.
    Admitted {
        /// What was read and how each entry was decided.
        admission: FolderAdmissionView,
    },
    /// Nothing was admitted and the previous snapshot, if any, is unchanged.
    Refused {
        /// Why.
        refusal: FolderRefusal,
    },
}

/// Why a whole folder was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderRefusal {
    /// The path was relative, empty, too long or had `.` or `..` components.
    NotAbsolute,
    /// The folder is not strictly beneath the development disposable root.
    OutsideDisposableRoot,
    /// The folder is missing, is not a directory, or a component is a link.
    Unavailable,
    /// The folder is not owned by this user or is writable by others.
    NotPrivate,
    /// More than the entry limit was found.
    TooManyEntries,
    /// A directory is nested deeper than the depth limit.
    TooDeep,
    /// Accepted text would exceed the total byte limit.
    TooLarge,
    /// A directory changed while it was read.
    Changed,
    /// A question is prepared or running, so the snapshot cannot change.
    Busy,
    /// The host could not complete the read.
    Failed,
    /// The development activation's roots changed since the host started.
    ActivationChanged,
}

impl FolderRefusal {
    /// One stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotAbsolute => "standalone.folder.not-absolute",
            Self::OutsideDisposableRoot => "standalone.folder.outside-disposable-root",
            Self::Unavailable => "standalone.folder.unavailable",
            Self::NotPrivate => "standalone.folder.not-private",
            Self::TooManyEntries => "standalone.folder.too-many-entries",
            Self::TooDeep => "standalone.folder.too-deep",
            Self::TooLarge => "standalone.folder.too-large",
            Self::Changed => "standalone.folder.changed",
            Self::Busy => "standalone.folder.busy",
            Self::Failed => "standalone.folder.failed",
            Self::ActivationChanged => "standalone.folder.activation-changed",
        }
    }
}

/// How one entry was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderEntryDisposition {
    /// Admitted as an immutable prepared source.
    Accepted,
    /// Left out by rule: hidden, unsupported format or empty.
    Skipped,
    /// Refused because it was unsafe or unreadable as text.
    Rejected,
}

/// One enumerated file or link, in path order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderEntryView {
    /// Path relative to the selected folder, `/`-separated.
    pub path: String,
    /// The host's decision.
    pub disposition: FolderEntryDisposition,
    /// One stable reason code.
    pub reason_code: String,
    /// Bytes read, zero when not read.
    pub byte_len: u64,
    /// Digest of the admitted bytes, only for accepted entries.
    pub content_sha256: Option<String>,
    /// Prepared source identity, only for accepted entries.
    pub source_id: Option<String>,
}

/// Limits the host applied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderLimitsView {
    /// Deepest directory level enumerated.
    pub max_depth: u32,
    /// Most entries enumerated.
    pub max_entries: u32,
    /// Largest accepted file.
    pub max_file_bytes: u64,
    /// Largest total of accepted bytes.
    pub max_total_bytes: u64,
    /// Longest accepted line.
    pub max_line_bytes: u32,
}

impl FolderLimitsView {
    /// The limits this build applies.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            max_depth: MAX_FOLDER_DEPTH,
            max_entries: MAX_FOLDER_ENTRIES,
            max_file_bytes: MAX_FOLDER_FILE_BYTES,
            max_total_bytes: MAX_FOLDER_TOTAL_BYTES,
            max_line_bytes: MAX_FOLDER_LINE_BYTES,
        }
    }
}

/// The admitted snapshot as a person sees it. It grants nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderAdmissionView {
    /// View schema.
    pub schema_version: u16,
    /// Digest of this view with this field set to zeroes.
    pub admission_sha256: String,
    /// The selected absolute folder.
    pub folder: String,
    /// Workspace identity a question about this folder names.
    pub workspace_id: String,
    /// Every enumerated file or link, in path order.
    pub entries: Vec<FolderEntryView>,
    /// Count of accepted entries.
    pub accepted: u32,
    /// Count of skipped entries.
    pub skipped: u32,
    /// Count of rejected entries.
    pub rejected: u32,
    /// Total accepted bytes.
    pub admitted_bytes: u64,
    /// Limits applied.
    pub limits: FolderLimitsView,
}

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

impl FolderAdmissionView {
    /// Recomputes the digest this view should carry.
    pub fn compute_sha256(&self) -> Result<String, serde_json::Error> {
        let mut unsealed = self.clone();
        ZERO_SHA256.clone_into(&mut unsealed.admission_sha256);
        Ok(hex_sha256(&serde_json::to_vec(&unsealed)?))
    }

    /// Whether the view is internally consistent: schema, digest, counts,
    /// ordering, identities and limits. A client checks this before display.
    #[must_use]
    pub fn verify(&self) -> bool {
        let count = |disposition| {
            self.entries
                .iter()
                .filter(|entry| entry.disposition == disposition)
                .count()
        };
        let accepted_bytes = self
            .entries
            .iter()
            .filter(|entry| entry.disposition == FolderEntryDisposition::Accepted)
            .try_fold(0_u64, |total, entry| total.checked_add(entry.byte_len));
        self.schema_version == FOLDER_ADMISSION_SCHEMA_VERSION
            && self.limits == FolderLimitsView::current()
            && self.workspace_id == folder_workspace_id(&self.folder)
            && self.entries.len() <= MAX_FOLDER_ENTRIES as usize
            && self
                .entries
                .windows(2)
                .all(|pair| pair[0].path < pair[1].path)
            && self.entries.iter().all(FolderEntryView::well_formed)
            && usize::try_from(self.accepted).ok() == Some(count(FolderEntryDisposition::Accepted))
            && usize::try_from(self.skipped).ok() == Some(count(FolderEntryDisposition::Skipped))
            && usize::try_from(self.rejected).ok() == Some(count(FolderEntryDisposition::Rejected))
            && accepted_bytes == Some(self.admitted_bytes)
            && self.admitted_bytes <= MAX_FOLDER_TOTAL_BYTES
            && self
                .compute_sha256()
                .is_ok_and(|digest| digest == self.admission_sha256)
    }
}

impl FolderEntryView {
    fn well_formed(&self) -> bool {
        let accepted = self.disposition == FolderEntryDisposition::Accepted;
        !self.path.is_empty()
            && self.path.len() <= MAX_FOLDER_ENTRY_PATH_BYTES
            && !self.reason_code.is_empty()
            && self.byte_len <= MAX_FOLDER_FILE_BYTES
            && self.content_sha256.as_deref().is_some_and(valid_sha256) == accepted
            && self.source_id.as_deref().is_some_and(|id| !id.is_empty()) == accepted
    }
}

/// The workspace identity of a selected folder: stable for the same path.
#[must_use]
pub fn folder_workspace_id(folder: &str) -> String {
    format!(
        "standalone-evidence-{}",
        &hex_sha256(folder.as_bytes())[..24]
    )
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> FolderAdmissionView {
        let mut view = FolderAdmissionView {
            schema_version: FOLDER_ADMISSION_SCHEMA_VERSION,
            admission_sha256: ZERO_SHA256.to_owned(),
            folder: "/private/disposable/aurora".to_owned(),
            workspace_id: folder_workspace_id("/private/disposable/aurora"),
            entries: vec![
                FolderEntryView {
                    path: "a.md".to_owned(),
                    disposition: FolderEntryDisposition::Accepted,
                    reason_code: "folder.entry.accepted".to_owned(),
                    byte_len: 12,
                    content_sha256: Some("a".repeat(64)),
                    source_id: Some("folder-source-001".to_owned()),
                },
                FolderEntryView {
                    path: "b.pdf".to_owned(),
                    disposition: FolderEntryDisposition::Skipped,
                    reason_code: "folder.entry.unsupported-format".to_owned(),
                    byte_len: 0,
                    content_sha256: None,
                    source_id: None,
                },
            ],
            accepted: 1,
            skipped: 1,
            rejected: 0,
            admitted_bytes: 12,
            limits: FolderLimitsView::current(),
        };
        view.admission_sha256 = view.compute_sha256().expect("digest");
        view
    }

    #[test]
    fn a_sealed_view_verifies_and_every_altered_field_does_not() {
        let sealed = view();
        assert!(sealed.verify());
        type Change = Box<dyn Fn(&mut FolderAdmissionView)>;
        let mut changes: Vec<Change> = vec![
            Box::new(|view| view.accepted = 2),
            Box::new(|view| view.admitted_bytes = 13),
            Box::new(|view| view.entries.swap(0, 1)),
            Box::new(|view| view.entries[0].source_id = None),
            Box::new(|view| view.entries[1].content_sha256 = Some("b".repeat(64))),
            Box::new(|view| view.workspace_id = "standalone-evidence-other".to_owned()),
            Box::new(|view| view.limits.max_entries = 201),
            Box::new(|view| view.folder.push('x')),
        ];
        for change in changes.drain(..) {
            let mut altered = sealed.clone();
            change(&mut altered);
            assert!(!altered.verify());
            // Resealing does not make an inconsistent view valid either,
            // except where only the digest was wrong.
            altered.admission_sha256 = altered.compute_sha256().expect("digest");
            assert!(!altered.verify());
        }
    }

    #[test]
    fn requests_and_answers_are_closed() {
        let request: FolderRequest =
            serde_json::from_str(r#"{"request":"admit","folder":"/x"}"#).expect("admit");
        assert_eq!(
            request,
            FolderRequest::Admit {
                folder: "/x".to_owned()
            }
        );
        for refused in [
            r#"{"request":"admit","folder":"/x","extra":1}"#,
            r#"{"request":"list"}"#,
            r#"{"request":"admit"}"#,
        ] {
            assert!(serde_json::from_str::<FolderRequest>(refused).is_err());
        }
        let answer = FolderAnswer::Refused {
            refusal: FolderRefusal::Busy,
        };
        let bytes = serde_json::to_vec(&answer).expect("encode");
        assert_eq!(
            serde_json::from_slice::<FolderAnswer>(&bytes).expect("decode"),
            answer
        );
        assert_eq!(FolderRefusal::Busy.code(), "standalone.folder.busy");
    }
}

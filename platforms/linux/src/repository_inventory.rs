//! Bounded Git inventory evidence for repository-map projection on Linux.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use agentmage_kernel_engine::propagation::CancellationToken;
use sha2::{Digest, Sha256};

use crate::repository_safety::{
    LinuxRepositoryCollector, LinuxRepositoryError, LinuxRepositoryErrorKind, LinuxRepositoryScope,
};

const MAX_INVENTORY_BYTES: usize = 32 * 1024 * 1024;
const MAX_INVENTORY_ENTRIES: usize = 100_000;

/// Git index mode retained as an inert object-kind hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxRepositoryObjectHint {
    /// A regular file, including an executable regular file.
    RegularFile,
    /// A symbolic link whose target must not be followed.
    SymbolicLink,
    /// A Gitlink whose submodule must not be entered.
    Gitlink,
    /// No index mode exists; the host must classify the held worktree object.
    WorktreeObject,
}

/// Exact Git state retained before held-object projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinuxRepositoryInventoryState {
    /// A path represented by one or more Git index records.
    Tracked {
        /// Exact lowercase object identifier selected from the index.
        index_object_id: String,
        /// Exact six-digit index mode.
        index_mode: String,
        /// Whether the index differs from the exact `HEAD` tree.
        staged_changed: bool,
        /// Whether the index contains unresolved nonzero stages for this path.
        conflicted: bool,
    },
    /// A worktree object not represented by the index or ignore rules.
    Untracked,
    /// A worktree object matched by applicable Git ignore rules.
    Ignored,
}

/// One canonical UTF-8 path discovered through hardened Git observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxRepositoryInventoryEntry {
    /// Canonical path components relative to the repository root.
    pub path: Vec<String>,
    /// Inert object-kind hint from the Git index, when present.
    pub object_hint: LinuxRepositoryObjectHint,
    /// Exact pre-projection Git state.
    pub state: LinuxRepositoryInventoryState,
}

/// Complete bounded Git evidence required before held-file projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxRepositoryInventory {
    /// Stable digest of the verified repository path and identity.
    pub repository_sha256: String,
    /// Git object format used by index and `HEAD` object identifiers.
    pub object_format: String,
    /// Exact symbolic branch ref, or `None` for detached `HEAD`.
    pub branch: Option<String>,
    /// Exact commit object identifier at collection time.
    pub commit_id: String,
    /// Sorted complete discovered path evidence.
    pub entries: Vec<LinuxRepositoryInventoryEntry>,
    /// Digest over every raw Git observation used to construct this evidence.
    pub git_evidence_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IndexRecord {
    mode: String,
    object_id: String,
    stage: u8,
}

impl LinuxRepositoryCollector {
    /// Collects a bounded path-bearing Git inventory without opening worktree objects.
    pub fn collect_inventory(
        &self,
        scope: &LinuxRepositoryScope,
    ) -> Result<LinuxRepositoryInventory, LinuxRepositoryError> {
        self.collect_inventory_inner(scope, None)
    }

    /// Collects the same inventory while honoring one propagated cancellation token.
    pub fn collect_inventory_cancellable(
        &self,
        scope: &LinuxRepositoryScope,
        cancellation: &CancellationToken,
    ) -> Result<LinuxRepositoryInventory, LinuxRepositoryError> {
        self.collect_inventory_inner(scope, Some(cancellation))
    }

    fn collect_inventory_inner(
        &self,
        scope: &LinuxRepositoryScope,
        cancellation: Option<&CancellationToken>,
    ) -> Result<LinuxRepositoryInventory, LinuxRepositoryError> {
        let object_format = required_text(self.observe_inventory(
            scope,
            &["rev-parse", "--show-object-format"],
            1_024,
            cancellation,
        )?)?;
        if !matches!(object_format.as_str(), "sha1" | "sha256") {
            return Err(failed());
        }
        let commit_id = required_text(self.observe_inventory(
            scope,
            &["rev-parse", "--verify", "HEAD"],
            1_024,
            cancellation,
        )?)?;
        let expected_object_bytes = if object_format == "sha1" { 40 } else { 64 };
        if commit_id.len() != expected_object_bytes
            || !commit_id.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(failed());
        }
        let branch = self
            .observe_inventory_optional(
                scope,
                &["symbolic-ref", "--quiet", "HEAD"],
                1_024,
                cancellation,
            )?
            .map(required_text)
            .transpose()?;
        let index = self.observe_inventory(
            scope,
            &["ls-files", "--stage", "-z"],
            MAX_INVENTORY_BYTES,
            cancellation,
        )?;
        let head = self.observe_inventory(
            scope,
            &["ls-tree", "-r", "-z", "--full-tree", "HEAD"],
            MAX_INVENTORY_BYTES,
            cancellation,
        )?;
        let untracked = self.observe_inventory(
            scope,
            &["ls-files", "--others", "--exclude-standard", "-z"],
            MAX_INVENTORY_BYTES,
            cancellation,
        )?;
        let ignored = self.observe_inventory(
            scope,
            &[
                "ls-files",
                "--others",
                "--ignored",
                "--exclude-standard",
                "-z",
            ],
            MAX_INVENTORY_BYTES,
            cancellation,
        )?;

        let head_records = parse_head_records(&head, expected_object_bytes)?;
        let mut entries = parse_index_records(&index, &head_records, expected_object_bytes)?;
        append_worktree_records(
            &mut entries,
            &untracked,
            LinuxRepositoryInventoryState::Untracked,
        )?;
        append_worktree_records(
            &mut entries,
            &ignored,
            LinuxRepositoryInventoryState::Ignored,
        )?;
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        if entries.len() > MAX_INVENTORY_ENTRIES
            || entries.windows(2).any(|pair| pair[0].path == pair[1].path)
        {
            return Err(failed());
        }

        let git_evidence_sha256 = framed_digest(&[
            object_format.as_bytes(),
            commit_id.as_bytes(),
            branch.as_deref().unwrap_or("detached").as_bytes(),
            &index,
            &head,
            &untracked,
            &ignored,
        ])?;
        Ok(LinuxRepositoryInventory {
            repository_sha256: scope.repository_path_sha256().to_owned(),
            object_format,
            branch,
            commit_id,
            entries,
            git_evidence_sha256,
        })
    }
}

fn parse_index_records(
    bytes: &[u8],
    head: &BTreeMap<Vec<String>, IndexRecord>,
    object_bytes: usize,
) -> Result<Vec<LinuxRepositoryInventoryEntry>, LinuxRepositoryError> {
    let mut grouped = BTreeMap::<Vec<String>, Vec<IndexRecord>>::new();
    for record in nul_records(bytes)? {
        let (metadata, path) = split_once(record, b'\t')?;
        let fields = metadata.split(|byte| *byte == b' ').collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(failed());
        }
        let mode = ascii(fields[0])?;
        let object_id = ascii(fields[1])?;
        let stage = ascii(fields[2])?.parse::<u8>().map_err(|_| failed())?;
        if !valid_mode(&mode)
            || object_id.len() != object_bytes
            || !object_id.bytes().all(|byte| byte.is_ascii_hexdigit())
            || stage > 3
        {
            return Err(failed());
        }
        grouped
            .entry(path_components(path)?)
            .or_default()
            .push(IndexRecord {
                mode,
                object_id: object_id.to_ascii_lowercase(),
                stage,
            });
        if grouped.len() > MAX_INVENTORY_ENTRIES {
            return Err(failed());
        }
    }

    grouped
        .into_iter()
        .map(|(path, mut records)| {
            records.sort_by_key(|record| record.stage);
            let selected = records
                .iter()
                .find(|record| record.stage == 0)
                .unwrap_or(&records[0]);
            let conflicted = records.iter().any(|record| record.stage != 0);
            let staged_changed = conflicted || head.get(&path) != Some(selected);
            Ok(LinuxRepositoryInventoryEntry {
                path,
                object_hint: hint_for_mode(&selected.mode)?,
                state: LinuxRepositoryInventoryState::Tracked {
                    index_object_id: selected.object_id.clone(),
                    index_mode: selected.mode.clone(),
                    staged_changed,
                    conflicted,
                },
            })
        })
        .collect()
}

fn parse_head_records(
    bytes: &[u8],
    object_bytes: usize,
) -> Result<BTreeMap<Vec<String>, IndexRecord>, LinuxRepositoryError> {
    let mut output = BTreeMap::new();
    for record in nul_records(bytes)? {
        let (metadata, path) = split_once(record, b'\t')?;
        let fields = metadata.split(|byte| *byte == b' ').collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(failed());
        }
        let mode = ascii(fields[0])?;
        let object_id = ascii(fields[2])?;
        if !valid_mode(&mode)
            || object_id.len() != object_bytes
            || !object_id.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(failed());
        }
        if output
            .insert(
                path_components(path)?,
                IndexRecord {
                    mode,
                    object_id: object_id.to_ascii_lowercase(),
                    stage: 0,
                },
            )
            .is_some()
            || output.len() > MAX_INVENTORY_ENTRIES
        {
            return Err(failed());
        }
    }
    Ok(output)
}

fn append_worktree_records(
    output: &mut Vec<LinuxRepositoryInventoryEntry>,
    bytes: &[u8],
    state: LinuxRepositoryInventoryState,
) -> Result<(), LinuxRepositoryError> {
    for record in nul_records(bytes)? {
        output.push(LinuxRepositoryInventoryEntry {
            path: path_components(record)?,
            object_hint: LinuxRepositoryObjectHint::WorktreeObject,
            state: state.clone(),
        });
        if output.len() > MAX_INVENTORY_ENTRIES {
            return Err(failed());
        }
    }
    Ok(())
}

fn hint_for_mode(mode: &str) -> Result<LinuxRepositoryObjectHint, LinuxRepositoryError> {
    match mode {
        "100644" | "100755" => Ok(LinuxRepositoryObjectHint::RegularFile),
        "120000" => Ok(LinuxRepositoryObjectHint::SymbolicLink),
        "160000" => Ok(LinuxRepositoryObjectHint::Gitlink),
        _ => Err(failed()),
    }
}

fn valid_mode(mode: &str) -> bool {
    matches!(mode, "100644" | "100755" | "120000" | "160000")
}

fn nul_records(bytes: &[u8]) -> Result<impl Iterator<Item = &[u8]>, LinuxRepositoryError> {
    if !bytes.is_empty() && !bytes.ends_with(&[0]) {
        return Err(failed());
    }
    Ok(bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty()))
}

fn split_once(bytes: &[u8], delimiter: u8) -> Result<(&[u8], &[u8]), LinuxRepositoryError> {
    let index = bytes
        .iter()
        .position(|byte| *byte == delimiter)
        .ok_or_else(failed)?;
    Ok((&bytes[..index], &bytes[index + 1..]))
}

fn path_components(bytes: &[u8]) -> Result<Vec<String>, LinuxRepositoryError> {
    let path = std::str::from_utf8(bytes).map_err(|_| failed())?;
    let components = path.split('/').map(str::to_owned).collect::<Vec<_>>();
    if components.is_empty()
        || components.iter().any(|component| {
            component.is_empty()
                || matches!(component.as_str(), "." | "..")
                || component.chars().any(char::is_control)
        })
    {
        return Err(failed());
    }
    Ok(components)
}

fn required_text(bytes: Vec<u8>) -> Result<String, LinuxRepositoryError> {
    let value = String::from_utf8(bytes)
        .map_err(|_| failed())?
        .trim()
        .to_owned();
    if value.is_empty() || value.len() > 1_024 || value.chars().any(char::is_control) {
        return Err(failed());
    }
    Ok(value)
}

fn ascii(bytes: &[u8]) -> Result<String, LinuxRepositoryError> {
    if !bytes.is_ascii() {
        return Err(failed());
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| failed())
}

fn framed_digest(parts: &[&[u8]]) -> Result<String, LinuxRepositoryError> {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(
            u64::try_from(part.len())
                .map_err(|_| failed())?
                .to_be_bytes(),
        );
        digest.update(part);
    }
    let mut output = String::with_capacity(64);
    for byte in digest.finalize() {
        write!(&mut output, "{byte:02x}").map_err(|_| failed())?;
    }
    Ok(output)
}

const fn failed() -> LinuxRepositoryError {
    LinuxRepositoryError::from_kind(LinuxRepositoryErrorKind::ObservationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsers_preserve_spaces_tabs_and_index_changes_without_path_ambiguity() {
        let head = b"100644 blob 0123456789012345678901234567890123456789\tspace name.rs\0";
        let index = b"100755 1123456789012345678901234567890123456789 0\tspace name.rs\0";
        let head = parse_head_records(head, 40).expect("HEAD records");
        let records = parse_index_records(index, &head, 40).expect("index records");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].path, ["space name.rs"]);
        assert_eq!(
            records[0].object_hint,
            LinuxRepositoryObjectHint::RegularFile
        );
        assert!(matches!(
            records[0].state,
            LinuxRepositoryInventoryState::Tracked {
                staged_changed: true,
                conflicted: false,
                ..
            }
        ));
    }

    #[test]
    fn malformed_records_and_unsafe_paths_fail_closed() {
        let empty = BTreeMap::new();
        assert!(parse_index_records(b"100644 bad 0\tfile\0", &empty, 40).is_err());
        assert!(
            parse_head_records(
                b"100644 blob 0123456789012345678901234567890123456789\t../x\0",
                40
            )
            .is_err()
        );
        assert!(nul_records(b"unterminated").is_err());
    }
}

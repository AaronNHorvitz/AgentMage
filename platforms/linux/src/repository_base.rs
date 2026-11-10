// SPDX-License-Identifier: Apache-2.0
//! Bounded immutable-base reads for the existing coding candidate planner.

use std::collections::BTreeMap;

use agentmage_kernel_engine::local_commit::CommitFileMode;
use sha2::{Digest, Sha256};

use crate::repository_safety::{
    LinuxRepositoryCollector, LinuxRepositoryError, LinuxRepositoryErrorKind, LinuxRepositoryScope,
};

const MAX_PATHS: usize = 16;
const MAX_BLOB_BYTES: usize = 128 * 1024;
const MAX_TREE_BYTES: usize = crate::sandbox::MAX_OUTPUT_BYTES;
const MAX_TREE_ENTRIES: usize = 100_000;

#[derive(Debug)]
struct Entry {
    kind: &'static str,
    object: String,
    mode: Option<CommitFileMode>,
}

/// Exact content and Git mode of an ordinary file at the immutable base.
/// Absence is represented separately by the containing `Option`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BaseFilePreimage {
    pub(crate) sha256: String,
    pub(crate) mode: CommitFileMode,
}

impl LinuxRepositoryCollector {
    // Called only after source/metadata read consent by the original live owner.
    // Use its linked read-only scope so Git's object/config reads stay inside
    // the existing offline worker. No index, worktree bytes or mutable ref is
    // used as a substitute for the exact creation base. Replacement refs are
    // disabled on every base read, even before preservation rejects them.
    pub(crate) fn collect_base_preimages(
        &self,
        scope: &LinuxRepositoryScope,
        base: &str,
        paths: &[String],
    ) -> Result<BTreeMap<String, Option<BaseFilePreimage>>, LinuxRepositoryError> {
        collect_with(base, paths, |arguments, limit| {
            self.observe_inventory(scope, arguments, limit, None)
        })
    }
}

// The observation seam is private to this module. Tests can exercise malformed
// Git output without accepting caller-supplied base proofs in product code.
fn collect_with(
    base: &str,
    paths: &[String],
    mut observe: impl FnMut(&[&str], usize) -> Result<Vec<u8>, LinuxRepositoryError>,
) -> Result<BTreeMap<String, Option<BaseFilePreimage>>, LinuxRepositoryError> {
    let format = match base.len() {
        40 => "sha1",
        64 => "sha256",
        _ => return Err(failed()),
    };
    if !object_id(base, base.len())
        || paths.is_empty()
        || paths.len() > MAX_PATHS
        || paths.iter().any(|path| !valid_path(path))
        || paths
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != paths.len()
    {
        return Err(failed());
    }
    if observe(&["--no-replace-objects", "cat-file", "-t", base], 32)? != b"commit\n" {
        return Err(failed());
    }
    let raw = observe(
        &[
            "--no-replace-objects",
            "ls-tree",
            "-r",
            "-t",
            "-z",
            "--full-tree",
            base,
        ],
        MAX_TREE_BYTES,
    )?;
    let entries = parse_tree(&raw, base.len())?;
    let mut output = BTreeMap::new();
    for path in paths {
        // A missing descendant of a symlink/gitlink/blob is not an absent
        // ordinary file. Nor may a selected directory become a source file.
        for (offset, _) in path.match_indices('/') {
            if entries
                .get(&path[..offset])
                .is_some_and(|e| e.kind != "tree")
            {
                return Err(failed());
            }
        }
        let preimage = match entries.get(path) {
            None => None,
            Some(entry) if entry.kind == "blob" => {
                let bytes = observe(
                    &["--no-replace-objects", "cat-file", "blob", &entry.object],
                    MAX_BLOB_BYTES,
                )?;
                if bytes.len() > MAX_BLOB_BYTES
                    || crate::linux_git_blob_object_id(format, &bytes).map_err(|_| failed())?
                        != entry.object
                {
                    return Err(failed());
                }
                Some(BaseFilePreimage {
                    sha256: Sha256::digest(&bytes)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                    mode: entry.mode.ok_or_else(failed)?,
                })
            }
            Some(_) => return Err(failed()),
        };
        output.insert(path.clone(), preimage);
    }
    Ok(output)
}

fn parse_tree(
    raw: &[u8],
    object_bytes: usize,
) -> Result<BTreeMap<String, Entry>, LinuxRepositoryError> {
    if raw.len() > MAX_TREE_BYTES || (!raw.is_empty() && !raw.ends_with(&[0])) {
        return Err(failed());
    }
    let mut entries = BTreeMap::new();
    if raw.is_empty() {
        return Ok(entries);
    }
    for record in raw[..raw.len() - 1].split(|b| *b == 0) {
        let tab = record.iter().position(|b| *b == b'\t').ok_or_else(failed)?;
        let (metadata, rest) = record.split_at(tab);
        let path = &rest[1..];
        let metadata = std::str::from_utf8(metadata).map_err(|_| failed())?;
        let path = std::str::from_utf8(path).map_err(|_| failed())?;
        let fields = metadata.split(' ').collect::<Vec<_>>();
        if fields.len() != 3 || !valid_path(path) || !object_id(fields[2], object_bytes) {
            return Err(failed());
        }
        let kind = match (fields[0], fields[1]) {
            ("100644" | "100755", "blob") => "blob",
            ("040000", "tree") => "tree",
            ("120000", "blob") => "symlink",
            ("160000", "commit") => "gitlink",
            _ => return Err(failed()),
        };
        if entries
            .insert(
                path.to_owned(),
                Entry {
                    kind,
                    object: fields[2].into(),
                    mode: match fields[0] {
                        "100644" => Some(CommitFileMode::Regular),
                        "100755" => Some(CommitFileMode::Executable),
                        _ => None,
                    },
                },
            )
            .is_some()
            || entries.len() > MAX_TREE_ENTRIES
        {
            return Err(failed());
        }
    }
    for path in entries.keys() {
        for (offset, _) in path.match_indices('/') {
            if entries
                .get(&path[..offset])
                .is_none_or(|e| e.kind != "tree")
            {
                return Err(failed());
            }
        }
    }
    Ok(entries)
}

fn object_id(value: &str, bytes: usize) -> bool {
    value.len() == bytes
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && value.bytes().any(|b| b != b'0')
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && path
            .split('/')
            .all(|part| !part.is_empty() && !matches!(part, "." | ".."))
        && !path.chars().any(char::is_control)
}

fn failed() -> LinuxRepositoryError {
    LinuxRepositoryError::from_kind(LinuxRepositoryErrorKind::ObservationFailed)
}

#[cfg(test)]
#[path = "repository_base_tests.rs"]
mod tests;

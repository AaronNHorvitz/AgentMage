//! Bounded operating-system polling adapter for an admitted Obsidian vault.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use agentmage_capability_knowledge::{
    ObsidianEntryKind, ObsidianIndexUpdate, ObsidianNoteInput, ObsidianVaultIndex,
    ObsidianVaultSelection, ObsidianVaultSnapshot, ObsidianWatchEvent, ObsidianWatchEventKind,
};
use agentmage_kernel_contracts::WorkspacePath;
use sha2::{Digest, Sha256};

const MAX_ENTRIES: usize = 100_000;

/// Content-free host watcher failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObsidianWatcherError {
    /// The selected host root or an entry was unavailable or unsupported.
    Filesystem,
    /// A host path could not be represented by the admitted workspace scope.
    InvalidPath,
    /// The bounded scan or parser rejected the observed snapshot.
    InvalidSnapshot,
    /// The disposable index rejected a stale or inconsistent event batch.
    Index,
}

/// Explicitly polled watcher over one already-admitted local vault root.
pub struct ObsidianPollingWatcher {
    host_root: PathBuf,
    selection: ObsidianVaultSelection,
    observed: BTreeMap<WorkspacePath, String>,
    index_revision: u64,
}

impl ObsidianPollingWatcher {
    /// Captures and publishes the initial derived projection.
    pub fn start(
        host_root: PathBuf,
        selection: ObsidianVaultSelection,
        index: &mut ObsidianVaultIndex,
    ) -> Result<(Self, ObsidianIndexUpdate), ObsidianWatcherError> {
        let inputs = scan(&host_root, &selection)?;
        let observed = digests(&inputs);
        let snapshot = ObsidianVaultSnapshot::from_snapshots(&selection, inputs)
            .map_err(|_| ObsidianWatcherError::InvalidSnapshot)?;
        let update = index
            .rebuild(&snapshot)
            .map_err(|_| ObsidianWatcherError::Index)?;
        Ok((
            Self {
                host_root,
                selection,
                observed,
                index_revision: update.report.revision,
            },
            update,
        ))
    }

    /// Polls exact host state and atomically replaces only the derived projection when changed.
    pub fn poll(
        &mut self,
        index: &mut ObsidianVaultIndex,
    ) -> Result<Option<(Vec<ObsidianWatchEvent>, ObsidianIndexUpdate)>, ObsidianWatcherError> {
        let inputs = scan(&self.host_root, &self.selection)?;
        let current = digests(&inputs);
        let events = changes(&self.observed, &current);
        if events.is_empty() {
            return Ok(None);
        }
        let snapshot = ObsidianVaultSnapshot::from_snapshots(&self.selection, inputs)
            .map_err(|_| ObsidianWatcherError::InvalidSnapshot)?;
        let update = index
            .apply_watch_batch(self.index_revision, &events, &snapshot)
            .map_err(|_| ObsidianWatcherError::Index)?;
        self.observed = current;
        self.index_revision = update.report.revision;
        Ok(Some((events, update)))
    }
}

fn scan(
    host_root: &Path,
    selection: &ObsidianVaultSelection,
) -> Result<Vec<ObsidianNoteInput>, ObsidianWatcherError> {
    if !host_root.is_absolute() || !host_root.is_dir() {
        return Err(ObsidianWatcherError::Filesystem);
    }
    let mut pending = vec![(host_root.to_path_buf(), Vec::<String>::new())];
    let mut values = Vec::new();
    while let Some((directory, relative)) = pending.pop() {
        let mut entries = fs::read_dir(directory)
            .map_err(|_| ObsidianWatcherError::Filesystem)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ObsidianWatcherError::Filesystem)?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries.into_iter().rev() {
            if values.len() + pending.len() >= MAX_ENTRIES {
                return Err(ObsidianWatcherError::InvalidSnapshot);
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| ObsidianWatcherError::InvalidPath)?;
            let mut components = relative.clone();
            components.push(name.clone());
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|_| ObsidianWatcherError::Filesystem)?;
            if metadata.file_type().is_dir() {
                pending.push((entry.path(), components));
                continue;
            }
            let kind = if metadata.file_type().is_symlink() {
                ObsidianEntryKind::SymbolicLink
            } else if metadata.file_type().is_file() {
                ObsidianEntryKind::RegularFile
            } else {
                ObsidianEntryKind::Special
            };
            let content = if kind == ObsidianEntryKind::RegularFile {
                fs::read(entry.path()).map_err(|_| ObsidianWatcherError::Filesystem)?
            } else {
                Vec::new()
            };
            let mut workspace_components = selection
                .root()
                .components()
                .iter()
                .map(|item| item.as_str().to_owned())
                .collect::<Vec<_>>();
            workspace_components.extend(components);
            let path = WorkspacePath::new(
                selection.root().workspace_id().clone(),
                workspace_components,
            )
            .map_err(|_| ObsidianWatcherError::InvalidPath)?;
            values.push(ObsidianNoteInput {
                path,
                entry_kind: kind,
                hidden: name.starts_with('.'),
                cloud_synchronized: false,
                content_sha256: hex_digest(&content),
                content,
            });
        }
    }
    values.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(values)
}

fn digests(inputs: &[ObsidianNoteInput]) -> BTreeMap<WorkspacePath, String> {
    inputs
        .iter()
        .filter(|item| item.entry_kind == ObsidianEntryKind::RegularFile && !item.hidden)
        .map(|item| (item.path.clone(), item.content_sha256.clone()))
        .collect()
}

fn changes(
    before: &BTreeMap<WorkspacePath, String>,
    after: &BTreeMap<WorkspacePath, String>,
) -> Vec<ObsidianWatchEvent> {
    let mut events = Vec::new();
    for (path, digest) in after {
        let kind = match before.get(path) {
            None => Some(ObsidianWatchEventKind::Created),
            Some(previous) if previous != digest => Some(ObsidianWatchEventKind::Modified),
            Some(_) => None,
        };
        if let Some(kind) = kind {
            events.push(ObsidianWatchEvent {
                path: path.clone(),
                kind,
                content_sha256: Some(digest.clone()),
            });
        }
    }
    for path in before.keys().filter(|path| !after.contains_key(*path)) {
        events.push(ObsidianWatchEvent {
            path: path.clone(),
            kind: ObsidianWatchEventKind::Deleted,
            content_sha256: None,
        });
    }
    events.sort_by(|left, right| left.path.cmp(&right.path));
    events
}

fn hex_digest(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_kernel_contracts::{
        StorageFilesystemClass, StrictLocalStorageObservation, WorkspaceId, WorkspaceScopePath,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> (PathBuf, ObsidianVaultSelection) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("agentmage-vault-watch-{nonce}"));
        fs::create_dir(&root).expect("root");
        let scope = WorkspaceScopePath::new(WorkspaceId::from_raw("watch-workspace"), ["Vault"])
            .expect("scope");
        let selection = ObsidianVaultSelection::admit(
            scope,
            StrictLocalStorageObservation {
                filesystem: StorageFilesystemClass::Local,
                symlink_free: true,
                synchronization_marker: None,
                root_identity_sha256: [7; 32],
            },
            Vec::new(),
        )
        .expect("selection");
        (root, selection)
    }

    #[test]
    fn polling_updates_only_the_disposable_index_and_emits_receipt() {
        let (root, selection) = fixture();
        fs::write(root.join("one.md"), "# One\n").expect("write one");
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        let (mut watcher, initial) =
            ObsidianPollingWatcher::start(root.clone(), selection, &mut index).expect("start");
        assert_eq!(initial.report.note_count, 1);
        assert!(watcher.poll(&mut index).expect("unchanged").is_none());
        fs::write(root.join("one.md"), "# Changed\n").expect("modify");
        fs::write(root.join("two.md"), "# Two\n").expect("create");
        let (events, update) = watcher
            .poll(&mut index)
            .expect("poll")
            .expect("changed");
        assert_eq!(events.len(), 2);
        assert_eq!(update.report.note_count, 2);
        assert!(!update.report.canonical);
        assert!(!update.receipt.source_files_mutated);
        assert!(!update.receipt.external_process_started);
        assert!(!update.receipt.network_accessed);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_is_observed_without_following_and_fails_closed() {
        use std::os::unix::fs::symlink;
        let (root, selection) = fixture();
        fs::write(root.join("one.md"), "# One\n").expect("write");
        symlink(root.join("one.md"), root.join("link.md")).expect("symlink");
        let mut index = ObsidianVaultIndex::in_memory().expect("index");
        assert!(matches!(
            ObsidianPollingWatcher::start(root.clone(), selection, &mut index),
            Err(ObsidianWatcherError::InvalidSnapshot)
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }
}

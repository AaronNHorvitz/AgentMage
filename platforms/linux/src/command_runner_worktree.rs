// SPDX-License-Identifier: MIT OR Apache-2.0
//! Complete bounded worktree observations and immutable command projections.

use std::collections::BTreeMap;
use std::os::fd::AsRawFd;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::{LinuxAuthorizedWorkspace, LinuxPathAdapter};
use agentmage_kernel_contracts::{
    FilePreimage, HeldWorkspaceObject, PathResolutionIntent, PlatformPathAdapter,
    WorkspaceObjectKind, WorkspacePath,
};
use agentmage_kernel_engine::command_runner::SealedCommandWorkspaceInput;
use agentmage_kernel_engine::propagation::EffectCancellationObservation;
use rustix::fd::OwnedFd;
use serde_json::json;
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

const MAX_ENTRIES: usize = 4096;
const MAX_DIRECTORIES: usize = 256;
const MAX_DEPTH: usize = 16;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_SEALED_FILES: usize = 128;

struct SealedFile {
    path: Vec<String>,
    mode: u32,
    descriptor: OwnedFd,
    preimage: FilePreimage,
}

/// Private projection owned until the original supervisor confirms cleanup.
/// No mutable source directory is passed to the native child in this mode.
pub(super) struct SealedWorktree {
    worktree_root_sha256: String,
    observation_sha256: String,
    root_mode: u32,
    directories: BTreeMap<Vec<String>, u32>,
    files: Vec<SealedFile>,
}

impl SealedWorktree {
    pub(super) fn capture(
        workspace: &LinuxAuthorizedWorkspace,
        cancellation: &dyn EffectCancellationObservation,
        command_deadline: Instant,
    ) -> Option<Self> {
        let deadline = command_deadline.min(Instant::now() + Duration::from_secs(2));
        let mut sealed = Self {
            worktree_root_sha256: workspace.root_path_sha256().into(),
            observation_sha256: String::new(),
            root_mode: 0,
            directories: BTreeMap::new(),
            files: Vec::new(),
        };
        let before =
            WorkspaceObservation::scan(workspace, deadline, Some(&mut sealed), Some(cancellation))?;
        // No partial tree or race recovery. Both complete observations include
        // identity and ctime; each file copy separately checks its exact preimage.
        if before != WorkspaceObservation::scan(workspace, deadline, None, Some(cancellation))? {
            return None;
        }
        sealed.files.sort_by(|a, b| a.path.cmp(&b.path));
        sealed.observation_sha256 = before.digest;
        Some(sealed)
    }

    pub(super) fn input_record(&self) -> SealedCommandWorkspaceInput {
        // file_projection verified these preimages while copying, then sealed
        // the descriptors used by mount_arguments. No later host read supplies
        // this identity. Delimited canonical JSON binds names, modes and bytes.
        let manifest = json!({"schema_version":1,"mode":"sealed-read-only-tree-v1",
            "root_mode":self.root_mode,
            "directories":self.directories.iter().collect::<Vec<_>>(),
            "files":self.files.iter().map(|file| json!({"path":file.path,
                "mode":file.mode,"preimage":file.preimage})).collect::<Vec<_>>()});
        SealedCommandWorkspaceInput {
            schema_version: 1,
            worktree_root_sha256: self.worktree_root_sha256.clone(),
            observation_sha256: self.observation_sha256.clone(),
            projection_sha256: hash(manifest.to_string().as_bytes()),
            files: self.files.len() as u32,
            directories: self.directories.len() as u32 + 1,
            file_bytes: self.files.iter().map(|file| file.preimage.byte_len()).sum(),
        }
    }

    pub(super) fn open_file_properties(&self, parent_pid: u32) -> Vec<String> {
        self.files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                super::open_file_property(
                    parent_pid,
                    file.descriptor.as_raw_fd(),
                    &format!("source{index}"),
                )
            })
            .collect()
    }

    pub(super) fn mount_arguments(&self, command: &mut Command, first_descriptor: u32) {
        // Create all directories before restoring their original permissions.
        // /work's final remount covers every directory; file mounts are already
        // read-only, because remount-ro is not recursive across nested mounts.
        command.args(["--tmpfs", "/work"]);
        for path in self.directories.keys() {
            command.args(["--dir", &guest_path(path)]);
        }
        for (index, file) in self.files.iter().enumerate() {
            command.args([
                "--perms",
                &format!("{:04o}", file.mode),
                "--ro-bind-data",
                &(first_descriptor + index as u32).to_string(),
                &guest_path(&file.path),
            ]);
        }
        for (path, mode) in self.directories.iter().rev() {
            command.args(["--chmod", &format!("{mode:04o}"), &guest_path(path)]);
        }
        command.args([
            "--chmod",
            &format!("{:04o}", self.root_mode),
            "/work",
            "--remount-ro",
            "/work",
        ]);
    }

    pub(super) fn into_descriptors(self) -> Vec<OwnedFd> {
        self.files.into_iter().map(|file| file.descriptor).collect()
    }
}

fn guest_path(components: &[String]) -> String {
    format!("/work/{}", components.join("/"))
}

// Component-only access to real sealing, never a production executor injection.
#[cfg(test)]
pub(crate) fn capture_input_for_test(
    workspace: &LinuxAuthorizedWorkspace,
    cancellation: &dyn EffectCancellationObservation,
) -> Option<SealedCommandWorkspaceInput> {
    Some(
        SealedWorktree::capture(
            workspace,
            cancellation,
            Instant::now() + Duration::from_secs(2),
        )?
        .input_record(),
    )
}

fn available(deadline: Instant, cancellation: Option<&dyn EffectCancellationObservation>) -> bool {
    Instant::now() < deadline
        && cancellation.is_none_or(|probe| matches!(probe.observe_effect_cancellation(), Ok(None)))
}

#[derive(PartialEq, Eq)]
pub(crate) struct WorkspaceObservation {
    pub(crate) root: String,
    pub(crate) digest: String,
    pub(crate) entries: usize,
    pub(crate) bytes: u64,
}

impl WorkspaceObservation {
    // All names count, including hidden/generated objects and the local Git
    // pointer. Nothing is followed outside the held root or silently excluded.
    pub(crate) fn capture(workspace: &LinuxAuthorizedWorkspace) -> Option<Self> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let first = Self::scan(workspace, deadline, None, None)?;
        (first == Self::scan(workspace, deadline, None, None)?).then_some(first)
    }

    fn scan(
        workspace: &LinuxAuthorizedWorkspace,
        deadline: Instant,
        mut sealed: Option<&mut SealedWorktree>,
        cancellation: Option<&dyn EffectCancellationObservation>,
    ) -> Option<Self> {
        workspace.revalidate().ok()?;
        let root_before = crate::snapshot(&workspace.root_descriptor, None).ok()?;
        if let Some(sealed) = sealed.as_mut() {
            if root_before.mode & 0o7000 != 0 {
                return None;
            }
            sealed.root_mode = root_before.mode & 0o777;
        }
        let adapter = LinuxPathAdapter::new(workspace.adapter_instance_id.clone(), MAX_FILE_BYTES);
        let mut pending = vec![Vec::<String>::new()];
        let mut entries = BTreeMap::new();
        let mut bytes = 0_u64;
        let mut directories = 0;
        while let Some(components) = pending.pop() {
            if !available(deadline, cancellation) || components.len() > MAX_DEPTH {
                return None;
            }
            directories += 1;
            if directories > MAX_DIRECTORIES {
                return None;
            }
            let directory = if components.is_empty() {
                None
            } else {
                let path =
                    WorkspacePath::new(workspace.workspace_id.clone(), components.clone()).ok()?;
                Some(
                    adapter
                        .resolve(workspace, &path, PathResolutionIntent::ReadDirectory)
                        .ok()?,
                )
            };
            let names = match &directory {
                None => workspace.observe_root_names(MAX_ENTRIES, 256 * 1024),
                Some(directory) => directory.observe_directory_names(MAX_ENTRIES, 256 * 1024),
            }
            .ok()?;
            for name in names {
                if !available(deadline, cancellation) || entries.len() >= MAX_ENTRIES {
                    return None;
                }
                let mut child = components.clone();
                child.push(name);
                let path =
                    WorkspacePath::new(workspace.workspace_id.clone(), child.clone()).ok()?;
                let object = adapter
                    .resolve(workspace, &path, PathResolutionIntent::Metadata)
                    .ok()?;
                let object = match object.object_kind() {
                    WorkspaceObjectKind::Directory => {
                        pending.push(child.clone());
                        object
                    }
                    WorkspaceObjectKind::RegularFile => {
                        let size = u64::try_from(object.object_snapshot.size).ok()?;
                        bytes = bytes.checked_add(size)?;
                        if bytes > MAX_TOTAL_BYTES {
                            return None;
                        }
                        let file = adapter
                            .resolve(workspace, &path, PathResolutionIntent::ContentHash)
                            .ok()?;
                        // Refuse a replacement between metadata and content reads.
                        if file.object_snapshot != object.object_snapshot {
                            return None;
                        }
                        file.revalidate().ok()?;
                        file
                    }
                };
                let stat = &object.object_snapshot;
                if let Some(sealed) = sealed.as_mut() {
                    if stat.mode & 0o7000 != 0 {
                        return None;
                    }
                    match object.object_kind() {
                        WorkspaceObjectKind::Directory => {
                            sealed.directories.insert(child.clone(), stat.mode & 0o777);
                        }
                        WorkspaceObjectKind::RegularFile => {
                            if sealed.files.len() >= MAX_SEALED_FILES {
                                return None;
                            }
                            let descriptor = crate::sandbox::file_projection(&object).ok()?;
                            sealed.files.push(SealedFile {
                                path: child.clone(),
                                mode: stat.mode & 0o777,
                                descriptor,
                                preimage: object.preimage()?.clone(),
                            });
                        }
                    }
                }
                entries.insert(
                    child,
                    json!({
                        "device":stat.device,"inode":stat.inode,"links":stat.link_count,
                        "mode":stat.mode,"size":stat.size,"mount":stat.mount_id,
                        "mtime":[stat.modified_seconds,stat.modified_nanoseconds],
                        "ctime":[stat.changed_seconds,stat.changed_nanoseconds],
                        "preimage":object.preimage(),
                    }),
                );
            }
            if let Some(directory) = directory {
                directory.revalidate().ok()?;
            }
        }
        // Directory timestamps detect additions/removals, including transient
        // replacements when filesystem timestamp precision records them. File
        // timestamps include ctime, not just caller-set mtime; this is sampled data.
        let root_after = crate::snapshot(&workspace.root_descriptor, None).ok()?;
        if root_before != root_after || !available(deadline, cancellation) {
            return None;
        }
        let material = json!({"version":1,"root":workspace.root_path_sha256(),
            "root_stat":{"device":root_before.device,"inode":root_before.inode,
                "links":root_before.link_count,"mode":root_before.mode,
                "size":root_before.size,"mount":root_before.mount_id,
                "mtime":[root_before.modified_seconds,root_before.modified_nanoseconds],
                "ctime":[root_before.changed_seconds,root_before.changed_nanoseconds]},
            "entries":entries.into_iter().collect::<Vec<_>>()});
        Some(Self {
            root: workspace.root_path_sha256().into(),
            digest: hash(&serde_json::to_vec(&material).ok()?),
            entries: material["entries"].as_array()?.len(),
            bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_kernel_contracts::{AdapterInstanceId, WorkspaceAuthorizationId, WorkspaceId};
    use agentmage_kernel_engine::propagation::CancellationObservationError;
    use rustix::fs::{SealFlags, fcntl_get_seals, ftruncate};
    use rustix::io::{pread, write};
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);
    struct Tree(std::path::PathBuf);
    impl Tree {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-sealed-worktree-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            Self(path)
        }
        fn held(&self) -> LinuxAuthorizedWorkspace {
            crate::authorize_workspace_root(
                &self.0,
                WorkspaceId::from_raw("sealed-worktree"),
                WorkspaceAuthorizationId::from_raw("sealed-worktree-auth"),
                AdapterInstanceId::from_raw("sealed-worktree-adapter"),
            )
            .unwrap()
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    struct Quiet;
    impl EffectCancellationObservation for Quiet {
        fn observe_effect_cancellation(
            &self,
        ) -> Result<
            Option<agentmage_kernel_contracts::CancellationSignal>,
            CancellationObservationError,
        > {
            Ok(None)
        }
    }
    fn seal(tree: &Tree) -> Option<SealedWorktree> {
        SealedWorktree::capture(
            &tree.held(),
            &Quiet,
            Instant::now() + Duration::from_secs(10),
        )
    }
    fn bytes(descriptor: &OwnedFd) -> Vec<u8> {
        let size = rustix::fs::fstat(descriptor).unwrap().st_size as usize;
        let mut bytes = vec![0; size];
        if size > 0 {
            assert_eq!(pread(descriptor, &mut bytes, 0).unwrap(), size);
        }
        bytes
    }

    #[test]
    fn sealed_worktree_admission_requires_exact_bounded_projection_count() {
        use crate::sandbox::supervision::CommandProjections;
        const UNIT: &str = "agentmage-command-0123456789abcdef01234567.service";
        for files in [0, 2, MAX_SEALED_FILES] {
            let tree = Tree::new();
            for index in 0..files {
                fs::write(tree.0.join(format!("file-{index}")), b"source").unwrap();
            }
            let capture = seal(&tree).unwrap();
            let input = capture.input_record();
            let admission = super::super::command_projection_policy(Some(&capture));
            assert!(
                matches!(admission, CommandProjections::Sealed { files: count } if count == files)
            );
            assert_eq!(input.files as usize, files);
            let descriptors = capture.into_descriptors();
            assert_eq!(descriptors.len(), files);
            for actual in [
                0,
                1,
                2,
                3,
                MAX_SEALED_FILES,
                MAX_SEALED_FILES + 1,
                usize::MAX,
            ] {
                assert_eq!(admission.accepts(UNIT, actual), actual == files);
                assert!(
                    !admission.accepts("agentmage-worker-0123456789abcdef01234567.service", actual)
                );
                assert!(!admission.accepts("agentmage-command-*.service", actual));
            }
            assert!(admission.accepts(UNIT, descriptors.len()));
        }
        for files in [MAX_SEALED_FILES + 1, usize::MAX] {
            let admission = CommandProjections::Sealed { files };
            assert!(!admission.accepts(UNIT, files));
            assert!(!admission.accepts(UNIT, 2));
        }
        let live = super::super::command_projection_policy(None);
        assert!(matches!(live, CommandProjections::Live));
        for actual in [0, 1, 2, MAX_SEALED_FILES, MAX_SEALED_FILES + 1] {
            assert_eq!(live.accepts(UNIT, actual), actual <= 1);
        }
    }

    #[test]
    fn sealed_input_digest_binds_guest_contents_names_modes_and_empty_directories() {
        let tree = Tree::new();
        fs::write(tree.0.join("source"), b"first").unwrap();
        fs::set_permissions(tree.0.join("source"), fs::Permissions::from_mode(0o640)).unwrap();
        let snapshot = seal(&tree).unwrap();
        let original = snapshot.input_record();
        assert_eq!(original, seal(&tree).unwrap().input_record());
        assert_eq!(original.files, 1);
        assert_eq!(original.directories, 1);
        assert_eq!(original.file_bytes, 5);
        assert_eq!(
            original.worktree_root_sha256,
            tree.held().root_path_sha256()
        );
        assert_eq!(
            original.observation_sha256,
            WorkspaceObservation::capture(&tree.held()).unwrap().digest
        );
        let mut previous = original.clone();
        for case in 0..5 {
            match case {
                0 => fs::write(tree.0.join("source"), b"other").unwrap(),
                1 => fs::rename(tree.0.join("source"), tree.0.join("renamed")).unwrap(),
                2 => fs::set_permissions(tree.0.join("renamed"), fs::Permissions::from_mode(0o600))
                    .unwrap(),
                3 => fs::create_dir(tree.0.join("empty")).unwrap(),
                4 => fs::write(tree.0.join(".hidden"), b"hidden").unwrap(),
                _ => unreachable!(),
            }
            let changed = seal(&tree).unwrap().input_record();
            assert_ne!(
                previous.projection_sha256, changed.projection_sha256,
                "case {case}"
            );
            previous = changed;
            assert_eq!(snapshot.input_record(), original);
            assert_eq!(bytes(&snapshot.files[0].descriptor), b"first");
        }
        fs::remove_file(tree.0.join(".hidden")).unwrap();
        fs::remove_dir(tree.0.join("empty")).unwrap();
        fs::rename(tree.0.join("renamed"), tree.0.join("source")).unwrap();
        fs::write(tree.0.join("source"), b"first").unwrap();
        fs::set_permissions(tree.0.join("source"), fs::Permissions::from_mode(0o640)).unwrap();
        // Projection identity describes reproduced data, not source inode times.
        assert_eq!(
            seal(&tree).unwrap().input_record().projection_sha256,
            original.projection_sha256
        );
    }

    #[test]
    fn sealed_worktree_preserves_all_bytes_and_modes_after_source_changes() {
        let tree = Tree::new();
        fs::create_dir_all(tree.0.join("tests/empty")).unwrap();
        fs::write(tree.0.join("tests/check.py"), b"assert True\n").unwrap();
        fs::set_permissions(
            tree.0.join("tests/check.py"),
            fs::Permissions::from_mode(0o751),
        )
        .unwrap();
        fs::write(tree.0.join(".git"), b"gitdir: ../metadata/worktrees/task\n").unwrap();
        fs::write(tree.0.join(".settings"), b"\0\xff\n").unwrap();
        fs::write(tree.0.join("empty"), b"").unwrap();
        let snapshot = seal(&tree).unwrap();
        assert_eq!(snapshot.files.len(), 4);
        assert_eq!(snapshot.directories.len(), 2);
        let file = snapshot
            .files
            .iter()
            .find(|f| f.path == ["tests", "check.py"])
            .unwrap();
        assert_eq!(file.mode, 0o751);
        let before = bytes(&file.descriptor);
        fs::write(tree.0.join("tests/check.py"), b"replacement").unwrap();
        fs::remove_file(tree.0.join(".git")).unwrap();
        fs::write(tree.0.join("new"), b"later").unwrap();
        assert_eq!(bytes(&file.descriptor), before);
        assert_eq!(
            snapshot
                .files
                .iter()
                .find(|f| f.path == [".git"])
                .map(|f| bytes(&f.descriptor))
                .unwrap(),
            b"gitdir: ../metadata/worktrees/task\n"
        );
        for file in &snapshot.files {
            assert_eq!(
                fcntl_get_seals(&file.descriptor).unwrap(),
                SealFlags::SEAL | SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK
            );
            assert!(write(&file.descriptor, b"change").is_err());
            assert!(ftruncate(&file.descriptor, bytes(&file.descriptor).len() as u64 + 1).is_err());
            if !bytes(&file.descriptor).is_empty() {
                assert!(ftruncate(&file.descriptor, 0).is_err());
            }
        }
        assert_eq!(snapshot.into_descriptors().len(), 4);
    }

    #[test]
    fn sealed_worktree_mounts_only_indexed_copies_and_remounts_all_directories_read_only() {
        let tree = Tree::new();
        fs::create_dir_all(tree.0.join("a/empty")).unwrap();
        fs::set_permissions(tree.0.join("a"), fs::Permissions::from_mode(0o750)).unwrap();
        fs::write(tree.0.join("a/--options data"), b"one").unwrap();
        fs::write(tree.0.join(".hidden"), b"two").unwrap();
        for path in [".hidden", "a/--options data"] {
            fs::set_permissions(tree.0.join(path), fs::Permissions::from_mode(0o644)).unwrap();
        }
        fs::set_permissions(tree.0.join("a/empty"), fs::Permissions::from_mode(0o755)).unwrap();
        let snapshot = seal(&tree).unwrap();
        let properties = snapshot.open_file_properties(4242);
        assert_eq!(properties.len(), 2);
        for (index, property) in properties.iter().enumerate() {
            assert_eq!(
                *property,
                super::super::open_file_property(
                    4242,
                    snapshot.files[index].descriptor.as_raw_fd(),
                    &format!("source{index}")
                )
            );
        }
        let mut command = Command::new("inert-command");
        snapshot.mount_arguments(&mut command, 5);
        let args = command
            .get_args()
            .map(|a| a.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            args,
            [
                "--tmpfs",
                "/work",
                "--dir",
                "/work/a",
                "--dir",
                "/work/a/empty",
                "--perms",
                "0644",
                "--ro-bind-data",
                "5",
                "/work/.hidden",
                "--perms",
                "0644",
                "--ro-bind-data",
                "6",
                "/work/a/--options data",
                "--chmod",
                "0755",
                "/work/a/empty",
                "--chmod",
                "0750",
                "/work/a",
                "--chmod",
                "0755",
                "/work",
                "--remount-ro",
                "/work"
            ]
        );
        assert!(!args.contains(&"--ro-bind-fd"));
        assert!(!args.iter().any(|a| a.contains(tree.0.to_str().unwrap())));
    }

    #[test]
    fn sealed_worktree_refuses_unsafe_and_incomplete_captures_without_a_prefix() {
        for case in 0..10 {
            let tree = Tree::new();
            fs::write(tree.0.join("source"), b"exact").unwrap();
            match case {
                0 => {
                    symlink("source", tree.0.join("link")).unwrap();
                }
                1 => {
                    fs::hard_link(tree.0.join("source"), tree.0.join("hard")).unwrap();
                }
                2 => {
                    use std::os::unix::ffi::OsStringExt;
                    fs::write(
                        tree.0.join(std::ffi::OsString::from_vec(vec![0xff])),
                        b"bad name",
                    )
                    .unwrap();
                }
                3 => {
                    fs::set_permissions(tree.0.join("source"), fs::Permissions::from_mode(0o4755))
                        .unwrap();
                }
                4 => {
                    let file = fs::File::create(tree.0.join("large")).unwrap();
                    file.set_len(MAX_FILE_BYTES + 1).unwrap();
                }
                5 => {
                    for index in 0..MAX_SEALED_FILES - 1 {
                        fs::write(tree.0.join(format!("file-{index}")), b"").unwrap();
                    }
                }
                6 => {
                    let mut path = tree.0.clone();
                    for _ in 0..=MAX_DEPTH {
                        path.push("dir");
                        fs::create_dir(&path).unwrap();
                    }
                }
                7 => {
                    rustix::fs::mkfifoat(
                        rustix::fs::CWD,
                        tree.0.join("pipe"),
                        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
                    )
                    .unwrap();
                }
                8 => {
                    std::os::unix::net::UnixListener::bind(tree.0.join("socket")).unwrap();
                }
                9 => {
                    fs::write(tree.0.join("unsupported:name"), b"must refuse all").unwrap();
                }
                _ => unreachable!(),
            }
            if case == 5 {
                assert_eq!(seal(&tree).unwrap().files.len(), MAX_SEALED_FILES);
                fs::write(tree.0.join("over-file-budget"), b"").unwrap();
            }
            assert!(seal(&tree).is_none(), "case {case}");
        }
        // Empty directories and all-or-nothing entry counting remain bounded.
        let tree = Tree::new();
        for index in 0..MAX_DIRECTORIES {
            fs::create_dir(tree.0.join(format!("dir-{index}"))).unwrap();
        }
        assert!(seal(&tree).is_none());
    }

    #[test]
    fn sealed_worktree_refuses_expiry_unavailable_cancellation_and_changed_capture() {
        let tree = Tree::new();
        fs::write(tree.0.join("source"), b"first").unwrap();
        let held = tree.held();
        assert!(SealedWorktree::capture(&held, &Quiet, Instant::now()).is_none());
        struct Unavailable;
        impl EffectCancellationObservation for Unavailable {
            fn observe_effect_cancellation(
                &self,
            ) -> Result<
                Option<agentmage_kernel_contracts::CancellationSignal>,
                CancellationObservationError,
            > {
                Err(CancellationObservationError::Unavailable)
            }
        }
        assert!(
            SealedWorktree::capture(
                &held,
                &Unavailable,
                Instant::now() + Duration::from_secs(10)
            )
            .is_none()
        );
        let cancellation = agentmage_kernel_engine::propagation::CancellationToken::root(
            agentmage_kernel_contracts::BoundaryKind::Shell,
            agentmage_kernel_contracts::TaskId::from_raw("sealed-task"),
            agentmage_kernel_contracts::CorrelationId::from_raw("sealed-task"),
        );
        cancellation
            .cancel(agentmage_kernel_contracts::CancellationSignal {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw(
                    "sealed-cancel",
                ),
                task_id: agentmage_kernel_contracts::TaskId::from_raw("sealed-task"),
                correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw("sealed-task"),
                reason: agentmage_kernel_contracts::CancellationReason::UserRequested,
                requested_by: agentmage_kernel_contracts::BoundaryKind::Shell,
            })
            .unwrap();
        assert!(
            SealedWorktree::capture(
                &held,
                &cancellation,
                Instant::now() + Duration::from_secs(10)
            )
            .is_none()
        );
        struct Mutate<'a> {
            path: &'a std::path::Path,
            reads: AtomicUsize,
        }
        impl EffectCancellationObservation for Mutate<'_> {
            fn observe_effect_cancellation(
                &self,
            ) -> Result<
                Option<agentmage_kernel_contracts::CancellationSignal>,
                CancellationObservationError,
            > {
                if self.reads.fetch_add(1, Ordering::SeqCst) == 3 {
                    fs::write(self.path.join("source"), b"second").unwrap();
                }
                Ok(None)
            }
        }
        let probe = Mutate {
            path: &tree.0,
            reads: AtomicUsize::new(0),
        };
        assert!(
            SealedWorktree::capture(&held, &probe, Instant::now() + Duration::from_secs(10))
                .is_none()
        );
        assert!(probe.reads.load(Ordering::SeqCst) >= 4);
    }
}

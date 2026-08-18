//! Held-object Linux projection into the pure repository-map capability.

use std::fmt;
use std::fmt::Write as _;

use agentmage_capability_repository_map::{
    GitTrackedState, RepositoryFileInput, RepositoryMap, RepositoryMapInput, RepositoryObjectKind,
    build_repository_map, verify_repository_map,
};
use agentmage_kernel_contracts::{
    AuthorizedWorkspaceHandle, HeldWorkspaceObject, PathResolutionIntent, WorkspacePath,
};
use agentmage_kernel_engine::platform_startup::VerifiedPlatformAdapter;
use agentmage_platform_linux::{
    LinuxAuthorizedWorkspace, LinuxHeldObject, LinuxPlatformAdapter, LinuxRepositoryInventory,
    LinuxRepositoryInventoryEntry, LinuxRepositoryInventoryState, LinuxRepositoryObjectHint,
    LinuxSymbolicLinkEvidence, linux_git_blob_object_id, observe_linux_workspace_symbolic_link,
    resolve_linux_workspace_object,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_POLICY_RULES: usize = 256;
const MAX_POLICY_REVISION_BYTES: usize = 128;

/// Deterministic path policy applied before repository bytes enter the map capability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LinuxRepositoryMapPolicy {
    revision: String,
    excluded_prefixes: Vec<Vec<String>>,
    generated_prefixes: Vec<Vec<String>>,
    vendored_prefixes: Vec<Vec<String>>,
    policy_sha256: String,
}

impl LinuxRepositoryMapPolicy {
    /// Constructs and seals an exact policy revision and path-prefix rule set.
    pub fn new(
        revision: impl Into<String>,
        mut excluded_prefixes: Vec<Vec<String>>,
        mut generated_prefixes: Vec<Vec<String>>,
        mut vendored_prefixes: Vec<Vec<String>>,
    ) -> Result<Self, LinuxRepositoryMapProjectionError> {
        let revision = revision.into();
        if revision.is_empty()
            || revision.len() > MAX_POLICY_REVISION_BYTES
            || revision.chars().any(char::is_control)
        {
            return Err(LinuxRepositoryMapProjectionError::InvalidPolicy);
        }
        for rules in [
            &mut excluded_prefixes,
            &mut generated_prefixes,
            &mut vendored_prefixes,
        ] {
            rules.sort();
            if rules.len() > MAX_POLICY_RULES
                || rules.windows(2).any(|pair| pair[0] == pair[1])
                || rules.iter().any(|rule| {
                    rule.is_empty()
                        || rule.iter().any(|component| {
                            component.is_empty()
                                || matches!(component.as_str(), "." | "..")
                                || component.contains(['/', '\\'])
                                || component.chars().any(char::is_control)
                        })
                })
            {
                return Err(LinuxRepositoryMapProjectionError::InvalidPolicy);
            }
        }
        let mut policy = Self {
            revision,
            excluded_prefixes,
            generated_prefixes,
            vendored_prefixes,
            policy_sha256: String::new(),
        };
        policy.policy_sha256 = digest(&(
            policy.revision.as_str(),
            &policy.excluded_prefixes,
            &policy.generated_prefixes,
            &policy.vendored_prefixes,
        ))?;
        Ok(policy)
    }

    /// Returns the conservative built-in v0.1 repository projection policy.
    pub fn v0_1() -> Self {
        Self::new(
            "agentmage.repository-map-policy.v0.1",
            vec![vec![".git".to_owned()]],
            [".next", "build", "coverage", "dist", "target"]
                .into_iter()
                .map(|component| vec![component.to_owned()])
                .collect(),
            ["node_modules", "third_party", "vendor"]
                .into_iter()
                .map(|component| vec![component.to_owned()])
                .collect(),
        )
        .expect("the built-in repository policy is static and valid")
    }

    /// Returns the exact sealed policy digest.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    fn classify(&self, path: &[String]) -> (bool, bool, bool) {
        (
            matches_prefix(path, &self.excluded_prefixes),
            matches_prefix(path, &self.generated_prefixes),
            matches_prefix(path, &self.vendored_prefixes),
        )
    }
}

/// Stable content-free failure from Linux repository-map composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxRepositoryMapProjectionError {
    /// The deterministic product policy is malformed.
    InvalidPolicy,
    /// Git inventory evidence or a path cannot be represented safely.
    InvalidInventory,
    /// One current worktree object could not be held and revalidated.
    ObjectUnavailable,
    /// The pure map rejected or failed to verify the projected evidence.
    MapRejected,
}

impl LinuxRepositoryMapProjectionError {
    /// Returns the stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidPolicy => "repository-map.linux.policy-invalid",
            Self::InvalidInventory => "repository-map.linux.inventory-invalid",
            Self::ObjectUnavailable => "repository-map.linux.object-unavailable",
            Self::MapRejected => "repository-map.linux.map-rejected",
        }
    }
}

impl fmt::Display for LinuxRepositoryMapProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for LinuxRepositoryMapProjectionError {}

/// Projects one complete Git inventory through verified, continuously held Linux objects.
pub fn build_linux_repository_map(
    platform: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,
    workspace: &LinuxAuthorizedWorkspace,
    inventory: LinuxRepositoryInventory,
    policy: &LinuxRepositoryMapPolicy,
) -> Result<RepositoryMap, LinuxRepositoryMapProjectionError> {
    project(
        workspace,
        inventory,
        policy,
        |path, intent| resolve_linux_workspace_object(platform, workspace, path, intent),
        |path| observe_linux_workspace_symbolic_link(platform, workspace, path),
    )
}

fn project(
    workspace: &LinuxAuthorizedWorkspace,
    inventory: LinuxRepositoryInventory,
    policy: &LinuxRepositoryMapPolicy,
    mut resolve: impl FnMut(
        &WorkspacePath,
        PathResolutionIntent,
    )
        -> Result<LinuxHeldObject, agentmage_kernel_contracts::PathAdapterError>,
    mut observe_link: impl FnMut(
        &WorkspacePath,
    ) -> Result<
        LinuxSymbolicLinkEvidence,
        agentmage_kernel_contracts::PathAdapterError,
    >,
) -> Result<RepositoryMap, LinuxRepositoryMapProjectionError> {
    if inventory.repository_sha256 != workspace.root_path_sha256() {
        return Err(LinuxRepositoryMapProjectionError::InvalidInventory);
    }
    workspace
        .revalidate()
        .map_err(|_| LinuxRepositoryMapProjectionError::ObjectUnavailable)?;
    let mut files = Vec::with_capacity(inventory.entries.len());
    let mut held_files = Vec::new();
    let mut held_links = Vec::new();
    for entry in &inventory.entries {
        let path = WorkspacePath::new(workspace.workspace_id().clone(), entry.path.clone())
            .map_err(|_| LinuxRepositoryMapProjectionError::InvalidInventory)?;
        let (policy_excluded, generated, vendored) = policy.classify(&entry.path);
        let admit_content = !policy_excluded
            && !generated
            && !vendored
            && !matches!(entry.state, LinuxRepositoryInventoryState::Ignored);

        let file = if entry.object_hint == LinuxRepositoryObjectHint::Gitlink {
            project_gitlink(entry, policy_excluded, generated, vendored)?
        } else if let Ok(held) = resolve(&path, PathResolutionIntent::ContentHash) {
            let preimage = held
                .preimage()
                .ok_or(LinuxRepositoryMapProjectionError::ObjectUnavailable)?;
            let content = admit_content
                .then(|| held.read_exact_bytes())
                .transpose()
                .map_err(|_| LinuxRepositoryMapProjectionError::ObjectUnavailable)?;
            let current_blob = if let Some(bytes) = content.as_deref() {
                linux_git_blob_object_id(&inventory.object_format, bytes)
                    .map_err(|_| LinuxRepositoryMapProjectionError::InvalidInventory)?
            } else {
                let bytes = held
                    .read_exact_bytes()
                    .map_err(|_| LinuxRepositoryMapProjectionError::ObjectUnavailable)?;
                linux_git_blob_object_id(&inventory.object_format, &bytes)
                    .map_err(|_| LinuxRepositoryMapProjectionError::InvalidInventory)?
            };
            let projected = RepositoryFileInput {
                path: entry.path.clone(),
                size_bytes: preimage.byte_len(),
                content_sha256: hex(preimage.content_sha256()),
                content,
                object_kind: RepositoryObjectKind::RegularFile,
                git_state: final_git_state(entry, Some(&current_blob)),
                policy_excluded,
                generated,
                vendored,
            };
            held_files.push(held);
            projected
        } else {
            let link = observe_link(&path)
                .map_err(|_| LinuxRepositoryMapProjectionError::ObjectUnavailable)?;
            let current_blob =
                linux_git_blob_object_id(&inventory.object_format, link.target_bytes())
                    .map_err(|_| LinuxRepositoryMapProjectionError::InvalidInventory)?;
            let projected = RepositoryFileInput {
                path: entry.path.clone(),
                size_bytes: link.size_bytes,
                content_sha256: link.target_sha256.clone(),
                content: None,
                object_kind: RepositoryObjectKind::SymbolicLink,
                git_state: final_git_state(entry, Some(&current_blob)),
                policy_excluded,
                generated,
                vendored,
            };
            held_links.push(link);
            projected
        };
        files.push(file);
    }

    let worktree_sha256 = worktree_digest(&files)?;
    let freshness_sha256 = digest(&(
        inventory.git_evidence_sha256.as_str(),
        worktree_sha256.as_str(),
        policy.policy_sha256(),
    ))?;
    let map = build_repository_map(RepositoryMapInput {
        workspace_id: workspace.workspace_id().clone(),
        repository_sha256: inventory.repository_sha256,
        worktree_sha256,
        branch: inventory.branch,
        commit_id: inventory.commit_id,
        policy_sha256: policy.policy_sha256().to_owned(),
        freshness_sha256,
        files,
    })
    .map_err(|_| LinuxRepositoryMapProjectionError::MapRejected)?;

    for held in &held_files {
        held.revalidate()
            .map_err(|_| LinuxRepositoryMapProjectionError::ObjectUnavailable)?;
    }
    for held in &held_links {
        held.revalidate()
            .map_err(|_| LinuxRepositoryMapProjectionError::ObjectUnavailable)?;
    }
    workspace
        .revalidate()
        .map_err(|_| LinuxRepositoryMapProjectionError::ObjectUnavailable)?;
    verify_repository_map(&map)
        .then_some(map)
        .ok_or(LinuxRepositoryMapProjectionError::MapRejected)
}

fn project_gitlink(
    entry: &LinuxRepositoryInventoryEntry,
    policy_excluded: bool,
    generated: bool,
    vendored: bool,
) -> Result<RepositoryFileInput, LinuxRepositoryMapProjectionError> {
    let LinuxRepositoryInventoryState::Tracked {
        index_object_id,
        staged_changed,
        conflicted,
        ..
    } = &entry.state
    else {
        return Err(LinuxRepositoryMapProjectionError::InvalidInventory);
    };
    Ok(RepositoryFileInput {
        path: entry.path.clone(),
        size_bytes: 0,
        content_sha256: sha256(index_object_id.as_bytes()),
        content: None,
        object_kind: RepositoryObjectKind::Gitlink,
        git_state: if *staged_changed || *conflicted {
            GitTrackedState::TrackedChanged
        } else {
            GitTrackedState::TrackedClean
        },
        policy_excluded,
        generated,
        vendored,
    })
}

fn final_git_state(
    entry: &LinuxRepositoryInventoryEntry,
    current_blob: Option<&str>,
) -> GitTrackedState {
    match &entry.state {
        LinuxRepositoryInventoryState::Tracked {
            index_object_id,
            staged_changed,
            conflicted,
            ..
        } if *staged_changed || *conflicted || current_blob != Some(index_object_id.as_str()) => {
            GitTrackedState::TrackedChanged
        }
        LinuxRepositoryInventoryState::Tracked { .. } => GitTrackedState::TrackedClean,
        LinuxRepositoryInventoryState::Untracked => GitTrackedState::Untracked,
        LinuxRepositoryInventoryState::Ignored => GitTrackedState::Ignored,
    }
}

fn matches_prefix(path: &[String], prefixes: &[Vec<String>]) -> bool {
    prefixes.iter().any(|prefix| path.starts_with(prefix))
}

fn worktree_digest(
    files: &[RepositoryFileInput],
) -> Result<String, LinuxRepositoryMapProjectionError> {
    let material = files
        .iter()
        .map(|file| {
            (
                &file.path,
                file.size_bytes,
                &file.content_sha256,
                file.object_kind,
                file.git_state,
                file.policy_excluded,
                file.generated,
                file.vendored,
            )
        })
        .collect::<Vec<_>>();
    digest(&material)
}

fn digest(value: &impl Serialize) -> Result<String, LinuxRepositoryMapProjectionError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| LinuxRepositoryMapProjectionError::InvalidPolicy)
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
pub(crate) fn build_test_linux_repository_map(
    adapter_instance_id: agentmage_kernel_contracts::AdapterInstanceId,
    workspace: &LinuxAuthorizedWorkspace,
    inventory: LinuxRepositoryInventory,
    policy: &LinuxRepositoryMapPolicy,
) -> Result<RepositoryMap, LinuxRepositoryMapProjectionError> {
    project(
        workspace,
        inventory,
        policy,
        |path, intent| {
            agentmage_platform_linux::resolve_test_linux_workspace_object(
                workspace,
                adapter_instance_id.clone(),
                path,
                intent,
            )
        },
        |path| {
            agentmage_platform_linux::observe_test_linux_workspace_symbolic_link(
                workspace,
                adapter_instance_id.clone(),
                path,
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_capability_repository_map::{
        GitTrackedState, RepositoryEntryDisposition, RepositoryObjectKind,
    };
    use agentmage_kernel_contracts::{AdapterInstanceId, WorkspaceAuthorizationId, WorkspaceId};
    use agentmage_platform_linux::{
        LinuxGitArtifact, LinuxRepositoryCollector, LinuxRepositoryScope,
        select_test_linux_workspace,
    };

    use super::{LinuxRepositoryMapPolicy, build_test_linux_repository_map};

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
        repository: PathBuf,
        owned: PathBuf,
        filter_canary: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "agentmage-live-map-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            let repository = root.join("repository");
            let owned = root.join("owned");
            fs::create_dir_all(&repository).expect("repository creates");
            fs::create_dir_all(&owned).expect("owned root creates");
            fs::set_permissions(&repository, fs::Permissions::from_mode(0o700))
                .expect("repository permissions");
            fs::set_permissions(&owned, fs::Permissions::from_mode(0o700))
                .expect("owned permissions");
            git(&repository, &["init", "--initial-branch=main"]);
            fs::create_dir(repository.join("src")).expect("source directory");
            fs::write(repository.join("README.md"), "# Fixture\n").expect("README writes");
            fs::write(repository.join("src/changed.rs"), "fn before() {}\n")
                .expect("source writes");
            fs::write(repository.join(".gitignore"), "ignored.log\n").expect("ignore rule writes");
            fs::write(repository.join(".gitattributes"), "*.rs filter=hostile\n")
                .expect("attributes write");
            symlink("README.md", repository.join("readme-link")).expect("link writes");
            git(
                &repository,
                &[
                    "add",
                    "README.md",
                    "src/changed.rs",
                    ".gitignore",
                    ".gitattributes",
                    "readme-link",
                ],
            );
            let head = text(git(&repository, &["rev-parse", ":README.md"]));
            git(
                &repository,
                &[
                    "update-index",
                    "--add",
                    "--cacheinfo",
                    &format!("160000,{head},vendor/dependency"),
                ],
            );
            git(
                &repository,
                &[
                    "-c",
                    "user.name=AgentMage Fixture",
                    "-c",
                    "user.email=fixture@example.test",
                    "commit",
                    "-m",
                    "fixture",
                ],
            );
            fs::write(repository.join("src/changed.rs"), "fn after() {}\n")
                .expect("source modifies");
            fs::create_dir(repository.join("target")).expect("generated directory");
            fs::write(
                repository.join("target/generated.rs"),
                "fn generated() {}\n",
            )
            .expect("generated writes");
            fs::create_dir_all(repository.join("vendor/local")).expect("vendor directory");
            fs::write(
                repository.join("vendor/local/lib.py"),
                "def vendored(): pass\n",
            )
            .expect("vendor writes");
            fs::write(repository.join("untracked.py"), "def current(): pass\n")
                .expect("untracked writes");
            fs::write(repository.join("ignored.log"), "private ignored bytes\n")
                .expect("ignored writes");
            let filter_canary = root.join("hostile-filter-executed");
            git(
                &repository,
                &[
                    "config",
                    "filter.hostile.clean",
                    &format!("touch {}", filter_canary.display()),
                ],
            );
            git(
                &repository,
                &[
                    "config",
                    "filter.hostile.smudge",
                    &format!("touch {}", filter_canary.display()),
                ],
            );
            Self {
                root,
                repository,
                owned,
                filter_canary,
            }
        }

        fn scope(&self) -> LinuxRepositoryScope {
            LinuxRepositoryScope::verify(
                &self.repository,
                self.repository.join(".git"),
                &self.owned,
            )
            .expect("repository scope")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn git(repository: &Path, arguments: &[&str]) -> Vec<u8> {
        let output = Command::new("/usr/bin/git")
            .env_clear()
            .env("HOME", "/nonexistent")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args(arguments)
            .current_dir(repository)
            .output()
            .expect("fixture Git runs");
        assert!(output.status.success(), "fixture Git failed: {arguments:?}");
        output.stdout
    }

    fn text(bytes: Vec<u8>) -> String {
        String::from_utf8(bytes)
            .expect("Git text")
            .trim()
            .to_owned()
    }

    #[test]
    fn live_projection_is_git_aware_policy_aware_and_non_following() {
        let fixture = Fixture::new();
        let adapter_id = AdapterInstanceId::from_raw("adapter-live-map-0001");
        let workspace = select_test_linux_workspace(
            &fixture.repository,
            WorkspaceId::from_raw("workspace-live-map-0001"),
            WorkspaceAuthorizationId::from_raw("authorization-live-map-0001"),
            adapter_id.clone(),
        )
        .expect("workspace selects");
        let collector = LinuxRepositoryCollector::new(
            LinuxGitArtifact::verify("/usr/bin/git").expect("Git verifies"),
        );
        let inventory = collector
            .collect_inventory(&fixture.scope())
            .expect("Git inventory collects");
        let inventory_before = inventory.clone();
        let mut foreign_inventory = inventory.clone();
        foreign_inventory.repository_sha256 = "a".repeat(64);
        assert!(
            build_test_linux_repository_map(
                adapter_id.clone(),
                &workspace,
                foreign_inventory,
                &LinuxRepositoryMapPolicy::v0_1(),
            )
            .is_err()
        );
        let map = build_test_linux_repository_map(
            adapter_id,
            &workspace,
            inventory,
            &LinuxRepositoryMapPolicy::v0_1(),
        )
        .expect("live map builds");

        let file = |path: &str| {
            map.files
                .iter()
                .find(|file| {
                    file.path
                        .components()
                        .iter()
                        .map(|component| component.as_str())
                        .collect::<Vec<_>>()
                        .join("/")
                        == path
                })
                .expect("mapped path")
        };
        assert_eq!(file("README.md").git_state, GitTrackedState::TrackedClean);
        assert_eq!(
            file("src/changed.rs").git_state,
            GitTrackedState::TrackedChanged
        );
        assert_eq!(
            file("ignored.log").disposition,
            RepositoryEntryDisposition::GitIgnored
        );
        assert_eq!(
            file("target/generated.rs").disposition,
            RepositoryEntryDisposition::GeneratedExcluded
        );
        assert_eq!(
            file("vendor/local/lib.py").disposition,
            RepositoryEntryDisposition::VendoredExcluded
        );
        assert_eq!(
            file("readme-link").object_kind,
            RepositoryObjectKind::SymbolicLink
        );
        assert_eq!(
            file("vendor/dependency").object_kind,
            RepositoryObjectKind::Gitlink
        );
        assert_eq!(file("untracked.py").git_state, GitTrackedState::Untracked);
        assert!(file("untracked.py").content_read);
        assert_eq!(map.coverage.discovered_files, 10);
        assert_eq!(map.coverage.excluded_files, 4);
        assert!(!fixture.filter_canary.exists());
        assert_eq!(
            collector
                .collect_inventory(&fixture.scope())
                .expect("post-map inventory"),
            inventory_before
        );
    }

    #[test]
    fn malformed_or_ambiguous_policy_rules_fail_closed() {
        assert!(LinuxRepositoryMapPolicy::new("", Vec::new(), Vec::new(), Vec::new()).is_err());
        assert!(
            LinuxRepositoryMapPolicy::new(
                "policy-v1",
                vec![vec!["..".to_owned()]],
                Vec::new(),
                Vec::new(),
            )
            .is_err()
        );
        assert!(
            LinuxRepositoryMapPolicy::new(
                "policy-v1",
                vec![vec!["private".to_owned()], vec!["private".to_owned()]],
                Vec::new(),
                Vec::new(),
            )
            .is_err()
        );
    }
}

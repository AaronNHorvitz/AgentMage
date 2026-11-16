// SPDX-License-Identifier: MIT OR Apache-2.0
//! Real descriptor lifetime tests with synthetic profile/map provenance only.

use super::*;
use crate::coding_session::tests::input_with_worktree_path_sha256;
use agentmage_capability_read_only::{
    ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyRequest, ReadOnlyToolKind,
};
use agentmage_capability_repository_map::{
    GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_repository_map,
};
use agentmage_kernel_contracts::{
    ActionId, CONTRACT_SCHEMA_VERSION, ContractPayload, CorrelationId, ToolCall, ToolCallId, ToolId,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const SOURCE: &[u8] = b"pub fn runtime_fixture() {}\n";

type Foundation = (
    Arc<LinuxDevelopmentPlatformAdapter>,
    Arc<CodingSessionProfile>,
    RepositoryMap,
);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!(
            "agentmage-binding-lifetime-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(path.join("src")).unwrap();
        fs::write(path.join("src/lib.rs"), SOURCE).unwrap();
        Self(path)
    }

    fn foundation(&self) -> Foundation {
        let platform = Arc::new(
            LinuxDevelopmentPlatformAdapter::activate(
                "coding-development-v1",
                "b".repeat(64),
                AdapterInstanceId::from_raw("lifetime-adapter"),
            )
            .unwrap(),
        );
        let identity = linux_repository_path_sha256(&self.0).unwrap();
        let mut input = input_with_worktree_path_sha256(identity.clone());
        let map = build_repository_map(RepositoryMapInput {
            workspace_id: input.write_scope.workspace_id().clone(),
            repository_sha256: "a".repeat(64),
            worktree_sha256: identity,
            branch: Some(input.worktree.branch_ref.clone()),
            commit_id: input.immutable_base_commit.clone(),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![RepositoryFileInput {
                path: vec!["src".into(), "lib.rs".into()],
                size_bytes: SOURCE.len() as u64,
                content_sha256: sha256_hex(SOURCE),
                content: Some(SOURCE.to_vec()),
                object_kind: agentmage_capability_repository_map::RepositoryObjectKind::RegularFile,
                git_state: GitTrackedState::TrackedClean,
                policy_excluded: false,
                generated: false,
                vendored: false,
            }],
        })
        .unwrap();
        input.repository_snapshot_sha256 = map.map_sha256.clone();
        input.change_plan =
            crate::coding_plan::build_coding_development_plan_binding(&map).unwrap();
        let profile = Arc::new(CodingSessionProfile::build(input).unwrap());
        (platform, profile, map)
    }

    fn descriptors(&self) -> usize {
        // Restrict the observation to this test's unique tree. Other tests can
        // open/close arbitrary descriptors without affecting the assertion.
        fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(|entry| fs::read_link(entry.ok()?.path()).ok())
            .filter(|target| target.starts_with(&self.0))
            .count()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn bind(fixture: &Fixture, foundation: &Foundation) -> Arc<LinuxCodingWorkspace<'static, 'static>> {
    Arc::new(
        LinuxCodingWorkspace::bind_owned_development(
            Arc::clone(&foundation.0),
            Arc::clone(&foundation.1),
            foundation.2.clone(),
            &fixture.0,
            WorkspaceAuthorizationId::from_raw("lifetime-workspace"),
        )
        .unwrap(),
    )
}

fn read_call(profile: &CodingSessionProfile) -> ToolCall {
    let definition = profile
        .registry()
        .get_tool(&ToolId::from_raw(ReadOnlyToolKind::ReadText.id()), "1.0.0")
        .unwrap();
    let bytes = serde_json::to_vec(&ReadOnlyRequest {
        schema_version: 1,
        paths: vec![vec!["src".into(), "lib.rs".into()]],
        query: None,
        byte_offset: Some(0),
        byte_count: Some(128),
        encoding: ReadOnlyEncoding::Utf8,
        limits: ReadOnlyLimits::default(),
        call_depth: 0,
    })
    .unwrap();
    ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw("lifetime-read"),
        correlation_id: CorrelationId::from_raw("lifetime-correlation"),
        action_id: ActionId::from_raw("lifetime-action"),
        tool_id: definition.tool_id.clone(),
        tool_version: definition.tool_version.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".into(),
            sha256: sha256_hex(&bytes),
            bytes,
        },
    }
}

#[test]
fn owned_binding_matches_borrowed_plans_and_retains_its_exact_root_until_last_operation() {
    let fixture = Fixture::new();
    let foundation = fixture.foundation();
    let weak_platform = Arc::downgrade(&foundation.0);
    let weak_profile = Arc::downgrade(&foundation.1);
    let owned = bind(&fixture, &foundation);
    let weak_root = Arc::downgrade(&owned.workspace);
    let call = read_call(&foundation.1);
    let borrowed = LinuxCodingWorkspace::bind_development(
        &foundation.0,
        &foundation.1,
        foundation.2.clone(),
        &fixture.0,
        WorkspaceAuthorizationId::from_raw("lifetime-workspace"),
    )
    .unwrap();
    let original = borrowed.prepare(&call).unwrap();
    let retained = owned.prepare(&call).unwrap();
    assert_eq!(
        original.operation().plan_sha256(),
        retained.operation().plan_sha256()
    );
    drop(original);
    drop(borrowed);
    drop(foundation);
    assert!(weak_profile.upgrade().is_some());
    assert!(weak_platform.upgrade().is_some());
    drop(owned);
    assert!(weak_profile.upgrade().is_none());
    assert!(weak_platform.upgrade().is_none());
    retained.revalidate().unwrap();
    assert!(weak_root.upgrade().is_some());
    assert!(fixture.descriptors() > 0);
    drop(retained);
    assert!(weak_root.upgrade().is_none());
    assert_eq!(fixture.descriptors(), 0);
    assert_eq!(fs::read(fixture.0.join("src/lib.rs")).unwrap(), SOURCE);
}

#[test]
fn refreshed_binding_releases_old_foundation_but_pending_operations_still_refuse_drift() {
    let fixture = Fixture::new();
    let first_foundation = fixture.foundation();
    let mut current = bind(&fixture, &first_foundation);
    let old_binding = Arc::downgrade(&current);
    let old_profile = Arc::downgrade(&first_foundation.1);
    let old_root = Arc::downgrade(&current.workspace);
    let retained = current.prepare(&read_call(&first_foundation.1)).unwrap();
    let second = fixture.foundation();
    current = bind(&fixture, &second);
    drop(first_foundation);
    assert!(old_binding.upgrade().is_none());
    assert!(old_profile.upgrade().is_none());
    assert!(old_root.upgrade().is_some());
    retained.revalidate().unwrap();
    fs::write(fixture.0.join("src/lib.rs"), b"pub fn changed() {}\n").unwrap();
    assert_eq!(
        retained.revalidate(),
        Err(LinuxCodingBindingError::TargetDenied)
    );
    // A fresh read may inspect changed bytes within the same path scope. It
    // does not reuse the stale operation or authorize a write of those bytes.
    let fresh = current.prepare(&read_call(&second.1)).unwrap();
    fresh.revalidate().unwrap();
    assert_ne!(retained.binding().target(0), fresh.binding().target(0));
    drop(fresh);
    drop(retained);
    assert!(old_root.upgrade().is_none());
    drop(current);
    drop(second);
    assert_eq!(fixture.descriptors(), 0);
}

#[test]
fn shared_runtime_handle_survives_factory_drop_and_releases_descriptors_after_worker_join() {
    let fixture = Fixture::new();
    let foundation = fixture.foundation();
    let binding = bind(&fixture, &foundation);
    let weak_binding = Arc::downgrade(&binding);
    let weak_profile = Arc::downgrade(&foundation.1);
    let weak_platform = Arc::downgrade(&foundation.0);
    let call = read_call(&foundation.1);
    let handle: LinuxCodingWorkspaceHandle<'static, 'static, 'static> = Arc::clone(&binding).into();
    drop(binding);
    drop(foundation);
    let worker = std::thread::spawn(move || {
        let pending = handle.prepare(&call).unwrap();
        pending.revalidate().unwrap();
        drop(handle);
        pending
    });
    let pending = worker.join().unwrap();
    assert!(weak_binding.upgrade().is_none());
    assert!(weak_profile.upgrade().is_none());
    assert!(weak_platform.upgrade().is_none());
    pending.revalidate().unwrap();
    drop(pending);
    assert_eq!(fixture.descriptors(), 0);
}

#[test]
fn repeated_owned_bind_refresh_and_refusal_cycles_leave_no_descriptors_or_foundations() {
    let fixture = Fixture::new();
    for _ in 0..64 {
        let foundation = fixture.foundation();
        let weak_profile = Arc::downgrade(&foundation.1);
        let weak_platform = Arc::downgrade(&foundation.0);
        let binding = bind(&fixture, &foundation);
        let retained = binding.prepare(&read_call(&foundation.1)).unwrap();
        // Fail only the held-path binding: the profile and map themselves are valid.
        assert!(
            LinuxCodingWorkspace::bind_owned_development(
                Arc::clone(&foundation.0),
                Arc::clone(&foundation.1),
                foundation.2.clone(),
                &fixture.0.join("src"),
                WorkspaceAuthorizationId::from_raw("wrong-root")
            )
            .is_err()
        );
        drop(binding);
        drop(foundation);
        retained.revalidate().unwrap();
        drop(retained);
        assert!(weak_profile.upgrade().is_none());
        assert!(weak_platform.upgrade().is_none());
        assert_eq!(fixture.descriptors(), 0);
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    use std::fmt::Write;
    let mut result = String::with_capacity(64);
    for byte in sha2::Sha256::digest(bytes) {
        write!(&mut result, "{byte:02x}").unwrap();
    }
    result
}

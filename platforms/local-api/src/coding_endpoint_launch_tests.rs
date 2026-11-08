// SPDX-License-Identifier: Apache-2.0
//! Private configuration refusal and production preparation components.
use super::*;
#[path = "../tests/support/endpoint_launch.rs"]
mod fixture;
use fixture::{Fixture, private_file};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

#[test]
fn launch_configuration_refuses_links_modes_devices_aliases_and_oversize() {
    let f = Fixture::new();
    let path = f.write();
    assert!(load(&path).is_ok());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(load(&path).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let alias = f.root.join("alias");
    symlink(&path, &alias).unwrap();
    assert!(load(&alias).is_err());
    fs::remove_file(&alias).unwrap();
    fs::hard_link(&path, &alias).unwrap();
    assert!(load(&path).is_err());
    fs::remove_file(&alias).unwrap();
    assert!(private_bytes(&path, 1).is_err());
    assert!(private_bytes(&f.root, MAX_LAUNCH_BYTES).is_err());
    assert!(private_bytes(Path::new("/dev/null"), 4096).is_err());
    assert!(private_bytes(Path::new("relative"), 4096).is_err());
    symlink(&f.root, f.root.join("parent-alias")).unwrap();
    assert!(load(&f.root.join("parent-alias/launch.json")).is_err());
    private_file(&path, b"");
    assert!(load(&path).is_err());
    private_file(&path, b"{\"inline_secret\":\"synthetic\"}");
    assert!(load(&path).is_err());
}

#[test]
fn launch_schema_and_original_seal_refuse_unknown_stale_and_unsupported_input() {
    let mut f = Fixture::new();
    let path = f.write();
    let array = serde_json::to_vec(&json!([
        f.launch["version"],
        f.launch["state_root"],
        f.launch["disposable_root"],
        f.launch["workspace_root"],
        f.launch["worker"],
        f.launch["credential_file"],
        f.launch["request"],
        f.launch["instructions"],
        f.launch["checks"]
    ]))
    .unwrap();
    assert!(serde_json::from_slice::<Launch>(&array).is_ok());
    private_file(&path, &array);
    assert!(load(&path).is_err());
    let check = f.launch["checks"][0].clone();
    f.launch["checks"][0] = json!([["sentinel"], 17, "0".repeat(64)]);
    assert!(serde_json::from_value::<Launch>(f.launch.clone()).is_ok());
    assert!(load(&f.write()).is_err());
    f.launch["checks"][0] = check;
    f.launch["unknown"] = json!(true);
    assert!(load(&f.write()).is_err());
    f.launch.as_object_mut().unwrap().remove("unknown");
    f.launch["checks"][0]["unknown"] = json!(true);
    assert!(load(&f.write()).is_err());
    f.launch["checks"][0]
        .as_object_mut()
        .unwrap()
        .remove("unknown");
    f.launch["version"] = json!(2);
    assert!(load(&f.write()).is_err());
    f.launch["version"] = json!(1);
    f.launch["request"]["request_sha256"] = json!("0".repeat(64));
    assert!(load(&f.write()).is_err());
}

#[test]
fn launch_rebinds_fresh_service_identities_then_uses_original_preparation_and_verifier() {
    let f = Fixture::new();
    let launch = load(&f.write()).unwrap();
    let original = launch.request;
    let request = bind_identities(
        original.clone(),
        RuntimeRunId::from_raw("fresh-run"),
        SessionId::from_raw("fresh-session"),
    )
    .unwrap();
    let mut expected = original.clone();
    expected.run_id = request.run_id.clone();
    expected.session_id = request.session_id.clone();
    expected.task.task_id = request.task.task_id.clone();
    expected.task.session_id = request.task.session_id.clone();
    expected.work_packet.task_id = request.work_packet.task_id.clone();
    expected.work_packet.work_packet_id = request.work_packet.work_packet_id.clone();
    assert_eq!(
        request,
        seal_runtime_endpoint_run_request(expected).unwrap()
    );
    assert_ne!(request.request_sha256, original.request_sha256);
    let activation = CodingDevelopmentActivation::validate(
        &launch.state_root,
        &launch.disposable_root,
        &launch.workspace_root,
    )
    .unwrap();
    let platform = LinuxDevelopmentPlatformAdapter::activate(
        CODING_DEVELOPMENT_ACTIVATION,
        activation.marker_sha256().into(),
        AdapterInstanceId::from_raw("synthetic-launch-adapter"),
    )
    .unwrap();
    let workspace = Arc::new(
        select_development_linux_workspace(
            &platform,
            &launch.workspace_root,
            request.workspace_id.clone(),
            WorkspaceAuthorizationId::from_raw("synthetic-launch-auth"),
        )
        .unwrap(),
    );
    let prepared = CodingEndpointStartup::prepare(
        request,
        workspace,
        ActorId::from_raw("synthetic-launch-actor"),
        &launch.instructions,
    )
    .unwrap();
    let check = &launch.checks[0];
    let check = CodingEndpointReadCheck::new(
        WorkspacePath::new(prepared.request().workspace_id.clone(), check.path.clone()).unwrap(),
        check.byte_len,
        check.content_sha256.clone(),
    )
    .unwrap();
    assert!(prepared.read_verifier(vec![check]).is_ok());
    assert!(
        agentmage_host::coding_endpoint_context::CodingEndpointContext::new(
            prepared.request(),
            &agentmage_host::runtime_tools::workspace_file_read_runtime_registry().unwrap(),
            &[],
            2048
        )
        .is_ok()
    );
    assert_eq!(fs::read_dir(launch.state_root).unwrap().count(), 0);
    assert_eq!(
        fs::read(launch.workspace_root.join("sentinel")).unwrap(),
        b"synthetic source\n"
    );
    let mut corrupt = original;
    corrupt.task.objective.push_str(" modified");
    assert!(
        bind_identities(
            corrupt,
            RuntimeRunId::from_raw("next-run"),
            SessionId::from_raw("next-session")
        )
        .is_err()
    );
}

#[test]
fn launch_refuses_actual_native_admission_before_credential_read_or_state_creation() {
    let f = Fixture::new();
    let launch = load(&f.write()).unwrap();
    let mut runs = Runs::default();
    assert_eq!(
        admit(
            launch,
            RuntimeRunId::from_raw("refused-run"),
            SessionId::from_raw("refused-session"),
            &mut runs
        )
        .err(),
        Some(ApiError::Configuration)
    );
    assert_eq!(fs::read_dir(f.root.join("state")).unwrap().count(), 0);
    assert_eq!(
        fs::read(f.root.join("disposable/workspace/sentinel")).unwrap(),
        b"synthetic source\n"
    );
}

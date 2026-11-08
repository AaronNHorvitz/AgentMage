// SPDX-License-Identifier: Apache-2.0
//! Synthetic startup metadata; no model, admitted worker or coding score.

use agentmage_host::{
    coding_development_activation::coding_development_marker,
    coding_endpoint_read_verifier::CodingEndpointReadCheck,
    runtime_tools::workspace_file_read_runtime_registry,
};
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    instruction_provenance::build_instruction_ledger,
    runtime_coordinator::{runtime_tool_catalog_sha256, seal_runtime_endpoint_run_request},
    runtime_loop::runtime_tool_references,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

pub struct Fixture {
    pub root: PathBuf,
    pub launch: Value,
}
impl Fixture {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "endpoint-launch-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for relative in [
            "",
            "state",
            "disposable",
            "disposable/workspace",
            "disposable/workspace/.git",
        ] {
            let path = root.join(relative);
            fs::create_dir(&path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let workspace = root.join("disposable/workspace");
        fs::write(workspace.join(".git/HEAD"), b"ref: refs/heads/main\n").unwrap();
        fs::write(workspace.join("sentinel"), b"synthetic source\n").unwrap();
        let marker = coding_development_marker(&workspace).unwrap();
        private_file(&workspace.join(".agentmage-development-workspace"), &marker);
        // This user-owned non-executable file MUST fail the original native
        // artifact admission. No script or substitute worker may execute.
        let worker = root.join("agentmage-read-only-worker");
        private_file(&worker, b"synthetic refused worker\n");
        let request = request();
        let instructions = build_instruction_ledger(
            request.workspace_snapshot_sha256.clone(),
            vec![],
            vec![],
            vec![],
            &BTreeMap::new(),
        )
        .unwrap();
        let launch = json!({
            "version":1, "state_root":root.join("state"),
            "disposable_root":root.join("disposable"), "workspace_root":workspace,
            "worker":worker, "credential_file":null,
            "request":request, "instructions":instructions,
            "checks":[{"path":["sentinel"],"byte_len":17,"content_sha256":digest()}]
        });
        Self { root, launch }
    }
    pub fn write(&self) -> PathBuf {
        let path = self.root.join("launch.json");
        private_file(&path, &serde_json::to_vec(&self.launch).unwrap());
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
pub fn private_file(path: &std::path::Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn digest() -> String {
    Sha256::digest(b"synthetic source\n")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn request() -> RuntimeEndpointRunRequest {
    // Existing first-party fixture supplies only common task contract fields.
    // Strip the native profile. No profile enters endpoint startup or admission.
    let mut value: Value = serde_json::from_slice(include_bytes!(
        "../../../../fixtures/runtime-producer/v5/run-request.json"
    ))
    .unwrap();
    value.as_object_mut().unwrap().remove("model_profile");
    value["mode"] = json!("ephemeral_read_only");
    value["event_cursor"] = Value::Null;
    value["model_selection"] =
        json!({"endpoint":"http://127.0.0.1:1234/v1","model":"synthetic-model"});
    value["max_output_tokens"] = json!(128);
    value["context_budget"] = json!({"max_messages":16,"max_content_bytes":32768,"max_serialized_messages_bytes":65536,"max_request_body_bytes":131072});
    value["task"]["constraints"] =
        json!(["Synthetic read-only startup; no source disclosure consent"]);
    value["work_packet"]["mutable_files"] = json!([]);
    value["work_packet"]["required_evidence"] = json!([EvidenceKind::ToolOutput]);
    value["work_packet"]["required_capability_class"] = json!(AuthorityClass::Observe);
    let workspace: WorkspaceId = serde_json::from_value(value["workspace_id"].clone()).unwrap();
    let check = CodingEndpointReadCheck::new(
        WorkspacePath::new(workspace, ["sentinel"]).unwrap(),
        17,
        digest(),
    )
    .unwrap();
    value["task"]["acceptance_criteria"] = json!([check.criterion()]);
    value["work_packet"]["acceptance_checks"] = json!([check.criterion()]);
    let registry = workspace_file_read_runtime_registry().unwrap();
    let catalog = ToolCatalogId::from_raw("synthetic-launch-file-catalog");
    let visible = runtime_tool_references(&registry).unwrap();
    value["tool_catalog_id"] = json!(catalog);
    value["tool_catalog_sha256"] = json!(runtime_tool_catalog_sha256(&catalog, &visible).unwrap());
    value["visible_tools"] = json!(visible);
    seal_runtime_endpoint_run_request(serde_json::from_value(value).unwrap()).unwrap()
}

// SPDX-License-Identifier: Apache-2.0
//! Synthetic project composition tests; no native execution or model qualification.
use super::*;
use agentmage_capability_repository_map::{
    GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_repository_map,
};

fn project(names: &[&str]) -> RepositoryMap {
    build_repository_map(RepositoryMapInput {
        workspace_id: WorkspaceId::from_raw("workspace-coding"),
        repository_sha256: "a".repeat(64),
        worktree_sha256: "2".repeat(64),
        branch: Some("refs/heads/agentmage/tasks/task-coding-0001".into()),
        commit_id: "1".repeat(40),
        policy_sha256: "d".repeat(64),
        freshness_sha256: "e".repeat(64),
        files: names
            .iter()
            .map(|name| {
                let content = b"def value():\n    return 1\n";
                RepositoryFileInput {
                    path: name.split('/').map(str::to_owned).collect(),
                    size_bytes: content.len() as u64,
                    content_sha256: sha256(content),
                    content: Some(content.to_vec()),
                    object_kind: RepositoryObjectKind::RegularFile,
                    git_state: GitTrackedState::TrackedClean,
                    policy_excluded: false,
                    generated: false,
                    vendored: false,
                }
            })
            .collect(),
    })
    .unwrap()
}

#[test]
fn python_project_selects_exact_sources_and_refuses_empty_or_excess_scope() {
    let map = project(&[
        "src/z.py",
        "tests/test_z.py",
        "src/nested/a.py",
        "setup.py",
        "src/note.md",
    ]);
    let paths = python_project_paths(&map).unwrap();
    let names: Vec<_> = paths
        .iter()
        .map(|p| {
            p.components()
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    assert_eq!(names, ["src/nested/a.py", "src/z.py"]);
    let plan = build_coding_project_plan_binding(&map, &paths).unwrap();
    assert!(plan.verify());
    for path in &paths {
        assert!(
            serde_json::to_value(&plan).unwrap()["proposed_paths"]
                .as_array()
                .unwrap()
                .contains(&serde_json::to_value(path).unwrap())
        );
    }
    assert!(python_project_paths(&project(&["tests/test_z.py", "setup.py"])).is_err());
    let names: Vec<_> = (0..17).map(|i| format!("src/f{i}.py")).collect();
    assert!(
        python_project_paths(&project(
            &names.iter().map(String::as_str).collect::<Vec<_>>()
        ))
        .is_err()
    );
    let mut denied = map.clone();
    for file in &mut denied.files {
        file.content_read = false;
    }
    assert!(python_project_paths(&denied).is_err());
    let mut denied = map.clone();
    for file in &mut denied.files {
        file.disposition =
            agentmage_capability_repository_map::RepositoryEntryDisposition::GeneratedExcluded;
    }
    assert!(python_project_paths(&denied).is_err());
    let mut reversed = paths.clone();
    reversed.reverse();
    assert!(build_coding_project_plan_binding(&map, &reversed).is_err());
    assert!(
        build_coding_project_plan_binding(&map, &[paths[0].clone(), paths[0].clone()]).is_err()
    );
    assert!(build_coding_project_plan_binding(&map, &[]).is_err());
    let foreign = WorkspacePath::new(WorkspaceId::from_raw("foreign"), ["src", "z.py"]).unwrap();
    assert!(build_coding_project_plan_binding(&map, &[foreign]).is_err());
    let absent = WorkspacePath::new(map.workspace_id.clone(), ["src", "absent.py"]).unwrap();
    assert!(build_coding_project_plan_binding(&map, &[absent]).is_err());
}

#[test]
fn python_project_definition_enters_existing_profile_with_no_test_write_scope() {
    let map = project(&["src/a.py", "src/b.py", "tests/test_a.py"]);
    let paths = python_project_paths(&map).unwrap();
    let mut input = crate::coding_session::tests::input();
    let command = python_project_command().unwrap();
    assert_eq!(
        command.arguments,
        [
            "-I", "-B", "-m", "unittest", "discover", "-s", "tests", "-t", ".", "-v"
        ]
    );
    assert!(
        !command.interactive
            && command.grant_required
            && !command.network
            && !command.inherit_environment
    );
    let template =
        python_project_validation(command.clone(), &input.worktree.record_sha256).unwrap();
    assert_eq!(template.parser, ValidationParserKind::PythonUnittestTextV1);
    assert_eq!(template.minimum_test_count, 1);
    assert!(!template.focused && !template.fail_fast);
    input.commands = CommandRegistry::build(vec![command]).unwrap();
    input.validations = ValidationTemplateRegistry::build(vec![template]).unwrap();
    input.repository_snapshot_sha256 = map.map_sha256.clone();
    input.change_plan = build_coding_project_plan_binding(&map, &paths).unwrap();
    input.write_scope = CodingWriteScope::exact_files(
        map.workspace_id.clone(),
        paths
            .iter()
            .map(|p| {
                p.components()
                    .iter()
                    .map(|p| p.as_str().to_owned())
                    .collect()
            })
            .collect(),
    )
    .unwrap();
    let profile = CodingSessionProfile::build(input).unwrap();
    assert!(profile.write_scope().is_exact_files());
    assert!(
        profile
            .write_scope()
            .resolve(&["src".into(), "a.py".into(), "child.py".into()])
            .is_err()
    );
    assert!(CodingWriteScope::exact_files(map.workspace_id.clone(), vec![Vec::new()]).is_err());
    assert_eq!(
        profile.write_scope().writable_roots(),
        &[
            vec!["src".to_owned(), "a.py".to_owned()],
            vec!["src".to_owned(), "b.py".to_owned()]
        ]
    );
    let preimage = b"def value():\n    return 1\n";
    let proposal = |path: Vec<String>| StructuredPatchProposal {
        schema_version: 1,
        change_id: "project-change".into(),
        path,
        expected_preimage_sha256: sha256(preimage),
        intent_sha256: profile.change_plan().intent_sha256().into(),
        change_plan_sha256: profile.change_plan().plan_sha256().into(),
        language: StructuredLanguage::Python,
        artifact_class: StructuredArtifactClass::Code,
        edits: vec![StructuredEdit::RenameIdentifier {
            edit_id: "rename-value".into(),
            old: "value".into(),
            replacement: "result".into(),
        }],
        additional_review_hooks: vec![],
        generated: false,
        allow_generated: false,
    };
    for path in [["tests", "test_a.py"], ["src", "new.py"], ["other", "a.py"]] {
        assert!(
            crate::coding_changes::bind_structured_patch_proposal(
                profile.write_scope(),
                proposal(path.map(str::to_owned).into()),
                preimage.to_vec()
            )
            .is_err()
        );
    }
    assert!(
        crate::coding_changes::bind_structured_patch_proposal(
            profile.write_scope(),
            proposal(vec!["src".into(), "a.py".into()]),
            preimage.to_vec()
        )
        .is_ok()
    );
    let template = &profile.validations().templates[0];
    let changed_scope =
        python_project_validation(template.command.clone(), &"f".repeat(64)).unwrap();
    assert_ne!(template.template_sha256, changed_scope.template_sha256);
}

#[test]
fn exact_file_scope_is_profile_bound_and_cannot_create_even_its_selected_path() {
    let roots = vec![vec!["src".to_owned(), "lib.rs".to_owned()]];
    let mut directory = crate::coding_session::tests::input();
    directory.write_scope =
        CodingWriteScope::new(directory.write_scope.workspace_id().clone(), roots.clone()).unwrap();
    let mut exact = crate::coding_session::tests::input();
    exact.write_scope =
        CodingWriteScope::exact_files(exact.write_scope.workspace_id().clone(), roots).unwrap();
    let directory = CodingSessionProfile::build(directory).unwrap();
    let exact = CodingSessionProfile::build(exact).unwrap();
    assert_ne!(directory.profile_sha256(), exact.profile_sha256());
    let proposal = ControlledFileCreationProposal {
        schema_version: 1,
        creation_id: "create-project".into(),
        path: vec!["src".into(), "lib.rs".into()],
        content: "pub fn value() {}\n".into(),
        mode: 0o644,
        classification: ControlledFileClassification::SourceCode,
        intent_sha256: exact.change_plan().intent_sha256().into(),
        change_plan_sha256: exact.change_plan().plan_sha256().into(),
        expected_parent_sha256: "a".repeat(64),
    };
    let definition = crate::coding_changes::controlled_create_tool_definition();
    let bytes = serde_json::to_vec(&proposal).unwrap();
    let call = agentmage_kernel_contracts::ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw("create-project"),
        correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw("project-correlation"),
        action_id: agentmage_kernel_contracts::ActionId::from_raw("project-action"),
        tool_id: definition.tool_id,
        tool_version: definition.tool_version,
        arguments: ContractPayload {
            schema: definition.input_schema,
            media_type: "application/json".into(),
            sha256: sha256(&bytes),
            bytes,
        },
    };
    assert!(exact.registry().validate_arguments(&call).is_err());
}

#[test]
fn selected_python_command_observes_project_imports_failure_then_fix_and_zero_tests() {
    // Standalone trusted fixture: exercises command construction, not the
    // confined native runner, authority, coordinator or a model.
    let root = std::env::temp_dir().join(format!(
        "agentmage-python-project-command-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("tests")).unwrap();
    fs::write(root.join("tests/__init__.py"), "").unwrap();
    fs::write(
        root.join("src/calc.py"),
        "def add(a, b):\n    return a - b\n",
    )
    .unwrap();
    fs::write(root.join("tests/test_calc.py"), "import unittest\nfrom src.calc import add\nclass Calc(unittest.TestCase):\n    def test_add(self):\n        self.assertEqual(add(2, 3), 5)\n").unwrap();
    // A workspace module must not replace the standard runner's initial import.
    fs::write(
        root.join("unittest.py"),
        "raise RuntimeError('workspace runner shadow')\n",
    )
    .unwrap();
    let spec = python_project_command().unwrap();
    let run = || {
        std::process::Command::new(&spec.executable)
            .args(&spec.arguments)
            .current_dir(&root)
            .env_clear()
            .envs(&spec.environment)
            .output()
            .unwrap()
    };
    let failed = run();
    assert_eq!(failed.status.code(), Some(1));
    assert!(failed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("FAILED (failures=1)"));
    fs::write(
        root.join("src/calc.py"),
        "def add(a, b):\n    return a + b\n",
    )
    .unwrap();
    let passed = run();
    assert!(passed.status.success());
    assert!(passed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&passed.stderr).contains("Ran 1 test"));
    assert!(passed.stderr.ends_with(b"OK\n"));
    fs::remove_file(root.join("tests/test_calc.py")).unwrap();
    let empty = run();
    assert!(matches!(empty.status.code(), Some(0 | 5)));
    assert!(String::from_utf8_lossy(&empty.stderr).contains("Ran 0 tests"));
    // Zero discovered tests never satisfy the positive-test pass requirement.
    // Supported runtimes return either zero or the dedicated no-tests exit 5.
    fs::remove_dir_all(root).unwrap();
}

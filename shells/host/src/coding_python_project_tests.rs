// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic project composition tests; no native execution or model qualification.
use super::*;
use agentmage_capability_repository_map::{
    GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_repository_map,
};

fn project(names: &[&str]) -> RepositoryMap {
    project_inputs(
        &names
            .iter()
            .map(|name| (*name, b"def value():\n    return 1\n".as_slice()))
            .collect::<Vec<_>>(),
    )
}

fn project_inputs(files: &[(&str, &[u8])]) -> RepositoryMap {
    build_repository_map(RepositoryMapInput {
        workspace_id: WorkspaceId::from_raw("workspace-coding"),
        repository_sha256: "a".repeat(64),
        worktree_sha256: "2".repeat(64),
        branch: Some("refs/heads/agentmage/tasks/task-coding-0001".into()),
        commit_id: "1".repeat(40),
        policy_sha256: "d".repeat(64),
        freshness_sha256: "e".repeat(64),
        files: files
            .iter()
            .map(|(name, content)| RepositoryFileInput {
                path: name.split('/').map(str::to_owned).collect(),
                size_bytes: content.len() as u64,
                content_sha256: sha256(content),
                content: Some(content.to_vec()),
                object_kind: RepositoryObjectKind::RegularFile,
                git_state: GitTrackedState::TrackedClean,
                policy_excluded: false,
                generated: false,
                vendored: false,
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
    let paths = python_project_paths(&map, None).unwrap();
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
    assert!(python_project_paths(&project(&["tests/test_z.py", "setup.py"]), None).is_err());
    let names: Vec<_> = (0..17).map(|i| format!("src/f{i}.py")).collect();
    assert!(
        python_project_paths(
            &project(&names.iter().map(String::as_str).collect::<Vec<_>>()),
            None
        )
        .is_err()
    );
    let mut denied = map.clone();
    for file in &mut denied.files {
        file.content_read = false;
    }
    assert!(python_project_paths(&denied, None).is_err());
    let mut denied = map.clone();
    for file in &mut denied.files {
        file.disposition =
            agentmage_capability_repository_map::RepositoryEntryDisposition::GeneratedExcluded;
    }
    assert!(python_project_paths(&denied, None).is_err());
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
    let paths = python_project_paths(&map, None).unwrap();
    let mut input = crate::coding_session::tests::input();
    let command = python_project_command(CodingDevelopmentScenario::PythonProject).unwrap();
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
    let template = python_project_validation(
        CodingDevelopmentScenario::PythonProject,
        command.clone(),
        &input.worktree.record_sha256,
    )
    .unwrap();
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
    let changed_scope = python_project_validation(
        CodingDevelopmentScenario::PythonProject,
        template.command.clone(),
        &"f".repeat(64),
    )
    .unwrap();
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
    let spec = python_project_command(CodingDevelopmentScenario::PythonProject).unwrap();
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

#[test]
fn root_project_definition_is_explicit_exact_and_resume_bound() {
    let packaged = CodingDevelopmentScenario::PythonProject;
    let root = CodingDevelopmentScenario::PythonRootProject;
    assert_eq!(
        CodingDevelopmentScenario::parse("python-root-project"),
        Some(root)
    );
    for invalid in [
        "python-root-project;sh",
        "python-root-project ",
        "root-project",
    ] {
        assert_eq!(CodingDevelopmentScenario::parse(invalid), None);
    }
    let command = python_project_command(root).unwrap();
    let package_command = python_project_command(packaged).unwrap();
    assert_eq!(
        command.arguments,
        [
            "-I", "-B", "-m", "unittest", "discover", "-s", ".", "-t", ".", "-v"
        ]
    );
    assert!(
        !command.network
            && !command.interactive
            && !command.inherit_environment
            && command.grant_required
    );
    assert_eq!(command.bounds, package_command.bounds);
    assert_ne!(command, package_command);
    let template = python_project_validation(root, command, &"a".repeat(64)).unwrap();
    let package_template =
        python_project_validation(packaged, package_command, &"a".repeat(64)).unwrap();
    assert_ne!(template.template_sha256, package_template.template_sha256);
    assert_eq!(template.parser, ValidationParserKind::PythonUnittestTextV1);
    assert_eq!(template.minimum_test_count, 1);
    assert!(!template.fail_fast && !template.focused);
    for scenario in [
        CodingDevelopmentScenario::NoOp,
        CodingDevelopmentScenario::FailedTestRepair,
    ] {
        assert!(python_project_command(scenario).is_err());
        assert!(python_project_constraints_match(scenario, &[]));
    }
    let package_marker = packaged.python_project_constraint().unwrap().to_owned();
    let root_marker = root.python_project_constraint().unwrap().to_owned();
    for (scenario, expected, other) in [
        (packaged, &package_marker, &root_marker),
        (root, &root_marker, &package_marker),
    ] {
        assert!(python_project_constraints_match(
            scenario,
            &["Unrelated constraint".into(), expected.clone()]
        ));
        for constraints in [
            vec![],
            vec![other.clone()],
            vec![expected.clone(), other.clone()],
            vec![expected.clone(), expected.clone()],
        ] {
            assert!(!python_project_constraints_match(scenario, &constraints));
        }
        assert!(!python_project_constraints_match(
            CodingDevelopmentScenario::FailedTestRepair,
            std::slice::from_ref(expected)
        ));
    }
}

#[test]
fn root_python_command_runs_actual_failure_repair_import_error_and_zero_tests() {
    // Actual standard-interpreter execution in a synthetic repository. This
    // tests the selected argv, not native confinement, approvals or inference.
    let started = std::time::Instant::now();
    let root = std::env::temp_dir().join(format!(
        "agentmage-python-root-command-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("src/calc.py"),
        "def add(a, b):\n    return a - b\n",
    )
    .unwrap();
    fs::write(root.join("test_calc.py"), "import unittest\nfrom src.calc import add\nclass Calc(unittest.TestCase):\n    def test_add(self):\n        self.assertEqual(add(2, 3), 5)\n").unwrap();
    fs::write(
        root.join("unittest.py"),
        "raise RuntimeError('workspace runner shadow')\n",
    )
    .unwrap();
    let spec = python_project_command(CodingDevelopmentScenario::PythonRootProject).unwrap();
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
    fs::write(
        root.join("src/calc.py"),
        "raise ImportError('synthetic broken source')\n",
    )
    .unwrap();
    let broken = run();
    assert_eq!(broken.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&broken.stderr).contains("FAILED (errors=1)"));
    fs::remove_file(root.join("test_calc.py")).unwrap();
    let empty = run();
    assert!(matches!(empty.status.code(), Some(0 | 5)));
    assert!(String::from_utf8_lossy(&empty.stderr).contains("Ran 0 tests"));
    fs::remove_dir_all(root).unwrap();
    eprintln!(
        "root_project_fail_repair_error_empty_us={}",
        started.elapsed().as_micros()
    );
}

fn selected(names: &[&str]) -> CodingPythonSourceSelection {
    CodingPythonSourceSelection::new(
        names
            .iter()
            .map(|name| name.split('/').map(str::to_owned).collect())
            .collect(),
    )
    .unwrap()
}

#[test]
fn project_sources_exclude_discovery_modules_initializers_and_test_directories() {
    let map = project(&[
        "src/calc.py",
        "src/__init__.py",
        "src/nested/__init__.py",
        "src/test_calc.py",
        "src/nested/testHelpers.py",
        "src/tests/helper.py",
        "src/test/helper.py",
        "test_calc.py",
    ]);
    let paths = python_project_paths(&map, None).unwrap();
    let scope = CodingWriteScope::exact_files(
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
    assert_eq!(paths.len(), 1);
    for path in [
        vec!["src", "__init__.py"],
        vec!["src", "nested", "__init__.py"],
        vec!["src", "test_calc.py"],
        vec!["src", "nested", "testHelpers.py"],
        vec!["src", "tests", "helper.py"],
        vec!["src", "test", "helper.py"],
        vec!["test_calc.py"],
    ] {
        assert!(
            scope
                .resolve(&path.into_iter().map(str::to_owned).collect::<Vec<_>>())
                .is_err()
        );
    }
    assert!(python_project_paths(&project(&["src/test_calc.py"]), None).is_err());
    assert!(python_project_paths(&project(&["src/__init__.py"]), None).is_err());
}

#[test]
fn explicit_project_sources_are_canonical_exact_and_all_must_be_admissible() {
    let map = project(&[
        "calculator.py",
        "package/core.py",
        "src/unselected.py",
        "test_calc.py",
    ]);
    let selection = selected(&["package/core.py", "calculator.py"]);
    assert_eq!(
        selection.constraint,
        selected(&["calculator.py", "package/core.py"]).constraint
    );
    let paths = python_project_paths(&map, Some(&selection)).unwrap();
    assert_eq!(paths.len(), 2);
    let plan = build_coding_project_plan_binding(&map, &paths).unwrap();
    assert!(plan.verify());
    let scope =
        CodingWriteScope::exact_files(map.workspace_id.clone(), selection.paths.clone()).unwrap();
    assert!(scope.resolve(&["calculator.py".into()]).is_ok());
    for path in [
        vec!["src", "unselected.py"],
        vec!["package", "absent.py"],
        vec!["package", "core.py", "child.py"],
        vec!["test_calc.py"],
    ] {
        assert!(
            scope
                .resolve(&path.into_iter().map(str::to_owned).collect::<Vec<_>>())
                .is_err()
        );
    }
    assert!(python_project_paths(&map, Some(&selected(&["missing.py"]))).is_err());
    for mutate in 0..3 {
        let mut denied = map.clone();
        for file in &mut denied.files {
            match mutate {
                0 => file.content_read = false,
                1 => file.object_kind = RepositoryObjectKind::SymbolicLink,
                _ => file.disposition = agentmage_capability_repository_map::RepositoryEntryDisposition::GeneratedExcluded,
            }
        }
        assert!(python_project_paths(&denied, Some(&selection)).is_err());
    }
    // Every selected input must be admitted; never silently narrow an explicit list.
    assert!(python_project_paths(&map, Some(&selected(&["calculator.py", "missing.py"]))).is_err());
}

#[test]
fn root_discovery_initializer_hook_cannot_enter_the_mutable_scope() {
    // Reproduce the review's actual discovery hook with a trusted interpreter,
    // then reject that edit through both default and explicit source selection.
    // This runs outside native confinement and supplies no coding score.
    let started = std::time::Instant::now();
    let root = std::env::temp_dir().join(format!(
        "agentmage-python-discovery-hook-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("src")).unwrap();
    let source = "def add(a, b):\n    return a - b\n";
    let tests = "import unittest\nfrom src.calc import add\nclass Calc(unittest.TestCase):\n    def test_add(self):\n        self.assertEqual(add(2, 3), 5)\n";
    let hook = "import unittest\nclass Decoy(unittest.TestCase):\n    def test_decoy(self):\n        pass\ndef load_tests(loader, tests, pattern):\n    return tests\n";
    fs::write(root.join("src/__init__.py"), "").unwrap();
    fs::write(root.join("src/calc.py"), source).unwrap();
    fs::write(root.join("src/test_calc.py"), tests).unwrap();
    let spec = python_project_command(CodingDevelopmentScenario::PythonRootProject).unwrap();
    let run = || {
        std::process::Command::new(&spec.executable)
            .args(&spec.arguments)
            .current_dir(&root)
            .env_clear()
            .envs(&spec.environment)
            .output()
            .unwrap()
    };
    let failure = run();
    assert_eq!(failure.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failure.stderr).contains("FAILED (failures=1)"));
    fs::write(root.join("src/__init__.py"), hook).unwrap();
    let hidden = run();
    assert!(hidden.status.success());
    assert!(String::from_utf8_lossy(&hidden.stderr).contains("Ran 1 test"));
    assert!(hidden.stderr.ends_with(b"OK\n"));
    assert_eq!(
        fs::read_to_string(root.join("src/calc.py")).unwrap(),
        source
    );
    assert_eq!(
        fs::read_to_string(root.join("src/test_calc.py")).unwrap(),
        tests
    );

    let map = project_inputs(&[
        ("src/__init__.py", b""),
        ("src/calc.py", source.as_bytes()),
        ("src/test_calc.py", tests.as_bytes()),
    ]);
    let explicit = selected(&["src/calc.py"]);
    for selection in [None, Some(&explicit)] {
        let paths = python_project_paths(&map, selection).unwrap();
        assert_eq!(paths.len(), 1);
        let scope = CodingWriteScope::exact_files(
            map.workspace_id.clone(),
            paths
                .iter()
                .map(|path| {
                    path.components()
                        .iter()
                        .map(|p| p.as_str().to_owned())
                        .collect()
                })
                .collect(),
        )
        .unwrap();
        assert!(scope.resolve(&["src".into(), "calc.py".into()]).is_ok());
        assert!(
            scope
                .resolve(&["src".into(), "__init__.py".into()])
                .is_err()
        );
        assert!(
            scope
                .resolve(&["src".into(), "test_calc.py".into()])
                .is_err()
        );
    }
    // Reject the entire mixed selection instead of silently dropping the hook.
    assert!(
        CodingPythonSourceSelection::new(vec![
            vec!["src".into(), "calc.py".into()],
            vec!["src".into(), "__init__.py".into()],
        ])
        .is_err()
    );

    fs::write(root.join("src/__init__.py"), "").unwrap();
    let restored = run();
    assert_eq!(restored.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&restored.stderr).contains("FAILED (failures=1)"));
    fs::remove_dir_all(root).unwrap();
    eprintln!(
        "root_initializer_hook_probe_us={}",
        started.elapsed().as_micros()
    );
}

#[test]
fn explicit_project_sources_refuse_unsafe_empty_duplicate_and_test_paths() {
    for names in [
        vec![],
        vec!["a.py", "a.py"],
        vec![""],
        vec!["../a.py"],
        vec!["/a.py"],
        vec!["a/../b.py"],
        vec!["a.py/child"],
        vec!["src/a.rs"],
        vec!["src/test_calc.py"],
        vec!["test.py"],
        vec!["tests/helper.py"],
        vec!["package/test/helper.py"],
        vec!["__init__.py"],
        vec!["src/__init__.py"],
        vec!["package/nested/__init__.py"],
        vec![".git/config.py"],
        vec![".hidden/a.py"],
        vec!["src//a.py"],
    ] {
        let paths = names
            .iter()
            .map(|name| name.split('/').map(str::to_owned).collect())
            .collect();
        assert!(
            CodingPythonSourceSelection::new(paths).is_err(),
            "{names:?}"
        );
    }
    let names = (0..17).map(|i| vec![format!("a{i}.py")]).collect();
    assert!(CodingPythonSourceSelection::new(names).is_err());
    assert!(CodingPythonSourceSelection::new(vec![vec!["x".repeat(256)]]).is_err());
}

#[test]
fn explicit_source_selection_cannot_be_removed_replaced_or_duplicated_on_resume() {
    let a = selected(&["calculator.py"]);
    let b = selected(&["package/core.py"]);
    assert_ne!(a.constraint, b.constraint);
    assert!(python_source_constraints_match(None, &[]));
    assert!(python_source_constraints_match(
        Some(&a),
        &["unrelated".into(), a.constraint.clone()]
    ));
    assert!(!python_source_constraints_match(
        None,
        std::slice::from_ref(&a.constraint)
    ));
    for constraints in [
        vec![],
        vec![b.constraint.clone()],
        vec![a.constraint.clone(), a.constraint.clone()],
        vec![a.constraint.clone(), b.constraint.clone()],
        vec![format!("{PYTHON_SOURCE_CONSTRAINT_PREFIX}invalid")],
    ] {
        assert!(!python_source_constraints_match(Some(&a), &constraints));
    }
}

#[test]
fn root_validation_identity_describes_its_actual_selection_and_refuses_mismatched_command() {
    let root = CodingDevelopmentScenario::PythonRootProject;
    let command = python_project_command(root).unwrap();
    let expected = development_sha256(&serde_json::json!({"selection":"python-root-project", "command":command, "parser":"python_unittest_text_v1", "minimum_test_count":1})).unwrap();
    let validation = python_project_validation(root, command.clone(), &"a".repeat(64)).unwrap();
    assert_eq!(validation.source_sha256, expected);
    assert_eq!(validation.user_input_approval_sha256, Some(expected));
    assert!(
        python_project_validation(
            CodingDevelopmentScenario::PythonProject,
            command.clone(),
            &"a".repeat(64)
        )
        .is_err()
    );
    assert!(
        python_project_validation(
            CodingDevelopmentScenario::FailedTestRepair,
            command,
            &"a".repeat(64)
        )
        .is_err()
    );
}

#[test]
fn explicit_top_level_source_patch_repairs_actual_test_without_writing_tests() {
    // Trusted component execution, outside the native sandbox: selected scope ->
    // strict patch planner -> real interpreter. No model or workflow claim.
    let started = std::time::Instant::now();
    let root = std::env::temp_dir().join(format!(
        "agentmage-python-selected-source-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let preimage = "def add(a, b):\n    return a - b\n";
    let tests = "import unittest\nfrom calculator import add\nclass Calc(unittest.TestCase):\n    def test_add(self):\n        self.assertEqual(add(2, 3), 5)\n";
    fs::write(root.join("calculator.py"), preimage).unwrap();
    fs::write(root.join("test_calc.py"), tests).unwrap();
    let map = project_inputs(&[
        ("calculator.py", preimage.as_bytes()),
        ("test_calc.py", tests.as_bytes()),
    ]);
    let selection = selected(&["calculator.py"]);
    let paths = python_project_paths(&map, Some(&selection)).unwrap();
    let plan = build_coding_project_plan_binding(&map, &paths).unwrap();
    let scope =
        CodingWriteScope::exact_files(map.workspace_id.clone(), selection.paths.clone()).unwrap();
    let spec = python_project_command(CodingDevelopmentScenario::PythonRootProject).unwrap();
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
    assert!(String::from_utf8_lossy(&failed.stderr).contains("FAILED (failures=1)"));
    let start = preimage.find("a - b").unwrap() as u64;
    let proposal = StructuredPatchProposal {
        schema_version: 1,
        change_id: "selected-source-fix".into(),
        path: vec!["calculator.py".into()],
        expected_preimage_sha256: sha256(preimage.as_bytes()),
        intent_sha256: plan.intent_sha256().into(),
        change_plan_sha256: plan.plan_sha256().into(),
        language: StructuredLanguage::Python,
        artifact_class: StructuredArtifactClass::Code,
        edits: vec![StructuredEdit::ReplaceSyntaxNode {
            edit_id: "fix-sum".into(),
            start_byte: start,
            end_byte: start + 5,
            expected_node_sha256: sha256(b"a - b"),
            replacement: "a + b".into(),
        }],
        additional_review_hooks: vec![],
        generated: false,
        allow_generated: false,
    };
    let mut denied = proposal.clone();
    denied.path = vec!["test_calc.py".into()];
    assert!(
        crate::coding_changes::bind_structured_patch_proposal(
            &scope,
            denied,
            preimage.as_bytes().to_vec()
        )
        .is_err()
    );
    let patch = crate::coding_changes::bind_structured_patch_proposal(
        &scope,
        proposal,
        preimage.as_bytes().to_vec(),
    )
    .unwrap();
    fs::write(root.join("calculator.py"), patch.postimage()).unwrap();
    let passed = run();
    assert!(passed.status.success());
    assert!(String::from_utf8_lossy(&passed.stderr).contains("Ran 1 test"));
    assert!(passed.stderr.ends_with(b"OK\n"));
    assert_eq!(
        fs::read_to_string(root.join("test_calc.py")).unwrap(),
        tests
    );
    fs::remove_dir_all(root).unwrap();
    eprintln!(
        "explicit_source_strict_patch_fail_repair_us={}",
        started.elapsed().as_micros()
    );
}

#[test]
fn project_resume_binds_the_complete_profile_including_the_default_write_scope() {
    let default_input = crate::coding_session::tests::input();
    let mut exact_input = crate::coding_session::tests::input();
    exact_input.write_scope = CodingWriteScope::exact_files(
        exact_input.write_scope.workspace_id().clone(),
        vec![vec!["src".into(), "lib.rs".into()]],
    )
    .unwrap();
    let default = CodingSessionProfile::build(default_input).unwrap();
    let exact = CodingSessionProfile::build(exact_input).unwrap();
    let marker = format!(
        "{PYTHON_PROFILE_CONSTRAINT_PREFIX}{}",
        exact.profile_sha256()
    );
    assert!(python_profile_constraints_match(
        Some(&exact),
        std::slice::from_ref(&marker)
    ));
    assert!(!python_profile_constraints_match(
        Some(&default),
        std::slice::from_ref(&marker)
    ));
    assert!(!python_profile_constraints_match(Some(&exact), &[]));
    assert!(!python_profile_constraints_match(
        Some(&exact),
        &[marker.clone(), marker.clone()]
    ));
    assert!(!python_profile_constraints_match(None, &[marker]));
    assert!(python_profile_constraints_match(None, &[]));
}

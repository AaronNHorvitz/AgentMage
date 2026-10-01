use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use agentmage_kernel_engine::command_runner::{
    CommandBounds, CommandRisk, CommandSpec, CommandWorkingDirectory,
};
use agentmage_kernel_engine::engineering_recipe::{
    RecipeParameter, RecipeVersion, seal_recipe_manifest,
};
use agentmage_kernel_engine::validation_template::{
    ValidationParserKind, ValidationTemplateInput, ValidationTemplateSource,
    seal_validation_template,
};

use super::*;

fn request() -> RuntimeRunRequest {
    crate::runtime_read_tests::completed_native_read_fixture().0
}

fn workspace() -> WorkspaceId {
    request().workspace_id
}

fn template(
    id: &str,
    kind: ValidationKind,
) -> agentmage_kernel_engine::validation_template::ValidationTemplate {
    seal_validation_template(ValidationTemplateInput {
        validation_id: format!("validation-{id}"),
        kind,
        command: CommandSpec::seal(
            format!("command-{id}"),
            "1.0.0",
            "/usr/bin/cargo",
            "1".repeat(64),
            vec!["test".to_owned()],
            CommandWorkingDirectory::EmptyScratch,
            BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
            CommandRisk::Moderate,
            CommandBounds::new(30_000, 65_536, 65_536, 256 * 1024 * 1024, 32, 200).unwrap(),
        )
        .unwrap(),
        source: ValidationTemplateSource::TrustedProjectConfiguration,
        source_sha256: "2".repeat(64),
        source_path: Some(WorkspacePath::new(workspace(), ["tests", "run_validation.py"]).unwrap()),
        user_input_approval_sha256: None,
        execution_scope_sha256: "3".repeat(64),
        parser: if kind.is_test() {
            ValidationParserKind::AgentMageJsonV1
        } else {
            ValidationParserKind::ProcessExitV1
        },
        parser_version: "1.0.0".to_owned(),
        parser_sha256: "4".repeat(64),
        minimum_test_count: u32::from(kind.is_test()),
        focused: kind.is_test(),
        fail_fast: true,
        failed_test_rerun_allowed: kind.is_test(),
        setup_idempotent: true,
        expected_artifacts: Vec::new(),
    })
    .unwrap()
}

/// The development profile registers one unit validation.
fn registry() -> ValidationTemplateRegistry {
    ValidationTemplateRegistry::build(vec![template("unit", ValidationKind::Unit)]).unwrap()
}

/// The sample recipe that holds a repair to one file under `src`.
fn repair_manifest() -> RecipeManifest {
    seal_recipe_manifest(RecipeManifest {
        schema_version: 1,
        recipe_id: "agentmage-sample-repair".to_owned(),
        version: RecipeVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },
        kind: RecipeKind::Tests,
        title: "Change only the code under test until its unit validation passes".to_owned(),
        parameters: vec![
            RecipeParameter {
                name: "attempts".to_owned(),
                summary: "How many repair attempts the person expects".to_owned(),
                parameter_type: RecipeParameterType::Integer {
                    minimum: 1,
                    maximum: 3,
                },
                required: false,
            },
            RecipeParameter {
                name: "target".to_owned(),
                summary: "The source file the repair is about".to_owned(),
                parameter_type: RecipeParameterType::Path,
                required: true,
            },
        ],
        scope: vec!["src".to_owned()],
        max_changed_files: 1,
        prerequisites: vec![RecipePrerequisite::CleanWorktree],
        verification: vec![ValidationKind::Unit],
        rollback: RecipeRollback::RevertWritesOnly,
        needs_network_grant: false,
        manifest_sha256: String::new(),
    })
    .unwrap()
}

/// A sample whose scope holds only tests, so a repair of `src` is refused.
fn tests_only_manifest() -> RecipeManifest {
    let mut manifest = repair_manifest();
    manifest.recipe_id = "agentmage-sample-tests-only".to_owned();
    manifest.title = "Change only the tests".to_owned();
    manifest.parameters.clear();
    manifest.scope = vec!["tests".to_owned()];
    seal_recipe_manifest(manifest).unwrap()
}

/// A sample that would need a network grant.
fn needs_network_manifest() -> RecipeManifest {
    let mut manifest = repair_manifest();
    manifest.recipe_id = "agentmage-sample-needs-network".to_owned();
    manifest.needs_network_grant = true;
    seal_recipe_manifest(manifest).unwrap()
}

/// A sample verified by a build validation the development profile does not
/// register.
fn build_verified_manifest() -> RecipeManifest {
    let mut manifest = repair_manifest();
    manifest.recipe_id = "agentmage-sample-build-verified".to_owned();
    manifest.kind = RecipeKind::DependencyUpdate;
    manifest.prerequisites = vec![
        RecipePrerequisite::CleanWorktree,
        RecipePrerequisite::LockedDependencies,
    ];
    manifest.verification = vec![ValidationKind::Build];
    seal_recipe_manifest(manifest).unwrap()
}

fn pretty(manifest: &RecipeManifest) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(manifest).unwrap();
    bytes.push(b'\n');
    bytes
}

fn sample_files() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        ("repair-in-src.json".to_owned(), pretty(&repair_manifest())),
        ("tests-only.json".to_owned(), pretty(&tests_only_manifest())),
        (
            "needs-network.json".to_owned(),
            pretty(&needs_network_manifest()),
        ),
        (
            "build-verified.json".to_owned(),
            pretty(&build_verified_manifest()),
        ),
    ])
}

fn sample_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/recipe-sample")
}

fn pairs(values: &[(&str, &str)]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn sent() -> RuntimeRecipeRequest {
    recipe_request(
        repair_manifest(),
        &pairs(&[("target", "src/calc.py"), ("attempts", "2")]),
        &workspace(),
    )
    .unwrap()
}

/// A run request whose constraints bind `plan`, as the host prepares it.
fn bound_request(plan: &RecipePlan) -> RuntimeRunRequest {
    let mut request = request();
    request.task.constraints.extend(recipe_constraints(plan));
    request
}

#[test]
fn the_committed_samples_are_the_manifests_this_test_seals() {
    // Decision 0133: every committed sample is reproduced byte for byte, and
    // nothing else is in the sample directory.
    let mut committed = BTreeMap::new();
    for entry in std::fs::read_dir(sample_root()).unwrap() {
        let path = entry.unwrap().path();
        assert!(std::fs::symlink_metadata(&path).unwrap().is_file());
        committed.insert(
            path.file_name().unwrap().to_str().unwrap().to_owned(),
            std::fs::read(&path).unwrap(),
        );
    }
    assert_eq!(committed, sample_files());
    for bytes in committed.values() {
        assert!(parse_recipe_manifest(bytes).is_ok());
    }
}

#[test]
fn parameters_are_typed_by_their_declaration_and_checked_before_launch() {
    for (argument, expected) in [
        ("target=src/calc.py", Some(("target", "src/calc.py"))),
        ("attempts=2", Some(("attempts", "2"))),
        ("note=a=b", Some(("note", "a=b"))),
        ("target", None),
        ("=src/calc.py", None),
        ("target=", None),
        ("Target=x", None),
        ("2target=x", None),
        ("tar-get=x", None),
        ("target=src/\u{7}calc.py", None),
    ] {
        assert_eq!(
            parse_recipe_parameter(argument),
            expected.map(|(name, value)| (name.to_owned(), value.to_owned())),
            "{argument:?}"
        );
    }
    assert_eq!(
        parse_recipe_parameter(&format!(
            "target={}",
            "a".repeat(MAX_RECIPE_PARAMETER_BYTES)
        )),
        None
    );
    let integer = RecipeParameterType::Integer {
        minimum: 1,
        maximum: 3,
    };
    assert_eq!(recipe_value(&integer, "2"), Some(RecipeValue::Integer(2)));
    assert_eq!(recipe_value(&integer, "-2"), Some(RecipeValue::Integer(-2)));
    for text in ["+2", "02", "2.0", "-0", "two", ""] {
        assert_eq!(recipe_value(&integer, text), None, "{text:?}");
    }
    let request = sent();
    assert_eq!(
        request.parameters,
        BTreeMap::from([
            ("attempts".to_owned(), RecipeValue::Integer(2)),
            (
                "target".to_owned(),
                RecipeValue::Path("src/calc.py".to_owned())
            ),
        ])
    );
    // Each of these is refused before any host is launched.
    let invalid = RecipeRunRefusal::Recipe(RecipeError::ParameterInvalid);
    for values in [
        pairs(&[("target", "src/calc.py"), ("unknown", "1")]),
        pairs(&[("target", "src/calc.py"), ("target", "src/other.py")]),
        pairs(&[("target", "src/calc.py"), ("attempts", "4")]),
        pairs(&[("target", "src/calc.py"), ("attempts", "two")]),
        pairs(&[("target", "tests/test_calc.py")]),
        pairs(&[("target", "src/../tests/test_calc.py")]),
        pairs(&[("attempts", "1")]),
    ] {
        assert_eq!(
            recipe_request(repair_manifest(), &values, &workspace()),
            Err(invalid),
            "{values:?}"
        );
    }
    assert!(
        recipe_request(
            repair_manifest(),
            &pairs(&[("target", "src/calc.py")]),
            &workspace()
        )
        .is_ok()
    );
    assert_eq!(
        recipe_request(
            needs_network_manifest(),
            &pairs(&[("target", "src/calc.py")]),
            &workspace()
        ),
        Err(RecipeRunRefusal::NetworkGrantUnavailable)
    );
    let mut unsealed = repair_manifest();
    unsealed.max_changed_files = 5;
    assert_eq!(
        recipe_request(unsealed, &pairs(&[("target", "src/calc.py")]), &workspace()),
        Err(RecipeRunRefusal::Recipe(RecipeError::ManifestInvalid))
    );
}

#[test]
fn a_manifest_file_is_read_only_when_absolute_regular_bounded_and_sealed() {
    let directory = fixture::Directory::new("recipe-file");
    let valid = directory.file("recipe.json", &pretty(&repair_manifest()));
    assert_eq!(read_recipe_manifest(&valid), Ok(repair_manifest()));
    let link = directory.link("link.json", &valid);
    let fifo = directory.fifo("fifo.json");
    let large = directory.file(
        "large.json",
        &vec![b' '; usize::try_from(MAX_RECIPE_FILE_BYTES).unwrap() + 1],
    );
    // A relative path is refused even where it names a readable manifest:
    // tests run in the package directory, which holds the samples.
    let relative = PathBuf::from("fixtures/recipe-sample/repair-in-src.json");
    assert!(relative.is_file());
    for path in [
        relative,
        directory.0.join("missing.json"),
        link,
        fifo,
        directory.0.clone(),
        directory.file("empty.json", b""),
        large,
    ] {
        assert_eq!(
            read_recipe_manifest(&path),
            Err(RecipeRunRefusal::FileInvalid),
            "{path:?}"
        );
    }
    let mut unsealed = repair_manifest();
    unsealed.scope = vec!["tests".to_owned()];
    let mut extra = serde_json::to_value(repair_manifest()).unwrap();
    extra["granted"] = serde_json::Value::Bool(true);
    for bytes in [
        b"{".to_vec(),
        pretty(&unsealed),
        serde_json::to_vec(&extra).unwrap(),
    ] {
        assert_eq!(
            read_recipe_manifest(&directory.file("refused.json", &bytes)),
            Err(RecipeRunRefusal::Recipe(RecipeError::ManifestInvalid))
        );
    }
}

#[test]
fn the_host_instantiates_the_plan_against_the_workspace_registry() {
    let plan = instantiate_run_recipe(&sent(), &workspace(), &registry()).unwrap();
    assert!(verify_recipe_plan(&plan) && plan.proposal_only);
    assert_eq!(plan.validations.len(), 1);
    assert_eq!(plan.validations[0].kind, ValidationKind::Unit);
    assert_eq!(plan.validation_registry_sha256, registry().registry_sha256);
    // A declared validation kind the workspace does not register, a registry
    // that does not verify, a network recipe, an unsealed manifest and a
    // value the manifest refuses are each refused with a closed code.
    let build = RuntimeRecipeRequest {
        manifest: build_verified_manifest(),
        parameters: sent().parameters,
    };
    let mut broken = registry();
    broken.registry_sha256 = "0".repeat(64);
    let network = RuntimeRecipeRequest {
        manifest: needs_network_manifest(),
        parameters: sent().parameters,
    };
    let mut unsealed = sent();
    unsealed.manifest.max_changed_files = 5;
    let mut outside = sent();
    outside.parameters.insert(
        "target".to_owned(),
        RecipeValue::Path("tests/test_calc.py".to_owned()),
    );
    for (request, registry, refusal) in [
        (
            build,
            registry(),
            RecipeRunRefusal::Recipe(RecipeError::VerificationUnavailable),
        ),
        (
            sent(),
            broken,
            RecipeRunRefusal::Recipe(RecipeError::RegistryInvalid),
        ),
        (
            network,
            registry(),
            RecipeRunRefusal::NetworkGrantUnavailable,
        ),
        (
            unsealed,
            registry(),
            RecipeRunRefusal::Recipe(RecipeError::ManifestInvalid),
        ),
        (
            outside,
            registry(),
            RecipeRunRefusal::Recipe(RecipeError::ParameterInvalid),
        ),
    ] {
        assert_eq!(
            instantiate_run_recipe(&request, &workspace(), &registry),
            Err(refusal)
        );
    }
    for (refusal, code) in [
        (RecipeRunRefusal::FileInvalid, "recipe.file-invalid"),
        (
            RecipeRunRefusal::NetworkGrantUnavailable,
            "recipe.network-grant-unavailable",
        ),
        (
            RecipeRunRefusal::Recipe(RecipeError::VerificationUnavailable),
            "recipe.verification-unavailable",
        ),
    ] {
        assert_eq!(refusal.code(), code);
    }
}

#[test]
fn the_plan_is_bound_into_the_run_and_kept_only_when_it_is_what_was_sent() {
    let plan = instantiate_run_recipe(&sent(), &workspace(), &registry()).unwrap();
    let constraints = recipe_constraints(&plan);
    assert_eq!(
        constraints[0],
        format!("Recipe plan digest: {}", plan.plan_sha256)
    );
    assert!(constraints[1].contains("change only files under src, at most 1 distinct files"));
    let bound = bound_request(&plan);
    assert_eq!(declared_plan_sha256s(&bound), [plan.plan_sha256.as_str()]);
    assert!(declared_plan_sha256s(&request()).is_empty());
    // A prepared request binds a plan exactly when a recipe was sent.
    assert!(request_binds_recipe(&bound, Some(&sent())));
    assert!(request_binds_recipe(&request(), None));
    assert!(!request_binds_recipe(&request(), Some(&sent())));
    assert!(!request_binds_recipe(&bound, None));
    let mut twice = bound.clone();
    twice.task.constraints.push(constraints[0].clone());
    assert_eq!(declared_plan_sha256s(&twice).len(), 2);
    assert!(!request_binds_recipe(&twice, Some(&sent())));
    let mut malformed = request();
    malformed
        .task
        .constraints
        .push("Recipe plan digest: x".to_owned());
    assert!(!request_binds_recipe(&malformed, Some(&sent())));
    // The client keeps the declared plan only when it is the plan of what it
    // sent, for this run's workspace and bound by this run's constraints.
    assert!(verify_declared_recipe_plan(&plan, &bound, Some(&sent())));
    let mut other_values = sent();
    other_values
        .parameters
        .insert("attempts".to_owned(), RecipeValue::Integer(3));
    let other_plan = instantiate_run_recipe(&other_values, &workspace(), &registry()).unwrap();
    let mut tampered = plan.clone();
    tampered.max_changed_files = 9;
    let mut other_workspace = request();
    other_workspace.workspace_id = WorkspaceId::from_raw("workspace-other");
    let foreign = instantiate_run_recipe(
        &sent(),
        &other_workspace.workspace_id,
        &ValidationTemplateRegistry::build(vec![template("unit", ValidationKind::Unit)]).unwrap(),
    )
    .unwrap();
    // A host could reseal a plan that differs from the sent manifest in any
    // one member; each such plan verifies on its own and is still refused.
    let resealed = |change: &dyn Fn(&mut RecipePlan)| {
        let mut changed = plan.clone();
        change(&mut changed);
        changed.plan_sha256.clear();
        changed.plan_sha256 = {
            use sha2::{Digest as _, Sha256};
            Sha256::digest(serde_json::to_vec(&changed).unwrap())
                .iter()
                .fold(String::new(), |mut output, byte| {
                    let _ = write!(output, "{byte:02x}");
                    output
                })
        };
        assert!(verify_recipe_plan(&changed));
        changed
    };
    let mut refused = vec![
        (plan.clone(), bound.clone(), None),
        (plan.clone(), bound.clone(), Some(other_values)),
        (other_plan, bound.clone(), Some(sent())),
        (tampered, bound.clone(), Some(sent())),
        (plan.clone(), request(), Some(sent())),
        (foreign.clone(), bound_request(&foreign), Some(sent())),
    ];
    let changes: [&dyn Fn(&mut RecipePlan); 6] = [
        &|plan| plan.max_changed_files = 9,
        &|plan| {
            plan.scope =
                vec![WorkspaceScopePath::new(plan.workspace_id.clone(), ["tests"]).unwrap()];
        },
        &|plan| plan.validations.clear(),
        &|plan| {
            plan.rollback = RecipeRollback::Irreversible {
                reason_code: "recipe.no-rollback".to_owned(),
            }
        },
        &|plan| plan.prerequisites.clear(),
        &|plan| plan.network_grant_required = true,
    ];
    for change in changes {
        let changed = resealed(change);
        let request = bound_request(&changed);
        refused.push((changed, request, Some(sent())));
    }
    for (declared, request, sent_request) in &refused {
        assert!(!verify_declared_recipe_plan(
            declared,
            request,
            sent_request.as_ref()
        ));
    }
}

#[test]
fn a_run_scope_admits_writes_inside_the_plan_until_its_bound() {
    let mut manifest = repair_manifest();
    manifest.max_changed_files = 2;
    let manifest = seal_recipe_manifest(manifest).unwrap();
    let request =
        recipe_request(manifest, &pairs(&[("target", "src/calc.py")]), &workspace()).unwrap();
    let plan = instantiate_run_recipe(&request, &workspace(), &registry()).unwrap();
    let path = |value: &str| WorkspacePath::new(workspace(), value.split('/')).unwrap();
    let shared = SharedRecipeScope::new(RecipeRunScope::new(plan.clone()).unwrap());
    // A clone is the same scope, as a run continued in its host sees it.
    let continued = shared.clone();
    assert_eq!(shared.admit(&path("src/calc.py")), Ok(()));
    assert_eq!(continued.admit(&path("src/calc.py")), Ok(()));
    // A refused path is not counted.
    assert_eq!(
        shared.admit(&path("tests/test_calc.py")),
        Err(RecipeError::OutOfScope)
    );
    let foreign =
        WorkspacePath::new(WorkspaceId::from_raw("workspace-other"), ["src", "calc.py"]).unwrap();
    assert_eq!(shared.admit(&foreign), Err(RecipeError::OutOfScope));
    assert_eq!(continued.admit(&path("src/subtract.py")), Ok(()));
    assert_eq!(
        shared.admit(&path("src/third.py")),
        Err(RecipeError::TooManyChanges)
    );
    assert_eq!(shared.admit(&path("src/subtract.py")), Ok(()));
    assert_eq!(shared.plan(), Some(plan.clone()));
    let mut scope = RecipeRunScope::new(plan.clone()).unwrap();
    assert_eq!(scope.admit(&path("src/calc.py")), Ok(()));
    assert_eq!(scope.admitted_count(), 1);
    assert_eq!(scope.plan(), &plan);
    let mut tampered = plan;
    tampered.max_changed_files = 9;
    assert_eq!(
        RecipeRunScope::new(tampered).err(),
        Some(RecipeError::PlanInvalid)
    );
}

#[test]
fn only_writes_have_a_target_the_plan_checks() {
    let path = WorkspacePath::new(workspace(), ["src", "calc.py"]).unwrap();
    assert_eq!(
        recipe_write_target(&NativeCodingTargetPlan::ExistingFile {
            path: path.clone(),
            expected_preimage_sha256: "a".repeat(64),
        }),
        Some(&path)
    );
    assert_eq!(
        recipe_write_target(&NativeCodingTargetPlan::DestinationParent {
            parent: Some(WorkspacePath::new(workspace(), ["src"]).unwrap()),
            destination: path.clone(),
            expected_parent_sha256: "b".repeat(64),
        }),
        Some(&path)
    );
    assert_eq!(
        recipe_write_target(&NativeCodingTargetPlan::OwnedWorktreeRoot),
        None
    );
    assert_eq!(
        recipe_write_target(&NativeCodingTargetPlan::ReadProjection {
            objects: Vec::new(),
            projection_sha256: "c".repeat(64),
        }),
        None
    );
}

#[test]
fn the_person_sees_the_recipe_before_launch_and_the_plan_after_the_run() {
    let mut quoted = repair_manifest();
    quoted.title = "Repair the \"calc\" module".to_owned();
    let quoted = seal_recipe_manifest(quoted).unwrap();
    let request =
        recipe_request(quoted, &pairs(&[("target", "src/calc.py")]), &workspace()).unwrap();
    let text = render_recipe_request(&request, false);
    assert!(text.starts_with(&format!(
        "recipe agentmage-sample-repair 1.0.0 (tests) \"Repair the \\\"calc\\\" module\" manifest {}\n",
        &request.manifest.manifest_sha256[..12]
    )));
    for line in [
        "  may change: src; at most 1 files\n",
        "  verified by registered unit validation\n",
        "  prerequisites (shown, not checked by the plan): clean worktree\n",
        "  target = src/calc.py\n",
        "  rollback: the plan's own writes, each by an inverse write with its own approval\n",
        "  proposal only: a write outside the plan is refused; each write inside it still needs your approval\n",
    ] {
        assert!(text.contains(line), "{line}");
    }
    let json: serde_json::Value =
        serde_json::from_str(render_recipe_request(&request, true).trim_end()).unwrap();
    assert_eq!(json["type"], "recipe_requested");
    assert_eq!(json["proposal_only"], true);
    assert_eq!(json["parameters"]["target"]["value"], "src/calc.py");
    let plan = instantiate_run_recipe(&sent(), &workspace(), &registry()).unwrap();
    let rendered = render_declared_recipe_plan(Some(&plan), true, false);
    assert!(rendered.starts_with(&format!(
        "recipe plan agentmage-sample-repair 1.0.0 (tests) digest {}",
        &plan.plan_sha256[..12]
    )));
    assert!(rendered.contains("  scope: src; at most 1 files\n"));
    assert!(rendered.contains("  validation validation-unit (unit) template "));
    assert!(rendered.ends_with("  proposal only: the plan granted nothing\n"));
    assert_eq!(render_declared_recipe_plan(None, false, false), "");
    assert_eq!(render_declared_recipe_plan(None, false, true), "");
    assert_eq!(
        render_declared_recipe_plan(None, true, false),
        "recipe plan: unavailable; the host did not declare the plan that was sent\n"
    );
    let unavailable: serde_json::Value =
        serde_json::from_str(render_declared_recipe_plan(None, true, true).trim_end()).unwrap();
    assert_eq!(
        unavailable,
        serde_json::json!({"type": "recipe_plan", "available": false, "plan": null})
    );
    let available: serde_json::Value =
        serde_json::from_str(render_declared_recipe_plan(Some(&plan), true, true).trim_end())
            .unwrap();
    assert_eq!(available["available"], true);
    assert_eq!(
        serde_json::from_value::<RecipePlan>(available["plan"].clone()).unwrap(),
        plan
    );
    assert_eq!(
        render_recipe_refusal(RecipeRunRefusal::FileInvalid, false),
        "recipe refused: recipe.file-invalid\n"
    );
    assert_eq!(
        render_recipe_refusal(RecipeRunRefusal::NetworkGrantUnavailable, true),
        "{\"code\":\"recipe.network-grant-unavailable\",\"type\":\"recipe_refused\"}\n"
    );
    use crate::headless::ClientExitCode;
    for (refusal, exit) in [
        (RecipeRunRefusal::FileInvalid, ClientExitCode::InvalidInput),
        (
            RecipeRunRefusal::Recipe(RecipeError::ParameterInvalid),
            ClientExitCode::InvalidInput,
        ),
        (
            RecipeRunRefusal::NetworkGrantUnavailable,
            ClientExitCode::AuthorityDenied,
        ),
    ] {
        assert_eq!(refusal.exit_code(), exit);
    }
}

#[test]
fn a_recipe_crosses_the_wire_closed_and_absent_is_not_encoded() {
    let input = crate::runtime_transport::RuntimePrepareInput {
        resume: false,
        record_session: false,
        slow_subscriber_probe: false,
        preauthorization: None,
        engineering_session_id: None,
        profile_id: "profile".to_owned(),
        expected_entry_sha256: "a".repeat(64),
        workspace_id: workspace().as_str().to_owned(),
        workspace_root: "/tmp/workspace".to_owned(),
        prompt: "Repair".to_owned(),
        recipe: None,
    };
    let encoded = serde_json::to_value(&input).unwrap();
    assert!(encoded.get("recipe").is_none());
    let with_recipe = crate::runtime_transport::RuntimePrepareInput {
        recipe: Some(sent()),
        ..input
    };
    let encoded = serde_json::to_value(&with_recipe).unwrap();
    assert_eq!(
        serde_json::from_value::<crate::runtime_transport::RuntimePrepareInput>(encoded.clone())
            .unwrap(),
        with_recipe
    );
    let mut extra = encoded;
    extra["recipe"]["granted"] = serde_json::Value::Bool(true);
    assert!(
        serde_json::from_value::<crate::runtime_transport::RuntimePrepareInput>(extra).is_err()
    );
    assert_eq!(
        crate::runtime_transport::RuntimeTransportError::RecipeDenied.code(),
        "host.runtime.recipe_denied"
    );
}

// The manifest file fixture creates, links and removes files, so it is a test
// module of its own for the effect boundary scan.
#[cfg(test)]
mod fixture {
    use std::path::{Path, PathBuf};

    pub(super) struct Directory(pub(super) PathBuf);

    impl Directory {
        pub(super) fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "agentmage-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }

        pub(super) fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).unwrap();
            path
        }

        pub(super) fn link(&self, name: &str, target: &Path) -> PathBuf {
            let path = self.0.join(name);
            std::os::unix::fs::symlink(target, &path).unwrap();
            path
        }

        pub(super) fn fifo(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            rustix::fs::mknodat(
                rustix::fs::CWD,
                path.as_path(),
                rustix::fs::FileType::Fifo,
                rustix::fs::Mode::from_raw_mode(0o600),
                0,
            )
            .unwrap();
            path
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

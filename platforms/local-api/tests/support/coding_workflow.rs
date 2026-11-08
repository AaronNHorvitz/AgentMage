// SPDX-License-Identifier: Apache-2.0
//! Executable guided client against synthetic API/model/command observations.
use super::*;
use agentmage_kernel_engine::{
    command_runner::{
        CommandBounds, CommandReceipt, CommandRegistry, CommandRequest, CommandRisk, CommandSpec,
        CommandTermination, CommandWorkingDirectory, prepare_command,
    },
    validation_result::{
        ValidationObservation, ValidationOutputClassification, normalize_validation_result,
    },
    validation_template::{
        ValidationKind, ValidationParserKind, ValidationTemplate, ValidationTemplateInput,
        ValidationTemplateSource, seal_validation_template,
    },
};
use agentmage_platform_linux::LinuxCodingReadScope;
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn template() -> ValidationTemplate {
    let command = CommandSpec::seal(
        "synthetic-tests",
        "1.0.0",
        "/usr/bin/true",
        "a".repeat(64),
        vec![],
        CommandWorkingDirectory::OwnedWorktree,
        Default::default(),
        CommandRisk::Moderate,
        CommandBounds::new(1000, 4096, 4096, 128 * 1024 * 1024, 8, 100).unwrap(),
    )
    .unwrap();
    seal_validation_template(ValidationTemplateInput {
        validation_id: "local-project-tests".into(),
        kind: ValidationKind::Unit,
        command,
        source: ValidationTemplateSource::ExplicitUserInput,
        source_sha256: "b".repeat(64),
        source_path: None,
        user_input_approval_sha256: Some("b".repeat(64)),
        execution_scope_sha256: "c".repeat(64),
        parser: ValidationParserKind::AgentMageJsonV1,
        parser_version: "1.0.0".into(),
        parser_sha256: "d".repeat(64),
        minimum_test_count: 1,
        focused: true,
        fail_fast: false,
        failed_test_rerun_allowed: true,
        setup_idempotent: true,
        expected_artifacts: vec![],
    })
    .unwrap()
}
fn definition(t: &ValidationTemplate) -> String {
    json!({"command":t.command,"kind":"unit","minimum_test_count":1,
        "approve_definition":true,"prior_receipt_sha256":null})
    .to_string()
}
fn preview(t: &ValidationTemplate, root: &str, attempt: u64, prior: Option<&str>) -> Value {
    let registry = CommandRegistry::build(vec![t.command.clone()]).unwrap();
    let prepared = prepare_command(
        &registry,
        CommandRequest::new(format!("test-{attempt}"), &t.command),
    )
    .unwrap();
    json!({"operation":"execute-validation","worktree_root_sha256":root,
        "creation_receipt_sha256":"e".repeat(64),"preview_sha256":format!("{attempt:064x}"),
        "attempt":attempt,"maximum_attempts":3,"prior_receipt_sha256":prior,
        "validation_template":t,"command":prepared.preview(),"request_sha256":prepared.request_sha256()})
}
fn result(t: &ValidationTemplate, p: &Value, status: &str) -> Value {
    let registry = CommandRegistry::build(vec![t.command.clone()]).unwrap();
    let prepared = prepare_command(
        &registry,
        CommandRequest::new(
            p["command"]["command_attempt_id"].as_str().unwrap(),
            &t.command,
        ),
    )
    .unwrap();
    let passed = matches!(status, "passed" | "zero_tests" | "flaky");
    let wire_status = if status == "zero_tests" {
        "passed"
    } else {
        status
    };
    let compile = status == "compile_failed";
    let stdout = serde_json::to_vec(&json!({"schema_version":1,"status":wire_status,
        "passed":u8::from(passed && status != "zero_tests"),"failed":u8::from(!passed && !compile),"skipped":0,
        "duration_ms":1,"failed_names":if passed || compile {vec![]}else{vec!["synthetic.test"]},
        "artifact_ids":[],"retry_count":u8::from(status == "flaky"),"initial_failure_sha256":if status == "flaky" {Some("a".repeat(64))}else{None}}))
    .unwrap();
    let mut command = CommandReceipt {
        schema_version: 1,
        authority_transaction_id: "synthetic-transaction".into(),
        operation_attempt_id: "synthetic-operation".into(),
        command_attempt_id: prepared.preview().command_attempt_id.clone(),
        template_id: t.command.template_id.clone(),
        template_version: t.command.template_version.clone(),
        spec_sha256: t.command.spec_sha256.clone(),
        request_sha256: prepared.request_sha256().into(),
        preview_sha256: prepared.preview().preview_sha256.clone(),
        termination: CommandTermination::Exited,
        outcome: if passed {
            OperationOutcome::Succeeded
        } else {
            OperationOutcome::Failed
        },
        exit_code: Some(if passed { 0 } else { 1 }),
        signal: None,
        stdout_sha256: hash(&stdout),
        stdout_total_bytes: stdout.len() as u64,
        stdout_retained_bytes: stdout.len() as u64,
        stdout_truncated: false,
        stderr_sha256: hash(b""),
        stderr_total_bytes: 0,
        stderr_retained_bytes: 0,
        stderr_truncated: false,
        elapsed_ms: 1,
        resource_usage: None,
        descendants_terminated: true,
        platform_code: "test.synthetic.workflow".into(),
        receipt_sha256: "0".repeat(64),
    };
    command.receipt_sha256 = hash(&serde_json::to_vec(&command).unwrap());
    let receipt = normalize_validation_result(
        t,
        &prepared,
        &command,
        ValidationObservation {
            validation_attempt_id: prepared.preview().command_attempt_id.clone(),
            approval_sha256: p["preview_sha256"].as_str().unwrap().into(),
            stdout,
            stderr: vec![],
            stdout_classification: ValidationOutputClassification::Restricted,
            stderr_classification: ValidationOutputClassification::Restricted,
            secret_match_count: 0,
            artifacts: vec![],
            affected_files: vec![],
            changed_paths: vec![],
            baseline_failure_sha256s: vec![],
            baseline_known_clean: false,
            unverified_kinds: vec![],
        },
    )
    .unwrap();
    json!({"outcome":command.outcome,"command_receipt":command,"validation":{
        "receipt":receipt,"validation_passed":receipt.is_full_pass(),"attempt":p["attempt"],
        "maximum_attempts":3,"prior_receipt_sha256":p["prior_receipt_sha256"]}})
}

struct Script {
    replies: Vec<Value>,
    routes: Vec<String>,
    root: String,
    selected: Value,
    settings: Value,
}

#[test]
fn guided_client_keeps_the_original_disclosure_binding_on_every_proposal() {
    let f = Fixture::new();
    let t = template();
    let mut script = Script::new(&f);
    script.start(&t);
    let prior = script.attempt(&t, 1, None, "assertion_failed");
    script.attempt(&t, 2, Some(&prior), "passed");
    let selected = script.selected.clone();
    let settings = script.settings.clone();
    let root = script.root.clone();
    let (out, calls, _) = script.run(&f, &t, "approve");
    assert_eq!(out["status"], 0, "{out}");
    let proposals = calls
        .iter()
        .filter(|call| {
            call["route"] == "POST /v1/code/workspace/proposals HTTP/1.1"
                || call["route"] == "POST /v1/code/repository/repair/proposals HTTP/1.1"
        })
        .collect::<Vec<_>>();
    assert_eq!(proposals.len(), 2);
    for call in proposals {
        let body = &call["body"];
        assert_eq!(body["workspace_revision"], selected["workspace_revision"]);
        assert_eq!(
            body["workspace_selection_id"],
            selected["workspace_selection_id"]
        );
        assert_eq!(body["settings_revision"], settings["settings_revision"]);
        assert_eq!(body["settings_endpoint"], settings["endpoint"]);
        assert_eq!(body["expected_root_sha256"], root);
    }
}

#[test]
fn guided_client_refuses_replacement_during_the_proposal_helpers_fresh_reads() {
    let t = template();
    for case in [
        "first-selection",
        "repair-selection",
        "repair-settings",
        "repair-root",
    ] {
        let f = Fixture::new();
        let mut script = Script::new(&f);
        script.start(&t);
        let prior = script.attempt(&t, 1, None, "assertion_failed");
        script.attempt(&t, 2, Some(&prior), "passed");
        let repair = script
            .routes
            .iter()
            .position(|r| r == "GET /v1/code/repository/repair HTTP/1.1")
            .unwrap();
        let index = match case {
            "first-selection" => 7,
            "repair-selection" => repair - 1,
            "repair-settings" => repair + 1,
            "repair-root" => repair,
            _ => unreachable!(),
        };
        let field = match case {
            "repair-settings" => "settings_revision",
            "repair-root" => "worktree_root_sha256",
            _ => "workspace_selection_id",
        };
        script.replies[index][field] = if case == "repair-settings" {
            json!(2)
        } else {
            json!("9".repeat(64))
        };
        let (out, calls, routes) = script.run(&f, &t, "approve");
        assert_ne!(out["status"], 0, "{case}: {out}");
        assert_eq!(calls.len(), index + 1, "{case}: {out}");
        assert_eq!(
            out["answered"],
            if case == "first-selection" { 0 } else { 3 },
            "{case}: {out}"
        );
        assert_eq!(
            calls
                .iter()
                .map(|c| c["route"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>(),
            routes[..calls.len()]
        );
        assert!(
            !calls
                .iter()
                .any(|c| c["route"] == "POST /v1/code/repository/repair/proposals HTTP/1.1")
        );
    }
}
// Keep the root and ownership receipt unchanged: only the proposed edit is
// replaced. Test both initial generation and the failure-informed repair path.
fn replaced_edit_is_refused(after_inspection: bool) {
    let t = template();
    for repair in [false, true] {
        for field in ["preview_sha256", "diff"] {
            let f = Fixture::new();
            let mut script = Script::new(&f);
            script.start(&t);
            let prior = script.attempt(&t, 1, None, "assertion_failed");
            script.attempt(&t, 2, Some(&prior), "passed");
            let prepare = script
                .routes
                .iter()
                .enumerate()
                .filter(|(_, r)| *r == "POST /v1/code/repository/edit/prepare HTTP/1.1")
                .nth(usize::from(repair))
                .unwrap()
                .0;
            let index = prepare + if after_inspection { 2 } else { 1 };
            let replacement = if field == "preview_sha256" {
                json!("9".repeat(64))
            } else {
                // Equality must cover the whole preview, even an inconsistent
                // peer response that retains the original digest field.
                json!("replacement synthetic diff")
            };
            script.replies[index]["pending"][field] = replacement.clone();
            if !after_inspection {
                // A stable replacement in both GETs used to evade the binding.
                script.replies[index + 1]["pending"][field] = replacement;
            }
            let original = script.replies[prepare].clone();
            let (out, calls, routes) = script.run(&f, &t, "approve");
            assert_ne!(out["status"], 0, "repair={repair}, field={field}: {out}");
            assert_eq!(calls.len(), index + 1, "{out}");
            assert_eq!(out["answered"], if repair { 5 } else { 1 }, "{out}");
            assert_eq!(
                calls
                    .iter()
                    .map(|c| c["route"].as_str().unwrap().to_owned())
                    .collect::<Vec<_>>(),
                routes[..calls.len()],
                "{out}"
            );
            let edits = calls
                .iter()
                .filter(|c| c["route"] == "POST /v1/code/repository/edit/decide HTTP/1.1")
                .count();
            let tests = calls
                .iter()
                .filter(|c| c["route"] == "POST /v1/code/repository/validation/decide HTTP/1.1")
                .count();
            assert_eq!(edits, usize::from(repair), "{out}");
            assert_eq!(tests, usize::from(repair), "{out}");
            assert!(
                !out["transcript"]
                    .as_str()
                    .unwrap()
                    .contains("replacement synthetic diff")
            );
            assert_eq!(original["operation"], "edit-worktree");
            assert_eq!(
                fs::read_to_string(f.root.join("tree/calc.py")).unwrap(),
                "return a - b\n"
            );
        }
    }
}

#[test]
fn guided_client_refuses_edit_replacement_before_the_first_inspection() {
    replaced_edit_is_refused(false);
}

#[test]
fn guided_client_refuses_edit_replacement_before_the_approval_inspection() {
    replaced_edit_is_refused(true);
}

impl Script {
    fn new(f: &Fixture) -> Self {
        fs::create_dir(f.root.join("tree")).unwrap();
        fs::write(f.root.join("tree/calc.py"), "return a - b\n").unwrap();
        let scope =
            LinuxCodingReadScope::select(&f.root.join("tree"), &["calc.py".into()]).unwrap();
        Self {
            replies: vec![],
            routes: vec![],
            root: scope.root_sha256().into(),
            selected: json!({"selected":true,"model_context_approved":true,"workspace_revision":1,"workspace_selection_id":"synthetic-selection","paths":["calc.py"]}),
            settings: json!({"settings_revision":1,"endpoint":"http://127.0.0.1:4321/v1"}),
        }
    }
    fn push(&mut self, route: &str, reply: Value) {
        self.routes.push(format!("{route} HTTP/1.1"));
        self.replies.push(reply);
    }
    fn start(&mut self, t: &ValidationTemplate) {
        self.push("GET /v1/code/workspace", self.selected.clone());
        self.push("GET /v1/settings", self.settings.clone());
        self.push(
            "GET /v1/code/repository/edit",
            json!({"worktree_root_sha256":self.root,"pending":null}),
        );
        self.push(
            "GET /v1/code/repository/validation",
            json!({"attempts":0,"maximum_attempts":3,"latest":null,"pending":null}),
        );
        self.push(
            "POST /v1/code/repository/validation/prepare",
            preview(t, &self.root, 1, None),
        );
    }
    fn attempt(
        &mut self,
        t: &ValidationTemplate,
        n: u64,
        prior: Option<&str>,
        status: &str,
    ) -> String {
        self.push("GET /v1/code/workspace", self.selected.clone());
        self.push("GET /v1/settings", self.settings.clone());
        self.push("GET /v1/code/workspace", self.selected.clone());
        if let Some(prior) = prior {
            self.push("GET /v1/code/repository/repair",json!({"worktree_root_sha256":self.root,"receipt_sha256":prior,"feedback_sha256":"f".repeat(64),"output":{"stderr":"untrusted synthetic failure\u{1b}[2J"}}));
            self.push("GET /v1/settings", self.settings.clone());
        }
        let answer = "{\"edits\":[{\"path\":\"calc.py\",\"old\":\"a - b\",\"new\":\"a + b\"}]}";
        let feedback = self
            .replies
            .iter()
            .rev()
            .find(|v| v.get("feedback_sha256").is_some())
            .cloned();
        let mut context = json!({"type":"context","workspace_revision":1,"files":[{"path":"calc.py","sha256":hash(b"return a - b\n")}],"message_bytes":3000,"source_message_bytes":2500});
        if prior.is_some() {
            context["repair_feedback"] = feedback.unwrap();
        }
        self.push(if prior.is_some() {"POST /v1/code/repository/repair/proposals"}else{"POST /v1/code/workspace/proposals"},json!({"test_peer_stream":[context,{"type":"text","text":answer},{"type":"proposal","workspace_revision":1,"applied":false,"tested":false}]}));
        self.push("GET /v1/status", json!({"busy":false}));
        self.push(
            "GET /v1/code/repository/edit",
            json!({"worktree_root_sha256":self.root}),
        );
        let edit = json!({"operation":"edit-worktree","worktree_root_sha256":self.root,"creation_receipt_sha256":"e".repeat(64),"preview_sha256":format!("{n:064x}"),"diff":"synthetic diff only"});
        self.push("POST /v1/code/repository/edit/prepare", edit.clone());
        self.push("GET /v1/code/repository/edit", json!({"pending":edit}));
        self.push("GET /v1/code/repository/edit", json!({"pending":edit}));
        self.push(
            "POST /v1/code/repository/edit/decide",
            json!({"outcome":"committed"}),
        );
        self.push("GET /v1/status", json!({"busy":false}));
        let p = preview(t, &self.root, n, prior);
        let result = result(t, &p, status);
        let receipt = result["validation"]["receipt"]["receipt_sha256"]
            .as_str()
            .unwrap()
            .to_owned();
        self.push("POST /v1/code/repository/validation/prepare", p.clone());
        self.push("GET /v1/code/repository/validation", json!({"pending":p}));
        self.push("POST /v1/code/repository/validation/decide", result);
        if status != "passed" && n < 3 {
            self.push("GET /v1/status", json!({"busy":false}));
        }
        receipt
    }
    fn run(
        self,
        f: &Fixture,
        t: &ValidationTemplate,
        answer: &str,
    ) -> (Value, Vec<Value>, Vec<String>) {
        let peer = f.peer(self.replies);
        let started = Instant::now();
        let output = f.tty(
            peer.address,
            &[
                "code-worktree",
                f.root.join("tree").to_str().unwrap(),
                "synthetic-model",
                "4096",
                "128",
                "Fix addition",
                &definition(t),
            ],
            answer,
        );
        eprintln!(
            "synthetic guided-worktree client elapsed_us={}",
            started.elapsed().as_micros()
        );
        (output, peer.finish(), self.routes)
    }
}

#[test]
fn guided_client_connects_proposals_exact_edits_tests_and_bounded_repairs() {
    for statuses in [
        vec!["passed"],
        vec!["assertion_failed", "passed"],
        vec!["compile_failed", "assertion_failed", "passed"],
        vec!["assertion_failed"; 3],
    ] {
        let f = Fixture::new();
        let t = template();
        let mut script = Script::new(&f);
        script.start(&t);
        let mut prior = None;
        for (i, status) in statuses.iter().enumerate() {
            prior = Some(script.attempt(&t, i as u64 + 1, prior.as_deref(), status));
        }
        let (out, calls, routes) = script.run(&f, &t, "approve");
        assert_eq!(
            out["status"] == 0,
            statuses.last() == Some(&"passed"),
            "{out}"
        );
        assert_eq!(
            calls
                .iter()
                .map(|c| c["route"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>(),
            routes,
            "{out}"
        );
        assert_eq!(out["answered"], 3 * statuses.len() + statuses.len() - 1);
        let mut previous = None;
        for call in &calls {
            if call["route"] == "POST /v1/code/repository/validation/decide HTTP/1.1" {
                assert_eq!(call["body"]["approve"], true);
            }
            if call["route"] == "POST /v1/code/repository/validation/prepare HTTP/1.1" {
                let mut body = call["body"].clone();
                body["prior_receipt_sha256"] = Value::Null;
                assert_eq!(
                    body,
                    serde_json::from_str::<Value>(&definition(&t)).unwrap()
                );
            }
            if call["route"] == "POST /v1/code/repository/repair/proposals HTTP/1.1" {
                assert_eq!(call["body"]["approve_failure_disclosure"], true);
                assert_ne!(previous, Some(call["body"]["receipt_sha256"].clone()));
                previous = Some(call["body"]["receipt_sha256"].clone());
            }
        }
        assert!(
            out["transcript"]
                .as_str()
                .unwrap()
                .contains("independent_task_grade")
        );
        // The recording peer never executes model/tool work or applies a diff.
        assert_eq!(
            fs::read_to_string(f.root.join("tree/calc.py")).unwrap(),
            "return a - b\n"
        );
    }
}

#[test]
fn guided_client_stops_on_denial_preflight_loss_and_effect_reply_loss() {
    let t = template();
    for failure in [
        "denial",
        "preflight",
        "edit-loss",
        "test-loss",
        "bad-receipt",
        "swapped-preview",
        "changed-selection",
    ] {
        let f = Fixture::new();
        let mut script = Script::new(&f);
        script.start(&t);
        script.attempt(&t, 1, None, "passed");
        let (route, reply) = match failure {
            "preflight" => (
                "POST /v1/code/repository/validation/prepare",
                json!({"test_peer_status":409,"body":{"error":"linux.command.manifest.invalid"}}),
            ),
            "edit-loss" => (
                "POST /v1/code/repository/edit/decide",
                json!({"test_peer_close":true}),
            ),
            "test-loss" => (
                "POST /v1/code/repository/validation/decide",
                json!({"test_peer_close":true}),
            ),
            "bad-receipt" => {
                let i = script
                    .routes
                    .iter()
                    .position(|r| r.starts_with("POST /v1/code/repository/validation/decide "))
                    .unwrap();
                let mut r = script.replies[i].clone();
                r["validation"]["receipt"]["passed"] = json!(99);
                ("POST /v1/code/repository/validation/decide", r)
            }
            "changed-selection" => (
                "GET /v1/code/workspace",
                json!({"selected":true,"model_context_approved":true,"workspace_revision":2,"paths":["other.py"]}),
            ),
            "swapped-preview" => (
                "GET /v1/code/repository/validation",
                json!({"pending":{"operation":"execute-validation","preview_sha256":"9".repeat(64)}}),
            ),
            _ => ("", Value::Null),
        };
        if !route.is_empty() {
            let index = if failure == "changed-selection" {
                5
            } else {
                script
                    .routes
                    .iter()
                    .position(|r| r == &format!("{route} HTTP/1.1"))
                    .unwrap()
            };
            script.replies[index] = reply;
        }
        let (out, calls, routes) =
            script.run(&f, &t, if failure == "denial" { "no" } else { "approve" });
        assert_ne!(out["status"], 0, "{failure}: {out}");
        assert_eq!(
            calls
                .iter()
                .map(|c| c["route"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>(),
            routes[..calls.len()],
            "{failure}: {out}"
        );
        for route in [
            "POST /v1/code/repository/edit/decide HTTP/1.1",
            "POST /v1/code/repository/validation/decide HTTP/1.1",
        ] {
            assert!(calls.iter().filter(|c| c["route"] == route).count() <= 1);
        }
        assert!(
            !calls
                .iter()
                .any(|c| c["route"] == "POST /v1/code/repository/repair/proposals HTTP/1.1")
        );
        if failure == "preflight" {
            assert_eq!(calls.len(), 5);
            assert_eq!(out["answered"], 0);
        }
        if failure == "denial" {
            assert_eq!(out["answered"], 1);
            assert!(
                !calls
                    .iter()
                    .any(|c| c["route"] == "POST /v1/code/repository/edit/prepare HTTP/1.1")
            );
        }
    }
}

#[test]
fn guided_client_refuses_piped_consent_and_invalid_definitions_without_requests() {
    let f = Fixture::new();
    let t = template();
    let mut script = Script::new(&f);
    script.start(&t);
    let peer = f.peer(vec![]);
    let root = f.root.join("tree");
    let definition_text = definition(&t);
    let args = [
        "code-worktree",
        root.to_str().unwrap(),
        "synthetic-model",
        "4096",
        "128",
        "Fix addition",
        &definition_text,
    ];
    let out = f.cli(peer.address, &args);
    assert!(!out.status.success());
    assert!(peer.finish().is_empty());
    for text in [
        "[]".into(),
        definition(&t).replace(
            "\"approve_definition\":true",
            "\"approve_definition\":false",
        ),
        definition(&t).replace(
            "\"prior_receipt_sha256\":null",
            "\"prior_receipt_sha256\":\"stale\"",
        ),
        definition(&t).replacen('{', "{\"approve_definition\":false,", 1),
    ] {
        let peer = f.peer(vec![]);
        let out = f.tty(
            peer.address,
            &[
                "code-worktree",
                f.root.join("tree").to_str().unwrap(),
                "synthetic-model",
                "4096",
                "128",
                "Fix addition",
                &text,
            ],
            "approve",
        );
        assert_ne!(out["status"], 0, "{out}");
        assert_eq!(out["answered"], 0);
        assert!(peer.finish().is_empty());
    }
}

#[test]
fn guided_client_preserves_every_consent_and_refuses_nonretryable_test_results() {
    let t = template();
    for status in ["zero_tests", "malformed", "flaky"] {
        let f = Fixture::new();
        let mut script = Script::new(&f);
        script.start(&t);
        script.attempt(&t, 1, None, status);
        let (out, calls, routes) = script.run(&f, &t, "approve");
        assert_ne!(out["status"], 0, "{status}: {out}");
        assert_eq!(out["answered"], 3);
        assert_eq!(
            calls
                .iter()
                .map(|c| c["route"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>(),
            routes[..calls.len()]
        );
        assert_eq!(
            calls.last().unwrap()["route"],
            "POST /v1/code/repository/validation/decide HTTP/1.1"
        );
    }
    for denied in 0..7 {
        let f = Fixture::new();
        let mut script = Script::new(&f);
        script.start(&t);
        let prior = script.attempt(&t, 1, None, "assertion_failed");
        script.attempt(&t, 2, Some(&prior), "passed");
        let (out, calls, routes) = script.run(&f, &t, &format!("deny-at-{denied}"));
        assert_ne!(out["status"], 0, "denied {denied}: {out}");
        assert_eq!(out["answered"], denied + 1);
        assert_eq!(
            calls
                .iter()
                .map(|c| c["route"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>(),
            routes[..calls.len()]
        );
        let expected_last = match denied {
            0 | 4 => "GET /v1/code/repository/edit HTTP/1.1",
            1 | 5 => "GET /v1/code/repository/edit HTTP/1.1",
            2 | 6 => "GET /v1/code/repository/validation HTTP/1.1",
            3 => "GET /v1/settings HTTP/1.1",
            _ => unreachable!(),
        };
        assert_eq!(
            calls.last().unwrap()["route"],
            expected_last,
            "{denied}: {out}"
        );
    }
}

#[test]
fn guided_client_pins_clean_series_command_scope_and_exact_failure_before_disclosure() {
    let t = template();
    for case in [
        "old-series",
        "pending-edit",
        "wrong-root",
        "wrong-command",
        "wrong-definition",
        "settings-drift",
        "stale-failure",
        "new-creation",
    ] {
        let f = Fixture::new();
        let mut script = Script::new(&f);
        script.start(&t);
        let prior = script.attempt(&t, 1, None, "assertion_failed");
        script.attempt(&t, 2, Some(&prior), "passed");
        let (index, field, value) = match case {
            "old-series" => (3, "attempts", json!(1)),
            "pending-edit" => (2, "pending", json!({"operation":"edit-worktree"})),
            "wrong-root" => (4, "worktree_root_sha256", json!("9".repeat(64))),
            "wrong-definition" => (
                4,
                "validation_template",
                json!({"command":"not a template"}),
            ),
            "wrong-command" => (4, "command", json!({"command_attempt_id":"other"})),
            "settings-drift" => (6, "settings_revision", json!(2)),
            "stale-failure" => (
                script
                    .routes
                    .iter()
                    .position(|r| r == "GET /v1/code/repository/repair HTTP/1.1")
                    .unwrap(),
                "receipt_sha256",
                json!("9".repeat(64)),
            ),
            "new-creation" => {
                let index = script
                    .routes
                    .iter()
                    .position(|r| r == "POST /v1/code/repository/edit/prepare HTTP/1.1")
                    .unwrap()
                    + 1;
                script.replies[index]["pending"]["creation_receipt_sha256"] = json!("9".repeat(64));
                (index, "version", json!(1))
            }
            _ => unreachable!(),
        };
        script.replies[index][field] = value;
        let (out, calls, routes) = script.run(&f, &t, "approve");
        assert_ne!(out["status"], 0, "{case}: {out}");
        assert_eq!(
            calls
                .iter()
                .map(|c| c["route"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>(),
            routes[..calls.len()],
            "{case}: {out}"
        );
        assert_eq!(calls.len(), index + 1, "{case}: {out}");
        assert!(
            !calls
                .iter()
                .any(|c| c["route"] == "POST /v1/code/repository/repair/proposals HTTP/1.1")
        );
        if case == "stale-failure" {
            assert_eq!(out["answered"], 3, "{out}");
        }
        if case == "new-creation" {
            assert_eq!(out["answered"], 1, "{out}");
        }
    }
}

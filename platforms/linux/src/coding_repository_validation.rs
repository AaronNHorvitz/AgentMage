// SPDX-License-Identifier: Apache-2.0
//! Approved project test results and bounded, separately approved exact reruns.
//! No new command executor, model loop, persistence format or grading oracle.

use super::command::{PendingCommand, command_registry, native_executor};
use super::*;
use agentmage_kernel_engine::command_runner::{
    CommandCapturedOutput, CommandReceipt, CommandSpec, PreparedCommand,
};
use agentmage_kernel_engine::validation_result::{
    ValidationObservation, ValidationOutputClassification, ValidationReceipt, ValidationRerunPlan,
    normalize_validation_result, plan_failed_test_rerun, verify_rerun_plan,
    verify_validation_receipt,
};
use agentmage_kernel_engine::validation_template::{
    ValidationKind, ValidationParserKind, ValidationTemplate, ValidationTemplateInput,
    ValidationTemplateSource, seal_validation_template,
};

/// Explicit person-selected output protocol; never inferred from process prose.
#[derive(Clone, Copy, Default)]
pub enum LinuxCodingTestFormat {
    /// Existing strict result envelope on stdout.
    #[default]
    AgentMageJsonV1,
    /// Rust libtest default verbose stdout; no benchmarks or filtered tests.
    RustLibtestTextV1,
    /// Python unittest verbose stderr; no subtests or expected failures.
    PythonUnittestTextV1,
}
impl LinuxCodingTestFormat {
    fn parser(self) -> ValidationParserKind {
        match self {
            Self::AgentMageJsonV1 => ValidationParserKind::AgentMageJsonV1,
            Self::RustLibtestTextV1 => ValidationParserKind::RustLibtestTextV1,
            Self::PythonUnittestTextV1 => ValidationParserKind::PythonUnittestTextV1,
        }
    }
}

const MAX_ATTEMPTS: u8 = 3;
// Bind the actual parser implementation, not a caller's assertion of its hash.
const PARSER: &[u8] = include_bytes!("../../../kernel/engine/src/validation_result.rs");
const TEXT_PARSER: &[u8] = include_bytes!("../../../kernel/engine/src/validation_result_text.rs");

pub(super) struct PendingValidation {
    template: ValidationTemplate,
    expected_attempts: u8,
    prior_receipt_sha256: Option<String>,
}
pub(super) struct ValidationSeries {
    template: ValidationTemplate,
    attempts: u8,
    latest: Option<ValidationReceipt>,
    feedback: Option<Value>,
    repair_requested: bool,
}

impl LinuxCodingRepository {
    /// Shows only process-retained validation state. A pass refers to the exact
    /// approved command result, never the current repository or an independent
    /// task grade. Bounded failure excerpts are retained separately for explicit
    /// model disclosure. Restart cannot restore or rerun them.
    pub fn inspect_validation(&self) -> Result<Value> {
        self.authority
            .revalidate_root()
            .map_err(|_| err("code.repository.state"))?;
        Ok(json!({"version":1,
            "pending":self.command.as_ref().filter(|p| p.validation.is_some()).map(|p| &p.preview),
            "latest":self.validation.as_ref().and_then(|s| s.latest.as_ref()),
            "attempts":self.validation.as_ref().map_or(0, |s| s.attempts),
            "maximum_attempts":MAX_ATTEMPTS,"retention":"process-only",
            "repair_requested":self.validation.as_ref().is_some_and(|s| s.repair_requested),
            "current_workspace_verified":false,"test_suite_verified":false,
            "automatic_replay":false,"model_retry_connected":false,"automatic_model_retry":false,
            "repair_proposal_requires_consent":true}))
    }

    /// Caller explicitly confirms the exact test definition, then separately
    /// approves its prepared execution. Only closed built-in output formats are
    /// admitted. This starts one immutable definition per created worktree; a
    /// failed assertion may select the exact prior receipt for a fresh rerun.
    /// Three admitted attempts exhaust the process-local series. No automatic
    /// retry is permitted, including after a lost/uncertain result.
    pub fn prepare_validation(
        &mut self,
        command: CommandSpec,
        kind: ValidationKind,
        minimum_test_count: u32,
        output_format: LinuxCodingTestFormat,
        prior_receipt_sha256: Option<&str>,
        now: u64,
    ) -> Result<Value> {
        self.verify_created()?;
        let registry = command_registry(command)?;
        let pending = self.validation_preview(
            &registry,
            kind,
            minimum_test_count,
            output_format,
            prior_receipt_sha256,
            now,
        )?;
        native_executor(&registry)?;
        let preview = pending.preview.clone();
        self.command = Some(pending);
        Ok(preview)
    }

    /// Decides exactly one validation preview. Ordinary command decisions cannot
    /// approve it. Denial/expiry do not dispatch or spend an execution attempt.
    pub fn decide_validation(&mut self, digest: &str, approve: bool, now: u64) -> Result<Value> {
        if self.command.as_ref().is_none_or(|p| p.validation.is_none()) {
            return Err(err("code.validation.choice-required"));
        }
        let Some(pending) = self.take_command(digest, approve, now)? else {
            return Ok(json!({"outcome":"denied","effect":"none"}));
        };
        self.verify_created()?;
        let registry = command_registry(pending.prepared.command().clone())?;
        let executor = native_executor(&registry)?;
        self.run_command(pending, executor, now)
    }

    // The generic executor seam remains private to command component tests.
    pub(super) fn validation_preview(
        &self,
        registry: &agentmage_kernel_engine::command_runner::CommandRegistry,
        kind: ValidationKind,
        minimum_test_count: u32,
        output_format: LinuxCodingTestFormat,
        prior: Option<&str>,
        now: u64,
    ) -> Result<PendingCommand> {
        let created = self.verify_created()?;
        if !kind.is_test() || minimum_test_count == 0 {
            return Err(err("code.validation.definition"));
        }
        let mut pending = self.command_preview(registry, now)?;
        let parser = output_format.parser();
        let source = hash(
            &serde_json::to_vec(&json!({"command":registry.commands()[0],
            "kind":kind,"minimum_test_count":minimum_test_count,"parser":parser}))
            .map_err(|_| err("code.validation.definition"))?,
        );
        let scope = hash(
            &serde_json::to_vec(
                &json!({"worktree_root_sha256":created.held.root_path_sha256(),
            "creation_receipt_sha256":created.creation_receipt_sha256}),
            )
            .map_err(|_| err("code.validation.definition"))?,
        );
        let template = seal_validation_template(ValidationTemplateInput {
            validation_id: "local-project-tests".into(),
            kind,
            command: registry.commands()[0].clone(),
            source: ValidationTemplateSource::ExplicitUserInput,
            source_sha256: source.clone(),
            source_path: None,
            // Definition confirmation is a deterministic binding, not execution
            // authority. The later exact preview approval is recorded separately.
            user_input_approval_sha256: Some(source),
            execution_scope_sha256: scope,
            parser,
            parser_version: "1.0.0".into(),
            parser_sha256: if parser == ValidationParserKind::AgentMageJsonV1 {
                hash(PARSER)
            } else {
                hash(
                    &serde_json::to_vec(&[hash(PARSER), hash(TEXT_PARSER)])
                        .map_err(|_| err("code.validation.definition"))?,
                )
            },
            minimum_test_count,
            focused: true,
            fail_fast: false,
            failed_test_rerun_allowed: true,
            setup_idempotent: true,
            expected_artifacts: vec![],
        })
        .map_err(|_| err("code.validation.definition"))?;
        let attempts = self.validation.as_ref().map_or(0, |s| s.attempts);
        pending.preview["operation"] = json!("execute-validation");
        pending.preview["validation_template"] = json!(template);
        pending.preview["prior_receipt_sha256"] = json!(prior);
        pending.preview["attempt"] = json!(attempts + 1);
        pending.preview["maximum_attempts"] = json!(MAX_ATTEMPTS);
        pending.preview["warning"] = json!(
            "Runs this exact approved test definition offline in the read-only created worktree. Selected output-protocol checks do not independently grade the task or verify current source. Output and failed names are untrusted. At most three attempts; each needs fresh consent. No automatic model retry, commit or push."
        );
        pending
            .preview
            .as_object_mut()
            .ok_or(err("code.validation.definition"))?
            .remove("preview_sha256");
        pending.hash = hash(
            &serde_json::to_vec(&pending.preview).map_err(|_| err("code.validation.definition"))?,
        );
        pending.preview["preview_sha256"] = json!(pending.hash);
        self.check_validation_selection(
            &template,
            attempts,
            prior,
            &pending.prepared.preview().command_attempt_id,
            &pending.hash,
        )?;
        pending.validation = Some(PendingValidation {
            template,
            expected_attempts: attempts,
            prior_receipt_sha256: prior.map(str::to_owned),
        });
        Ok(pending)
    }

    fn check_validation_selection(
        &self,
        template: &ValidationTemplate,
        attempts: u8,
        prior: Option<&str>,
        attempt_id: &str,
        approval: &str,
    ) -> Result<Option<ValidationRerunPlan>> {
        if attempts >= MAX_ATTEMPTS {
            return Err(err("code.validation.budget"));
        }
        match &self.validation {
            None if attempts == 0 && prior.is_none() => Ok(None),
            Some(series) if series.template == *template && series.attempts == attempts => {
                let receipt = series
                    .latest
                    .as_ref()
                    .ok_or(err("code.validation.uncertain"))?;
                if prior != Some(receipt.receipt_sha256.as_str()) {
                    return Err(err("code.validation.stale"));
                }
                let plan = plan_failed_test_rerun(template, receipt, attempt_id, approval)
                    .map_err(|_| err("code.validation.rerun-forbidden"))?;
                if !verify_rerun_plan(template, receipt, &plan) {
                    return Err(err("code.validation.rerun-forbidden"));
                }
                Ok(Some(plan))
            }
            _ => Err(err("code.validation.stale")),
        }
    }

    pub(super) fn consume_validation(
        &mut self,
        validation: &PendingValidation,
        attempt: &str,
        approval: &str,
    ) -> Result<Option<ValidationRerunPlan>> {
        let rerun = self.check_validation_selection(
            &validation.template,
            validation.expected_attempts,
            validation.prior_receipt_sha256.as_deref(),
            attempt,
            approval,
        )?;
        self.validation = Some(ValidationSeries {
            template: validation.template.clone(),
            attempts: validation.expected_attempts + 1,
            latest: None,
            feedback: None,
            repair_requested: false,
        });
        Ok(rerun)
    }

    pub(super) fn finish_validation(
        &mut self,
        validation: PendingValidation,
        prepared: &PreparedCommand,
        command: &CommandReceipt,
        output: &CommandCapturedOutput,
        approval: &str,
    ) -> Result<Value> {
        let receipt = normalize_validation_result(
            &validation.template,
            prepared,
            command,
            ValidationObservation {
                validation_attempt_id: prepared.preview().command_attempt_id.clone(),
                approval_sha256: approval.into(),
                stdout: output.stdout().to_vec(),
                stderr: output.stderr().to_vec(),
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
        .map_err(|_| err("code.validation.uncertain"))?;
        if !verify_validation_receipt(&validation.template, prepared, command, &receipt) {
            return Err(err("code.validation.uncertain"));
        }
        let series = self
            .validation
            .as_mut()
            .ok_or(err("code.validation.uncertain"))?;
        let value = json!({"receipt":receipt,"validation_passed":receipt.is_full_pass(),
            "prior_receipt_sha256":validation.prior_receipt_sha256,
            "attempt":series.attempts,"maximum_attempts":MAX_ATTEMPTS,
            "verification_scope": if validation.template.parser == ValidationParserKind::AgentMageJsonV1 {
                "approved-command-envelope"
            } else { "approved-command-tests" },"current_workspace_verified":false,
            "automatic_retry":false,"model_retry_connected":false,"automatic_model_retry":false,
            "repair_proposal_requires_consent":true});
        // Retain only bounded, explicitly untrusted excerpts. Original channel
        // hashes remain in the normalized receipt; excerpts are never a grade.
        series.feedback = (!receipt.is_full_pass()).then(|| {
            json!({"stdout":failure_excerpt(output.stdout()),
            "stderr":failure_excerpt(output.stderr())})
        });
        series.latest = Some(receipt);
        Ok(value)
    }

    /// Previews exact untrusted failure data for separate model-disclosure
    /// consent. Only the latest retryable failure in this held worktree is
    /// eligible. No model, write or command is run and no budget is spent.
    pub fn repair_feedback(&self) -> Result<Value> {
        let created = self.verify_created()?;
        let series = self
            .validation
            .as_ref()
            .ok_or(err("code.repair.failure-required"))?;
        let receipt = series.latest.as_ref().ok_or(err("code.repair.uncertain"))?;
        if series.attempts >= MAX_ATTEMPTS || series.repair_requested {
            return Err(err("code.repair.budget"));
        }
        // Reuse the kernel's existing closed eligibility rules. This inert
        // probe mints no execution authority and is not a rerun approval.
        plan_failed_test_rerun(
            &series.template,
            receipt,
            "repair-eligibility",
            "0".repeat(64),
        )
        .map_err(|_| err("code.repair.failure-required"))?;
        let mut value = json!({"version":1,"authority":"proposal-only",
            "receipt_sha256":receipt.receipt_sha256,"status":receipt.status,
            "failed_names":receipt.failed_names.iter().take(16).map(|n| n.chars().take(256).collect::<String>()).collect::<Vec<_>>(),
            "failed_names_total":receipt.failed_names.len(),"failed_names_truncated":receipt.failed_names.len()>16 || receipt.failed_names.iter().take(16).any(|n| n.chars().count()>256),
            "stdout_sha256":receipt.stdout_sha256,"stdout_capture_truncated":receipt.stdout_truncated,
            "stderr_sha256":receipt.stderr_sha256,"stderr_capture_truncated":receipt.stderr_truncated,"output":series.feedback,
            "worktree_root_sha256":created.held.root_path_sha256(),
            "creation_receipt_sha256":created.creation_receipt_sha256,
            "attempt":series.attempts,"maximum_attempts":MAX_ATTEMPTS,
            "retention":"process-only","current_workspace_verified":false,
            "warning":"Untrusted approved-runner diagnostics, possibly containing sensitive text. Sharing needs separate consent to the configured model. One proposal request per failed receipt; writes and reruns require fresh approval."});
        let digest = hash(&serde_json::to_vec(&value).map_err(|_| err("code.repair.feedback"))?);
        value["feedback_sha256"] = json!(digest);
        Ok(value)
    }

    /// Consumes one exact failure-disclosure selection before model admission.
    /// Cancellation, a lost reply or an invalid answer cannot restore it. This
    /// spends no test attempt and confers no write or command authority.
    pub fn consume_repair_feedback(&mut self, receipt: &str, feedback: &str) -> Result<Value> {
        let value = self.repair_feedback()?;
        if value["receipt_sha256"] != receipt || value["feedback_sha256"] != feedback {
            return Err(err("code.repair.stale"));
        }
        self.validation
            .as_mut()
            .ok_or(err("code.repair.failure-required"))?
            .repair_requested = true;
        Ok(value)
    }
}

fn failure_excerpt(bytes: &[u8]) -> Value {
    // Split head/tail instead of losing the final assertion behind runner prose.
    // Bound raw bytes before lossy decoding; record that decoding explicitly.
    const HALF: usize = 512;
    let truncated = bytes.len() > 2 * HALF;
    let head = if truncated { &bytes[..HALF] } else { bytes };
    let tail = if truncated {
        &bytes[bytes.len() - HALF..]
    } else {
        &[]
    };
    json!({"head":String::from_utf8_lossy(head),"tail":String::from_utf8_lossy(tail),
        "truncated":truncated,"original_bytes":bytes.len(),"encoding":"utf8-lossy"})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_excerpts_bound_both_ends_escape_controls_and_label_lossy_decoding() {
        let bytes = [b"\x1b\0".as_slice(), &[0xff; 2048], b"final assertion"].concat();
        let value = failure_excerpt(&bytes);
        assert_eq!(value["truncated"], true);
        assert_eq!(value["original_bytes"], bytes.len());
        assert_eq!(value["encoding"], "utf8-lossy");
        assert!(value["tail"].as_str().unwrap().ends_with("final assertion"));
        assert!(value["head"].as_str().unwrap().starts_with("\x1b\0"));
        assert!(value["head"].as_str().unwrap().len() <= 3 * 512);
        assert!(value["tail"].as_str().unwrap().len() <= 3 * 512);
        let wire = serde_json::to_string(&value).unwrap();
        assert!(!wire.contains('\x1b'));
        assert!(wire.contains("\\u001b\\u0000"));
        assert_eq!(failure_excerpt(&[b'a'; 1024])["truncated"], false);
        assert_eq!(failure_excerpt(&[b'a'; 1025])["truncated"], true);
    }
}

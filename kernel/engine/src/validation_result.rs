//! Strict validation-result normalization, evidence receipts, and rerun plans.

use std::collections::BTreeSet;
use std::fmt::Write;

use agentmage_kernel_contracts::{OperationOutcome, WorkspacePath};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::command_runner::{
    CommandReceipt, CommandTermination, CommandWorkingDirectory, PreparedCommand,
    verify_command_receipt,
};
use crate::validation_template::{
    ValidationKind, ValidationParserKind, ValidationTemplate, verify_validation_template,
};

const VALIDATION_RESULT_SCHEMA_VERSION: u16 = 1;
const MAX_RESULT_BYTES: usize = 256 * 1024;
const MAX_FAILED_NAMES: usize = 2_048;
const MAX_ARTIFACTS: usize = 64;
const MAX_AFFECTED_FILES: usize = 10_000;
const MAX_UNVERIFIED_KINDS: usize = 9;

/// Stable reason validation evidence cannot be normalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationResultError {
    /// The template, command, receipt, or approval identity is stale or inconsistent.
    EvidenceMismatch,
    /// Retained process evidence does not match the terminal command receipt.
    OutputMismatch,
    /// Independently observed artifacts or affected files are malformed.
    ObservationInvalid,
    /// A terminal validation receipt could not be constructed.
    ReceiptFailure,
    /// A rerun is not allowed for this exact template and prior result.
    RerunForbidden,
}

impl ValidationResultError {
    /// Stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EvidenceMismatch => "validation-result.evidence-mismatch",
            Self::OutputMismatch => "validation-result.output-mismatch",
            Self::ObservationInvalid => "validation-result.observation-invalid",
            Self::ReceiptFailure => "validation-result.receipt-failure",
            Self::RerunForbidden => "validation-result.rerun-forbidden",
        }
    }
}

/// Security classification applied by a trusted output scanner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationOutputClassification {
    /// Safe public fixture or intentionally public output.
    Public,
    /// Ordinary local diagnostic output.
    Internal,
    /// Sensitive output whose raw bytes remain outside the receipt.
    Restricted,
    /// A trusted scanner observed one or more secret patterns.
    SecretDetected,
}

/// Closed non-conflated normalized result states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    /// Process, parser, test count, and artifacts all passed.
    Passed,
    /// One or more test assertions failed.
    AssertionFailed,
    /// Compilation failed before tests could complete.
    CompileFailed,
    /// Infrastructure or harness setup failed.
    InfrastructureFailed,
    /// The wall-clock ceiling terminated the process tree.
    TimedOut,
    /// Cancellation terminated or prevented the process tree.
    Cancelled,
    /// A signal or equivalent abnormal termination was observed.
    Crashed,
    /// A separately approved rerun passed after an exact prior failure.
    Flaky,
    /// No test ran and one or more tests were skipped.
    SkippedOnly,
    /// Strict parser syntax or semantic validation failed.
    Malformed,
    /// Process output crossed a configured bound.
    Truncated,
    /// A test command reported success without executing the minimum test count.
    ZeroTests,
    /// Required artifact or other independent postcondition evidence is absent.
    Unverified,
    /// Sensitive output prevented result interpretation.
    SensitiveOutput,
}

/// Completeness of the requested validation-kind set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationCoverageStatus {
    /// Every requested kind was run and normalized.
    Complete,
    /// One or more requested kinds remains explicitly unverified.
    Partial,
}

/// Deterministic failure-attribution family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationFailureClassification {
    /// Failure evidence intersects the approved changed path set.
    ChangeCaused,
    /// Exact failure signature existed in trusted baseline evidence.
    Baseline,
    /// A separately approved exact rerun passed after failure.
    Flaky,
    /// Dependency resolution or dependency availability failed.
    Dependency,
    /// Runner, sandbox, toolchain, or host environment failed.
    Environment,
    /// A permission boundary denied the check.
    Permission,
    /// Trusted clean-baseline evidence and path evidence establish no relation to the change.
    Unrelated,
    /// Available evidence cannot truthfully attribute the failure.
    Unclassified,
}

/// Independently observed output artifact.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedValidationArtifact {
    /// Stable expected artifact identity.
    pub artifact_id: String,
    /// Exact held workspace path.
    pub path: WorkspacePath,
    /// Exact observed byte digest.
    pub sha256: String,
    /// Exact observed byte count.
    pub byte_length: u64,
    /// Whether the trusted platform observed this state after process termination.
    pub observed_after_command: bool,
}

/// Independently observed changed file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationAffectedFile {
    /// Exact held workspace path.
    pub path: WorkspacePath,
    /// Exact pre-command digest, or absent when the file did not exist.
    pub preimage_sha256: Option<String>,
    /// Exact post-command digest, or absent when the file was removed.
    pub postimage_sha256: Option<String>,
}

/// Trusted observations required to normalize one terminal command attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationObservation {
    /// Stable non-replayable validation attempt identity.
    pub validation_attempt_id: String,
    /// Separate exact approval for this template and effective execution scope.
    pub approval_sha256: String,
    /// Retained stdout bytes from the trusted process executor.
    pub stdout: Vec<u8>,
    /// Retained stderr bytes from the trusted process executor.
    pub stderr: Vec<u8>,
    /// Trusted stdout classification.
    pub stdout_classification: ValidationOutputClassification,
    /// Trusted stderr classification.
    pub stderr_classification: ValidationOutputClassification,
    /// Total trusted secret-pattern matches across both streams.
    pub secret_match_count: u32,
    /// Independently observed expected artifacts.
    pub artifacts: Vec<ObservedValidationArtifact>,
    /// Independently observed affected files.
    pub affected_files: Vec<ValidationAffectedFile>,
    /// Exact approved changed paths used only for attribution.
    pub changed_paths: Vec<WorkspacePath>,
    /// Exact trusted baseline failure signatures.
    pub baseline_failure_sha256s: Vec<String>,
    /// Whether trusted evidence establishes a clean baseline for this check.
    pub baseline_known_clean: bool,
    /// Requested validation kinds that were not run.
    pub unverified_kinds: Vec<ValidationKind>,
}

/// Recomputed terminal validation receipt containing no raw process bytes or secret values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReceipt {
    /// Closed receipt schema version.
    pub schema_version: u16,
    /// Stable validation attempt identity.
    pub validation_attempt_id: String,
    /// Exact validation-template identity.
    pub validation_id: String,
    /// Exact validation kind.
    pub kind: ValidationKind,
    /// Exact validation-template digest.
    pub validation_template_sha256: String,
    /// Exact source configuration or explicit-input digest.
    pub template_source_sha256: String,
    /// Exact disposable snapshot and effective scope digest.
    pub execution_scope_sha256: String,
    /// Exact approval digest for this one execution.
    pub approval_sha256: String,
    /// Exact authority transaction identity.
    pub authority_transaction_id: String,
    /// Exact operation-attempt identity.
    pub operation_attempt_id: String,
    /// Exact command-attempt identity.
    pub command_attempt_id: String,
    /// Exact command-template identity.
    pub command_template_id: String,
    /// Exact command-template version.
    pub command_template_version: String,
    /// Exact command specification digest.
    pub command_spec_sha256: String,
    /// Exact command request digest.
    pub command_request_sha256: String,
    /// Exact approved command-preview digest.
    pub command_preview_sha256: String,
    /// Exact terminal command-receipt digest.
    pub command_receipt_sha256: String,
    /// Exact verified executable path.
    pub executable: String,
    /// Exact verified executable byte digest.
    pub executable_sha256: String,
    /// Exact literal argument vector.
    pub arguments: Vec<String>,
    /// Closed working-directory class.
    pub working_directory: CommandWorkingDirectory,
    /// Stable environment variable names; values are never retained here.
    pub environment_names: Vec<String>,
    /// Digest of the complete bounded replacement environment.
    pub environment_sha256: String,
    /// Closed result parser.
    pub parser: ValidationParserKind,
    /// Exact parser version.
    pub parser_version: String,
    /// Exact parser implementation digest.
    pub parser_sha256: String,
    /// Terminal process reason.
    pub termination: CommandTermination,
    /// Terminal mediated operation outcome.
    pub operation_outcome: OperationOutcome,
    /// Observed process exit code.
    pub exit_code: Option<i32>,
    /// Observed terminating signal.
    pub signal: Option<i32>,
    /// Stable content-free platform detail code.
    pub platform_code: String,
    /// Whether descendant cleanup was verified.
    pub descendants_terminated: bool,
    /// Observed monotonic duration.
    pub duration_ms: u64,
    /// Duration reported by the strict result parser when present.
    pub reported_duration_ms: Option<u64>,
    /// Complete stdout digest.
    pub stdout_sha256: String,
    /// Complete stdout byte count.
    pub stdout_total_bytes: u64,
    /// Whether stdout was truncated.
    pub stdout_truncated: bool,
    /// Trusted stdout classification.
    pub stdout_classification: ValidationOutputClassification,
    /// Complete stderr digest.
    pub stderr_sha256: String,
    /// Complete stderr byte count.
    pub stderr_total_bytes: u64,
    /// Whether stderr was truncated.
    pub stderr_truncated: bool,
    /// Trusted stderr classification.
    pub stderr_classification: ValidationOutputClassification,
    /// Trusted secret-pattern match count; raw matching material is never retained.
    pub secret_match_count: u32,
    /// Closed normalized result.
    pub status: ValidationStatus,
    /// Requested validation completeness.
    pub coverage: ValidationCoverageStatus,
    /// Number of passed tests.
    pub passed: u32,
    /// Number of failed tests.
    pub failed: u32,
    /// Number of skipped tests.
    pub skipped: u32,
    /// Stable failed-test names from strict parser output.
    pub failed_names: Vec<String>,
    /// Independently observed artifacts.
    pub artifacts: Vec<ObservedValidationArtifact>,
    /// Independently observed affected files.
    pub affected_files: Vec<ValidationAffectedFile>,
    /// Every requested but unrun kind.
    pub unverified_kinds: Vec<ValidationKind>,
    /// Deterministic failure classification when status is not passed.
    pub failure_classification: Option<ValidationFailureClassification>,
    /// Recomputable failure signature when status is not passed.
    pub failure_sha256: Option<String>,
    /// Number of separately approved reruns represented by strict parser evidence.
    pub retry_count: u16,
    /// Exact initial failure signature for a flaky result.
    pub initial_failure_sha256: Option<String>,
    /// This receipt carries no execution authority.
    pub execution_authority: bool,
    /// Canonical digest over every preceding field.
    pub receipt_sha256: String,
}

impl ValidationReceipt {
    /// Returns true only for complete, independently verified success.
    #[must_use]
    pub const fn is_full_pass(&self) -> bool {
        matches!(self.status, ValidationStatus::Passed)
            && matches!(self.coverage, ValidationCoverageStatus::Complete)
    }
}

/// Explicit separately approved rerun plan; construction never executes a command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationRerunPlan {
    /// Closed schema version.
    pub schema_version: u16,
    /// Stable new attempt identity.
    pub validation_attempt_id: String,
    /// Exact validation identity.
    pub validation_id: String,
    /// Exact unchanged validation-template digest.
    pub validation_template_sha256: String,
    /// Exact prior validation-receipt digest.
    pub prior_receipt_sha256: String,
    /// Exact prior failed-name set digest.
    pub failed_names_sha256: String,
    /// Exact separate rerun approval digest.
    pub approval_sha256: String,
    /// Setup was predeclared idempotent.
    pub setup_idempotent: bool,
    /// Command arguments remain byte-for-byte those of the registered template.
    pub command_spec_sha256: String,
    /// Rerun planning carries no execution authority.
    pub execution_authority: bool,
    /// Canonical digest over every preceding field.
    pub rerun_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ParsedStatus {
    Passed,
    AssertionFailed,
    CompileFailed,
    InfrastructureFailed,
    Flaky,
    SkippedOnly,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParsedValidationEnvelope {
    schema_version: u16,
    status: ParsedStatus,
    passed: u32,
    failed: u32,
    skipped: u32,
    duration_ms: u64,
    failed_names: Vec<String>,
    artifact_ids: Vec<String>,
    retry_count: u16,
    initial_failure_sha256: Option<String>,
}

#[derive(Debug)]
struct NormalizedFields {
    status: ValidationStatus,
    passed: u32,
    failed: u32,
    skipped: u32,
    failed_names: Vec<String>,
    retry_count: u16,
    initial_failure_sha256: Option<String>,
    reported_duration_ms: Option<u64>,
}

/// Normalizes one exact terminal validation attempt without trusting process narration.
pub fn normalize_validation_result(
    template: &ValidationTemplate,
    prepared: &PreparedCommand,
    command_receipt: &CommandReceipt,
    observation: ValidationObservation,
) -> Result<ValidationReceipt, ValidationResultError> {
    if !verify_validation_template(template)
        || template.command != *prepared.command()
        || !verify_command_receipt(prepared, command_receipt)
        || !valid_identifier(&observation.validation_attempt_id)
        || !is_sha256(&observation.approval_sha256)
    {
        return Err(ValidationResultError::EvidenceMismatch);
    }
    validate_output(command_receipt, &observation)?;
    validate_observations(template, &observation)?;

    let artifacts_complete = required_artifacts_complete(template, &observation.artifacts);
    let mut normalized = normalize_fields(template, command_receipt, &observation);
    if normalized.status == ValidationStatus::Passed && !artifacts_complete {
        normalized.status = ValidationStatus::Unverified;
    }
    let coverage = if observation.unverified_kinds.is_empty() {
        ValidationCoverageStatus::Complete
    } else {
        ValidationCoverageStatus::Partial
    };
    let failure_sha256 = (normalized.status != ValidationStatus::Passed)
        .then(|| failure_signature(normalized.status, &normalized.failed_names, command_receipt));
    let failure_classification = failure_sha256.as_ref().map(|signature| {
        classify_failure(normalized.status, signature, command_receipt, &observation)
    });
    let environment_names = template
        .command
        .environment
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let mut receipt = ValidationReceipt {
        schema_version: VALIDATION_RESULT_SCHEMA_VERSION,
        validation_attempt_id: observation.validation_attempt_id,
        validation_id: template.validation_id.clone(),
        kind: template.kind,
        validation_template_sha256: template.template_sha256.clone(),
        template_source_sha256: template.source_sha256.clone(),
        execution_scope_sha256: template.execution_scope_sha256.clone(),
        approval_sha256: observation.approval_sha256,
        authority_transaction_id: command_receipt.authority_transaction_id.clone(),
        operation_attempt_id: command_receipt.operation_attempt_id.clone(),
        command_attempt_id: command_receipt.command_attempt_id.clone(),
        command_template_id: command_receipt.template_id.clone(),
        command_template_version: command_receipt.template_version.clone(),
        command_spec_sha256: command_receipt.spec_sha256.clone(),
        command_request_sha256: command_receipt.request_sha256.clone(),
        command_preview_sha256: command_receipt.preview_sha256.clone(),
        command_receipt_sha256: command_receipt.receipt_sha256.clone(),
        executable: template.command.executable.clone(),
        executable_sha256: template.command.executable_sha256.clone(),
        arguments: template.command.arguments.clone(),
        working_directory: template.command.working_directory,
        environment_names,
        environment_sha256: sha256_json(&template.command.environment)?,
        parser: template.parser,
        parser_version: template.parser_version.clone(),
        parser_sha256: template.parser_sha256.clone(),
        termination: command_receipt.termination,
        operation_outcome: command_receipt.outcome,
        exit_code: command_receipt.exit_code,
        signal: command_receipt.signal,
        platform_code: command_receipt.platform_code.clone(),
        descendants_terminated: command_receipt.descendants_terminated,
        duration_ms: command_receipt.elapsed_ms,
        reported_duration_ms: normalized.reported_duration_ms,
        stdout_sha256: command_receipt.stdout_sha256.clone(),
        stdout_total_bytes: command_receipt.stdout_total_bytes,
        stdout_truncated: command_receipt.stdout_truncated,
        stdout_classification: observation.stdout_classification,
        stderr_sha256: command_receipt.stderr_sha256.clone(),
        stderr_total_bytes: command_receipt.stderr_total_bytes,
        stderr_truncated: command_receipt.stderr_truncated,
        stderr_classification: observation.stderr_classification,
        secret_match_count: observation.secret_match_count,
        status: normalized.status,
        coverage,
        passed: normalized.passed,
        failed: normalized.failed,
        skipped: normalized.skipped,
        failed_names: normalized.failed_names,
        artifacts: observation.artifacts,
        affected_files: observation.affected_files,
        unverified_kinds: observation.unverified_kinds,
        failure_classification,
        failure_sha256,
        retry_count: normalized.retry_count,
        initial_failure_sha256: normalized.initial_failure_sha256,
        execution_authority: false,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = validation_receipt_digest(&receipt)?;
    if !verify_validation_receipt(template, prepared, command_receipt, &receipt) {
        return Err(ValidationResultError::ReceiptFailure);
    }
    Ok(receipt)
}

/// Verifies a normalized receipt against its exact template and terminal command receipt.
#[must_use]
pub fn verify_validation_receipt(
    template: &ValidationTemplate,
    prepared: &PreparedCommand,
    command_receipt: &CommandReceipt,
    receipt: &ValidationReceipt,
) -> bool {
    let selected_environment_names = template
        .command
        .environment
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let complete_matches_unverified = match receipt.coverage {
        ValidationCoverageStatus::Complete => receipt.unverified_kinds.is_empty(),
        ValidationCoverageStatus::Partial => !receipt.unverified_kinds.is_empty(),
    };
    let pass_is_complete = receipt.status != ValidationStatus::Passed
        || (receipt.termination == CommandTermination::Exited
            && receipt.exit_code == Some(0)
            && !receipt.stdout_truncated
            && !receipt.stderr_truncated
            && receipt.secret_match_count == 0
            && receipt.stdout_classification != ValidationOutputClassification::SecretDetected
            && receipt.stderr_classification != ValidationOutputClassification::SecretDetected
            && required_artifacts_complete(template, &receipt.artifacts)
            && (!template.kind.is_test() || receipt.passed >= template.minimum_test_count));
    receipt.schema_version == VALIDATION_RESULT_SCHEMA_VERSION
        && verify_validation_template(template)
        && template.command == *prepared.command()
        && verify_command_receipt(prepared, command_receipt)
        && valid_identifier(&receipt.validation_attempt_id)
        && receipt.validation_id == template.validation_id
        && receipt.kind == template.kind
        && receipt.validation_template_sha256 == template.template_sha256
        && receipt.template_source_sha256 == template.source_sha256
        && receipt.execution_scope_sha256 == template.execution_scope_sha256
        && is_sha256(&receipt.approval_sha256)
        && receipt.authority_transaction_id == command_receipt.authority_transaction_id
        && receipt.operation_attempt_id == command_receipt.operation_attempt_id
        && receipt.command_attempt_id == command_receipt.command_attempt_id
        && receipt.command_template_id == command_receipt.template_id
        && receipt.command_template_version == command_receipt.template_version
        && receipt.command_spec_sha256 == command_receipt.spec_sha256
        && receipt.command_request_sha256 == command_receipt.request_sha256
        && receipt.command_preview_sha256 == command_receipt.preview_sha256
        && receipt.command_receipt_sha256 == command_receipt.receipt_sha256
        && receipt.executable == template.command.executable
        && receipt.executable_sha256 == template.command.executable_sha256
        && receipt.arguments == template.command.arguments
        && receipt.working_directory == template.command.working_directory
        && receipt.environment_names == selected_environment_names
        && sha256_json(&template.command.environment).as_ref() == Ok(&receipt.environment_sha256)
        && receipt.parser == template.parser
        && receipt.parser_version == template.parser_version
        && receipt.parser_sha256 == template.parser_sha256
        && receipt.termination == command_receipt.termination
        && receipt.operation_outcome == command_receipt.outcome
        && receipt.exit_code == command_receipt.exit_code
        && receipt.signal == command_receipt.signal
        && receipt.platform_code == command_receipt.platform_code
        && receipt.descendants_terminated == command_receipt.descendants_terminated
        && receipt.duration_ms == command_receipt.elapsed_ms
        && receipt
            .reported_duration_ms
            .is_none_or(|duration| duration <= receipt.duration_ms.saturating_add(1_000))
        && receipt.stdout_sha256 == command_receipt.stdout_sha256
        && receipt.stdout_total_bytes == command_receipt.stdout_total_bytes
        && receipt.stdout_truncated == command_receipt.stdout_truncated
        && receipt.stderr_sha256 == command_receipt.stderr_sha256
        && receipt.stderr_total_bytes == command_receipt.stderr_total_bytes
        && receipt.stderr_truncated == command_receipt.stderr_truncated
        && observations_are_sorted_and_valid(&receipt.artifacts, &receipt.affected_files)
        && receipt
            .unverified_kinds
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        && complete_matches_unverified
        && pass_is_complete
        && receipt.failure_sha256.is_some() == (receipt.status != ValidationStatus::Passed)
        && receipt.failure_classification.is_some() == (receipt.status != ValidationStatus::Passed)
        && !receipt.execution_authority
        && validation_receipt_digest(receipt).as_ref() == Ok(&receipt.receipt_sha256)
}

/// Creates a separately approved exact rerun plan only for safe failed-test setup.
pub fn plan_failed_test_rerun(
    template: &ValidationTemplate,
    prior: &ValidationReceipt,
    validation_attempt_id: impl Into<String>,
    approval_sha256: impl Into<String>,
) -> Result<ValidationRerunPlan, ValidationResultError> {
    let validation_attempt_id = validation_attempt_id.into();
    let approval_sha256 = approval_sha256.into();
    if !verify_validation_template(template)
        || prior.validation_id != template.validation_id
        || prior.validation_template_sha256 != template.template_sha256
        || !template.kind.is_test()
        || !template.failed_test_rerun_allowed
        || !template.setup_idempotent
        || !matches!(
            prior.status,
            ValidationStatus::AssertionFailed | ValidationStatus::CompileFailed
        )
        || prior.failed_names.is_empty()
        || !valid_identifier(&validation_attempt_id)
        || !is_sha256(&approval_sha256)
    {
        return Err(ValidationResultError::RerunForbidden);
    }
    let mut plan = ValidationRerunPlan {
        schema_version: VALIDATION_RESULT_SCHEMA_VERSION,
        validation_attempt_id,
        validation_id: template.validation_id.clone(),
        validation_template_sha256: template.template_sha256.clone(),
        prior_receipt_sha256: prior.receipt_sha256.clone(),
        failed_names_sha256: sha256_json(&prior.failed_names)?,
        approval_sha256,
        setup_idempotent: true,
        command_spec_sha256: template.command.spec_sha256.clone(),
        execution_authority: false,
        rerun_sha256: String::new(),
    };
    plan.rerun_sha256 = rerun_digest(&plan)?;
    Ok(plan)
}

/// Verifies that a rerun plan preserves the exact registered command and separate approval.
#[must_use]
pub fn verify_rerun_plan(
    template: &ValidationTemplate,
    prior: &ValidationReceipt,
    plan: &ValidationRerunPlan,
) -> bool {
    plan.schema_version == VALIDATION_RESULT_SCHEMA_VERSION
        && verify_validation_template(template)
        && valid_identifier(&plan.validation_attempt_id)
        && plan.validation_id == template.validation_id
        && plan.validation_template_sha256 == template.template_sha256
        && plan.prior_receipt_sha256 == prior.receipt_sha256
        && sha256_json(&prior.failed_names).as_ref() == Ok(&plan.failed_names_sha256)
        && is_sha256(&plan.approval_sha256)
        && plan.setup_idempotent
        && template.setup_idempotent
        && template.failed_test_rerun_allowed
        && plan.command_spec_sha256 == template.command.spec_sha256
        && !plan.execution_authority
        && rerun_digest(plan).as_ref() == Ok(&plan.rerun_sha256)
}

fn normalize_fields(
    template: &ValidationTemplate,
    command_receipt: &CommandReceipt,
    observation: &ValidationObservation,
) -> NormalizedFields {
    if command_receipt.termination != CommandTermination::Exited {
        return empty_fields(match command_receipt.termination {
            CommandTermination::Cancelled => ValidationStatus::Cancelled,
            CommandTermination::TimedOut => ValidationStatus::TimedOut,
            CommandTermination::OutputLimit => ValidationStatus::Truncated,
            CommandTermination::LaunchFailed => ValidationStatus::InfrastructureFailed,
            CommandTermination::Exited => unreachable!(),
        });
    }
    if command_receipt.signal.is_some() {
        return empty_fields(ValidationStatus::Crashed);
    }
    if command_receipt.stdout_truncated || command_receipt.stderr_truncated {
        return empty_fields(ValidationStatus::Truncated);
    }
    if observation.secret_match_count > 0
        || observation.stdout_classification == ValidationOutputClassification::SecretDetected
        || observation.stderr_classification == ValidationOutputClassification::SecretDetected
    {
        return empty_fields(ValidationStatus::SensitiveOutput);
    }
    if has_forbidden_controls(&observation.stdout) || has_forbidden_controls(&observation.stderr) {
        return empty_fields(ValidationStatus::Malformed);
    }
    match template.parser {
        ValidationParserKind::ProcessExitV1 => {
            if command_receipt.exit_code == Some(0) {
                empty_fields(ValidationStatus::Passed)
            } else {
                empty_fields(ValidationStatus::InfrastructureFailed)
            }
        }
        ValidationParserKind::AgentMageJsonV1 => parse_test_envelope(
            template,
            command_receipt,
            &observation.stdout,
            &observation.artifacts,
        )
        .unwrap_or_else(|| empty_fields(ValidationStatus::Malformed)),
    }
}

fn parse_test_envelope(
    template: &ValidationTemplate,
    command_receipt: &CommandReceipt,
    stdout: &[u8],
    artifacts: &[ObservedValidationArtifact],
) -> Option<NormalizedFields> {
    if stdout.is_empty() || stdout.len() > MAX_RESULT_BYTES {
        return None;
    }
    let parsed = serde_json::from_slice::<ParsedValidationEnvelope>(stdout).ok()?;
    let observed_artifact_ids = artifacts
        .iter()
        .map(|artifact| artifact.artifact_id.as_str())
        .collect::<Vec<_>>();
    if parsed.schema_version != VALIDATION_RESULT_SCHEMA_VERSION
        || parsed.duration_ms > command_receipt.elapsed_ms.saturating_add(1_000)
        || parsed.failed_names.len() > MAX_FAILED_NAMES
        || parsed
            .failed_names
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || parsed
            .failed_names
            .iter()
            .any(|name| !valid_test_name(name))
        || parsed.artifact_ids.len() > MAX_ARTIFACTS
        || parsed
            .artifact_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || parsed
            .artifact_ids
            .iter()
            .any(|name| !valid_identifier(name))
        || parsed.artifact_ids != observed_artifact_ids
        || parsed
            .initial_failure_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
    {
        return None;
    }
    let test_count = parsed.passed.saturating_add(parsed.failed);
    let status = match parsed.status {
        ParsedStatus::Passed
            if command_receipt.exit_code == Some(0)
                && parsed.failed == 0
                && parsed.failed_names.is_empty()
                && parsed.retry_count == 0
                && parsed.initial_failure_sha256.is_none() =>
        {
            if test_count < template.minimum_test_count {
                ValidationStatus::ZeroTests
            } else {
                ValidationStatus::Passed
            }
        }
        ParsedStatus::AssertionFailed
            if command_receipt.exit_code != Some(0)
                && parsed.failed > 0
                && parsed.failed as usize == parsed.failed_names.len()
                && parsed.retry_count == 0
                && parsed.initial_failure_sha256.is_none() =>
        {
            ValidationStatus::AssertionFailed
        }
        ParsedStatus::CompileFailed
            if command_receipt.exit_code != Some(0)
                && parsed.passed == 0
                && parsed.failed == 0
                && parsed.failed_names.is_empty()
                && parsed.retry_count == 0
                && parsed.initial_failure_sha256.is_none() =>
        {
            ValidationStatus::CompileFailed
        }
        ParsedStatus::InfrastructureFailed
            if command_receipt.exit_code != Some(0)
                && parsed.failed_names.is_empty()
                && parsed.retry_count == 0
                && parsed.initial_failure_sha256.is_none() =>
        {
            ValidationStatus::InfrastructureFailed
        }
        ParsedStatus::Flaky
            if command_receipt.exit_code == Some(0)
                && parsed.failed == 0
                && parsed.failed_names.is_empty()
                && parsed.passed >= template.minimum_test_count
                && parsed.retry_count > 0
                && parsed.initial_failure_sha256.is_some() =>
        {
            ValidationStatus::Flaky
        }
        ParsedStatus::SkippedOnly
            if command_receipt.exit_code == Some(0)
                && parsed.passed == 0
                && parsed.failed == 0
                && parsed.skipped > 0
                && parsed.failed_names.is_empty()
                && parsed.retry_count == 0
                && parsed.initial_failure_sha256.is_none() =>
        {
            ValidationStatus::SkippedOnly
        }
        _ => return None,
    };
    Some(NormalizedFields {
        status,
        passed: parsed.passed,
        failed: parsed.failed,
        skipped: parsed.skipped,
        failed_names: parsed.failed_names,
        retry_count: parsed.retry_count,
        initial_failure_sha256: parsed.initial_failure_sha256,
        reported_duration_ms: Some(parsed.duration_ms),
    })
}

fn empty_fields(status: ValidationStatus) -> NormalizedFields {
    NormalizedFields {
        status,
        passed: 0,
        failed: 0,
        skipped: 0,
        failed_names: Vec::new(),
        retry_count: 0,
        initial_failure_sha256: None,
        reported_duration_ms: None,
    }
}

fn validate_output(
    receipt: &CommandReceipt,
    observation: &ValidationObservation,
) -> Result<(), ValidationResultError> {
    if observation.stdout.len() as u64 != receipt.stdout_retained_bytes
        || observation.stderr.len() as u64 != receipt.stderr_retained_bytes
        || (!receipt.stdout_truncated && sha256_hex(&observation.stdout) != receipt.stdout_sha256)
        || (!receipt.stderr_truncated && sha256_hex(&observation.stderr) != receipt.stderr_sha256)
        || (observation.secret_match_count == 0
            && (observation.stdout_classification
                == ValidationOutputClassification::SecretDetected
                || observation.stderr_classification
                    == ValidationOutputClassification::SecretDetected))
        || (observation.secret_match_count > 0
            && observation.stdout_classification != ValidationOutputClassification::SecretDetected
            && observation.stderr_classification != ValidationOutputClassification::SecretDetected)
    {
        return Err(ValidationResultError::OutputMismatch);
    }
    Ok(())
}

fn validate_observations(
    template: &ValidationTemplate,
    observation: &ValidationObservation,
) -> Result<(), ValidationResultError> {
    if observation.artifacts.len() > MAX_ARTIFACTS
        || observation.affected_files.len() > MAX_AFFECTED_FILES
        || observation.changed_paths.len() > MAX_AFFECTED_FILES
        || observation.baseline_failure_sha256s.len() > MAX_FAILED_NAMES
        || observation.unverified_kinds.len() > MAX_UNVERIFIED_KINDS
        || !observations_are_sorted_and_valid(&observation.artifacts, &observation.affected_files)
        || observation
            .changed_paths
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || observation
            .baseline_failure_sha256s
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || observation
            .baseline_failure_sha256s
            .iter()
            .any(|digest| !is_sha256(digest))
        || observation
            .unverified_kinds
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || observation.unverified_kinds.contains(&template.kind)
        || !artifacts_match_expectations(template, &observation.artifacts)
    {
        return Err(ValidationResultError::ObservationInvalid);
    }
    Ok(())
}

fn observations_are_sorted_and_valid(
    artifacts: &[ObservedValidationArtifact],
    affected_files: &[ValidationAffectedFile],
) -> bool {
    artifacts.windows(2).all(|pair| pair[0] < pair[1])
        && artifacts.iter().all(|artifact| {
            valid_identifier(&artifact.artifact_id)
                && is_sha256(&artifact.sha256)
                && artifact.observed_after_command
        })
        && affected_files.windows(2).all(|pair| pair[0] < pair[1])
        && affected_files.iter().all(|file| {
            file.preimage_sha256.as_deref().is_none_or(is_sha256)
                && file.postimage_sha256.as_deref().is_none_or(is_sha256)
                && file.preimage_sha256 != file.postimage_sha256
                && (file.preimage_sha256.is_some() || file.postimage_sha256.is_some())
        })
}

fn artifacts_match_expectations(
    template: &ValidationTemplate,
    artifacts: &[ObservedValidationArtifact],
) -> bool {
    artifacts.iter().all(|observed| {
        template
            .expected_artifacts
            .iter()
            .find(|expected| expected.artifact_id == observed.artifact_id)
            .is_some_and(|expected| {
                expected.path == observed.path && observed.byte_length <= expected.maximum_bytes
            })
    })
}

fn required_artifacts_complete(
    template: &ValidationTemplate,
    artifacts: &[ObservedValidationArtifact],
) -> bool {
    template
        .expected_artifacts
        .iter()
        .filter(|expected| expected.required)
        .all(|expected| {
            artifacts.iter().any(|observed| {
                observed.artifact_id == expected.artifact_id
                    && observed.path == expected.path
                    && observed.byte_length <= expected.maximum_bytes
                    && observed.observed_after_command
            })
        })
}

fn classify_failure(
    status: ValidationStatus,
    signature: &str,
    receipt: &CommandReceipt,
    observation: &ValidationObservation,
) -> ValidationFailureClassification {
    let platform = receipt.platform_code.to_ascii_lowercase();
    if status == ValidationStatus::Flaky {
        ValidationFailureClassification::Flaky
    } else if platform.contains("permission") || platform.contains("denied") {
        ValidationFailureClassification::Permission
    } else if platform.contains("dependency") {
        ValidationFailureClassification::Dependency
    } else if matches!(
        status,
        ValidationStatus::InfrastructureFailed
            | ValidationStatus::TimedOut
            | ValidationStatus::Cancelled
            | ValidationStatus::Crashed
            | ValidationStatus::Truncated
            | ValidationStatus::SensitiveOutput
    ) || platform.contains("environment")
        || receipt.termination == CommandTermination::LaunchFailed
    {
        ValidationFailureClassification::Environment
    } else if observation
        .baseline_failure_sha256s
        .binary_search_by(|candidate| candidate.as_str().cmp(signature))
        .is_ok()
    {
        ValidationFailureClassification::Baseline
    } else if affected_intersects_changes(&observation.affected_files, &observation.changed_paths) {
        ValidationFailureClassification::ChangeCaused
    } else if observation.baseline_known_clean {
        ValidationFailureClassification::Unrelated
    } else {
        ValidationFailureClassification::Unclassified
    }
}

fn affected_intersects_changes(
    affected_files: &[ValidationAffectedFile],
    changed_paths: &[WorkspacePath],
) -> bool {
    let changed = changed_paths.iter().collect::<BTreeSet<_>>();
    affected_files
        .iter()
        .any(|file| changed.contains(&file.path))
}

fn failure_signature(
    status: ValidationStatus,
    failed_names: &[String],
    receipt: &CommandReceipt,
) -> String {
    sha256_json(&(
        status,
        failed_names,
        receipt.stderr_sha256.as_str(),
        receipt.platform_code.as_str(),
        receipt.exit_code,
        receipt.signal,
    ))
    .unwrap_or_else(|_| sha256_hex(b"validation-failure-signature-failed"))
}

fn validation_receipt_digest(receipt: &ValidationReceipt) -> Result<String, ValidationResultError> {
    let mut canonical = receipt.clone();
    canonical.receipt_sha256.clear();
    sha256_json(&canonical)
}

fn rerun_digest(plan: &ValidationRerunPlan) -> Result<String, ValidationResultError> {
    let mut canonical = plan.clone();
    canonical.rerun_sha256.clear();
    sha256_json(&canonical)
}

fn sha256_json(value: &impl Serialize) -> Result<String, ValidationResultError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| ValidationResultError::ReceiptFailure)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn has_forbidden_controls(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .any(|byte| *byte < 0x20 && !matches!(*byte, b'\n' | b'\r' | b'\t'))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_test_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agentmage_kernel_contracts::{OperationOutcome, WorkspaceId};
    use serde_json::json;

    use crate::command_runner::{
        CommandBounds, CommandRegistry, CommandRequest, CommandRisk, CommandSpec,
        CommandWorkingDirectory, prepare_command,
    };
    use crate::validation_template::{
        ValidationArtifactExpectation, ValidationTemplateInput, ValidationTemplateSource,
        seal_validation_template,
    };

    use super::*;

    fn path(parts: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-validation-result"),
            parts.iter().copied(),
        )
        .expect("path")
    }

    fn command() -> CommandSpec {
        CommandSpec::seal(
            "fixture.validation",
            "1.0.0",
            "/usr/bin/cargo",
            "1".repeat(64),
            vec!["test".to_owned(), "--message-format=json".to_owned()],
            CommandWorkingDirectory::EmptyScratch,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("NO_COLOR".to_owned(), "1".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Moderate,
            CommandBounds::new(30_000, 512 * 1024, 512 * 1024, 256 * 1024 * 1024, 32, 200)
                .expect("bounds"),
        )
        .expect("command")
    }

    fn template() -> ValidationTemplate {
        seal_validation_template(ValidationTemplateInput {
            validation_id: "validation-unit".to_owned(),
            kind: ValidationKind::Unit,
            command: command(),
            source: ValidationTemplateSource::TrustedProjectConfiguration,
            source_sha256: "2".repeat(64),
            source_path: Some(path(&["Cargo.toml"])),
            user_input_approval_sha256: None,
            execution_scope_sha256: "3".repeat(64),
            parser: ValidationParserKind::AgentMageJsonV1,
            parser_version: "1.0.0".to_owned(),
            parser_sha256: "4".repeat(64),
            minimum_test_count: 1,
            focused: true,
            fail_fast: true,
            failed_test_rerun_allowed: true,
            setup_idempotent: true,
            expected_artifacts: Vec::new(),
        })
        .expect("template")
    }

    fn prepared(template: &ValidationTemplate) -> PreparedCommand {
        let registry = CommandRegistry::build(vec![template.command.clone()]).expect("registry");
        prepare_command(
            &registry,
            CommandRequest::new("command-validation-0001", &template.command),
        )
        .expect("prepared")
    }

    #[allow(clippy::too_many_arguments)]
    fn command_receipt(
        prepared: &PreparedCommand,
        stdout: &[u8],
        stderr: &[u8],
        termination: CommandTermination,
        exit_code: Option<i32>,
        signal: Option<i32>,
        truncated: bool,
        platform_code: &str,
    ) -> CommandReceipt {
        let outcome = match termination {
            CommandTermination::Exited if exit_code == Some(0) => OperationOutcome::Succeeded,
            CommandTermination::Exited
            | CommandTermination::OutputLimit
            | CommandTermination::LaunchFailed => OperationOutcome::Failed,
            CommandTermination::Cancelled => OperationOutcome::Cancelled,
            CommandTermination::TimedOut => OperationOutcome::TimedOut,
        };
        let mut receipt = CommandReceipt {
            schema_version: 1,
            authority_transaction_id: "transaction-validation-0001".to_owned(),
            operation_attempt_id: "operation-validation-0001".to_owned(),
            command_attempt_id: prepared.preview().command_attempt_id.clone(),
            template_id: prepared.command().template_id.clone(),
            template_version: prepared.command().template_version.clone(),
            spec_sha256: prepared.command().spec_sha256.clone(),
            request_sha256: prepared.request_sha256().to_owned(),
            preview_sha256: prepared.preview().preview_sha256.clone(),
            termination,
            outcome,
            exit_code,
            signal,
            stdout_sha256: sha256_hex(stdout),
            stdout_total_bytes: stdout.len() as u64 + u64::from(truncated),
            stdout_retained_bytes: stdout.len() as u64,
            stdout_truncated: truncated,
            stderr_sha256: sha256_hex(stderr),
            stderr_total_bytes: stderr.len() as u64,
            stderr_retained_bytes: stderr.len() as u64,
            stderr_truncated: false,
            elapsed_ms: 100,
            resource_usage: None,
            descendants_terminated: true,
            platform_code: platform_code.to_owned(),
            receipt_sha256: "0".repeat(64),
        };
        receipt.receipt_sha256 = sha256_json(&receipt).expect("receipt digest");
        receipt
    }

    fn observation(stdout: Vec<u8>) -> ValidationObservation {
        ValidationObservation {
            validation_attempt_id: "validation-attempt-0001".to_owned(),
            approval_sha256: "5".repeat(64),
            stdout,
            stderr: Vec::new(),
            stdout_classification: ValidationOutputClassification::Internal,
            stderr_classification: ValidationOutputClassification::Internal,
            secret_match_count: 0,
            artifacts: Vec::new(),
            affected_files: Vec::new(),
            changed_paths: Vec::new(),
            baseline_failure_sha256s: Vec::new(),
            baseline_known_clean: false,
            unverified_kinds: Vec::new(),
        }
    }

    fn envelope(
        status: &str,
        passed: u32,
        failed: u32,
        skipped: u32,
        failed_names: &[&str],
        retry_count: u16,
        initial_failure_sha256: Option<String>,
    ) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "status": status,
            "passed": passed,
            "failed": failed,
            "skipped": skipped,
            "duration_ms": 50,
            "failed_names": failed_names,
            "artifact_ids": [],
            "retry_count": retry_count,
            "initial_failure_sha256": initial_failure_sha256,
        }))
        .expect("envelope")
    }

    fn normalize(
        output: Vec<u8>,
        termination: CommandTermination,
        exit_code: Option<i32>,
        signal: Option<i32>,
        truncated: bool,
        platform_code: &str,
    ) -> ValidationReceipt {
        let template = template();
        let prepared = prepared(&template);
        let command_receipt = command_receipt(
            &prepared,
            &output,
            &[],
            termination,
            exit_code,
            signal,
            truncated,
            platform_code,
        );
        normalize_validation_result(&template, &prepared, &command_receipt, observation(output))
            .expect("normalized")
    }

    #[test]
    fn parser_preserves_every_terminal_result_state_without_false_green() {
        let cases = [
            (
                envelope("passed", 2, 0, 0, &[], 0, None),
                CommandTermination::Exited,
                Some(0),
                None,
                false,
                "fixture.exited",
                ValidationStatus::Passed,
            ),
            (
                envelope("assertion_failed", 1, 1, 0, &["suite::fails"], 0, None),
                CommandTermination::Exited,
                Some(1),
                None,
                false,
                "fixture.exited",
                ValidationStatus::AssertionFailed,
            ),
            (
                envelope("compile_failed", 0, 0, 0, &[], 0, None),
                CommandTermination::Exited,
                Some(101),
                None,
                false,
                "fixture.exited",
                ValidationStatus::CompileFailed,
            ),
            (
                envelope("infrastructure_failed", 0, 0, 0, &[], 0, None),
                CommandTermination::Exited,
                Some(2),
                None,
                false,
                "fixture.environment",
                ValidationStatus::InfrastructureFailed,
            ),
            (
                envelope("flaky", 2, 0, 0, &[], 1, Some("6".repeat(64))),
                CommandTermination::Exited,
                Some(0),
                None,
                false,
                "fixture.exited",
                ValidationStatus::Flaky,
            ),
            (
                envelope("skipped_only", 0, 0, 2, &[], 0, None),
                CommandTermination::Exited,
                Some(0),
                None,
                false,
                "fixture.exited",
                ValidationStatus::SkippedOnly,
            ),
            (
                envelope("passed", 0, 0, 0, &[], 0, None),
                CommandTermination::Exited,
                Some(0),
                None,
                false,
                "fixture.exited",
                ValidationStatus::ZeroTests,
            ),
            (
                b"ALL TESTS PASSED".to_vec(),
                CommandTermination::Exited,
                Some(0),
                None,
                false,
                "fixture.exited",
                ValidationStatus::Malformed,
            ),
            (
                envelope("passed", 1, 0, 0, &[], 0, None),
                CommandTermination::TimedOut,
                None,
                None,
                false,
                "fixture.timeout",
                ValidationStatus::TimedOut,
            ),
            (
                envelope("passed", 1, 0, 0, &[], 0, None),
                CommandTermination::Cancelled,
                None,
                None,
                false,
                "fixture.cancelled",
                ValidationStatus::Cancelled,
            ),
            (
                b"{".to_vec(),
                CommandTermination::OutputLimit,
                None,
                None,
                true,
                "fixture.output-limit",
                ValidationStatus::Truncated,
            ),
            (
                Vec::new(),
                CommandTermination::LaunchFailed,
                None,
                None,
                false,
                "fixture.environment",
                ValidationStatus::InfrastructureFailed,
            ),
            (
                envelope("passed", 1, 0, 0, &[], 0, None),
                CommandTermination::Exited,
                Some(101),
                Some(11),
                false,
                "fixture.crashed",
                ValidationStatus::Crashed,
            ),
        ];
        for (output, termination, exit_code, signal, truncated, platform, expected) in cases {
            let receipt = normalize(output, termination, exit_code, signal, truncated, platform);
            assert_eq!(receipt.status, expected, "{platform}");
            assert_eq!(receipt.is_full_pass(), expected == ValidationStatus::Passed);
        }
    }

    #[test]
    fn controls_secret_output_parser_bombs_and_injected_artifacts_never_pass() {
        let mut ansi = b"\x1b[32m".to_vec();
        ansi.extend(envelope("passed", 1, 0, 0, &[], 0, None));
        assert_eq!(
            normalize(
                ansi,
                CommandTermination::Exited,
                Some(0),
                None,
                false,
                "fixture.exited"
            )
            .status,
            ValidationStatus::Malformed
        );

        let template = template();
        let prepared = prepared(&template);
        let output = envelope("passed", 1, 0, 0, &[], 0, None);
        let command_receipt = command_receipt(
            &prepared,
            &output,
            &[],
            CommandTermination::Exited,
            Some(0),
            None,
            false,
            "fixture.exited",
        );
        let mut secret = observation(output.clone());
        secret.stdout_classification = ValidationOutputClassification::SecretDetected;
        secret.secret_match_count = 1;
        let secret_receipt =
            normalize_validation_result(&template, &prepared, &command_receipt, secret)
                .expect("secret receipt");
        assert_eq!(secret_receipt.status, ValidationStatus::SensitiveOutput);
        assert!(
            !serde_json::to_string(&secret_receipt)
                .expect("serialize")
                .contains(std::str::from_utf8(&output).expect("utf8"))
        );

        let bomb = vec![b' '; MAX_RESULT_BYTES + 1];
        assert_eq!(
            normalize(
                bomb,
                CommandTermination::Exited,
                Some(0),
                None,
                false,
                "fixture.exited"
            )
            .status,
            ValidationStatus::Malformed
        );

        let mut injected = observation(output);
        injected.artifacts.push(ObservedValidationArtifact {
            artifact_id: "injected-result".to_owned(),
            path: path(&["result.json"]),
            sha256: "7".repeat(64),
            byte_length: 10,
            observed_after_command: true,
        });
        assert_eq!(
            normalize_validation_result(&template, &prepared, &command_receipt, injected),
            Err(ValidationResultError::ObservationInvalid)
        );
    }

    #[test]
    fn required_artifacts_partial_coverage_and_receipt_mutation_fail_closed() {
        let mut artifact_template = template();
        let input = ValidationTemplateInput {
            validation_id: artifact_template.validation_id.clone(),
            kind: artifact_template.kind,
            command: artifact_template.command.clone(),
            source: artifact_template.source,
            source_sha256: artifact_template.source_sha256.clone(),
            source_path: artifact_template.source_path.clone(),
            user_input_approval_sha256: artifact_template.user_input_approval_sha256.clone(),
            execution_scope_sha256: artifact_template.execution_scope_sha256.clone(),
            parser: artifact_template.parser,
            parser_version: artifact_template.parser_version.clone(),
            parser_sha256: artifact_template.parser_sha256.clone(),
            minimum_test_count: artifact_template.minimum_test_count,
            focused: artifact_template.focused,
            fail_fast: artifact_template.fail_fast,
            failed_test_rerun_allowed: artifact_template.failed_test_rerun_allowed,
            setup_idempotent: artifact_template.setup_idempotent,
            expected_artifacts: vec![ValidationArtifactExpectation {
                artifact_id: "coverage".to_owned(),
                path: path(&["coverage.json"]),
                required: true,
                maximum_bytes: 1024,
            }],
        };
        artifact_template = seal_validation_template(input).expect("artifact template");
        let artifact_prepared = prepared(&artifact_template);
        let output = envelope("passed", 1, 0, 0, &[], 0, None);
        let artifact_command_receipt = command_receipt(
            &artifact_prepared,
            &output,
            &[],
            CommandTermination::Exited,
            Some(0),
            None,
            false,
            "fixture.exited",
        );
        let receipt = normalize_validation_result(
            &artifact_template,
            &artifact_prepared,
            &artifact_command_receipt,
            observation(output),
        )
        .expect("missing artifact receipt");
        assert_eq!(receipt.status, ValidationStatus::Unverified);
        assert!(!receipt.is_full_pass());

        let plain_template = template();
        let plain_prepared = prepared(&plain_template);
        let plain_output = envelope("passed", 1, 0, 0, &[], 0, None);
        let plain_command_receipt = command_receipt(
            &plain_prepared,
            &plain_output,
            &[],
            CommandTermination::Exited,
            Some(0),
            None,
            false,
            "fixture.exited",
        );
        let mut partial_observation = observation(plain_output);
        partial_observation.unverified_kinds = vec![ValidationKind::Security];
        let partial = normalize_validation_result(
            &plain_template,
            &plain_prepared,
            &plain_command_receipt,
            partial_observation,
        )
        .expect("partial receipt");
        assert_eq!(partial.status, ValidationStatus::Passed);
        assert_eq!(partial.coverage, ValidationCoverageStatus::Partial);
        assert!(!partial.is_full_pass());

        let mut forged = partial.clone();
        forged.coverage = ValidationCoverageStatus::Complete;
        forged.receipt_sha256 = validation_receipt_digest(&forged).expect("forged digest");
        assert!(!verify_validation_receipt(
            &plain_template,
            &plain_prepared,
            &plain_command_receipt,
            &forged
        ));
    }

    #[test]
    fn exact_separate_rerun_requires_failed_idempotent_registered_template() {
        let template = template();
        let prepared = prepared(&template);
        let output = envelope("assertion_failed", 0, 1, 0, &["suite::fails"], 0, None);
        let command_receipt = command_receipt(
            &prepared,
            &output,
            &[],
            CommandTermination::Exited,
            Some(1),
            None,
            false,
            "fixture.exited",
        );
        let receipt = normalize_validation_result(
            &template,
            &prepared,
            &command_receipt,
            observation(output),
        )
        .expect("failed receipt");
        let plan = plan_failed_test_rerun(
            &template,
            &receipt,
            "validation-attempt-rerun-0002",
            "8".repeat(64),
        )
        .expect("rerun plan");
        assert!(verify_rerun_plan(&template, &receipt, &plan));
        assert!(!plan.execution_authority);
        assert_eq!(plan.command_spec_sha256, template.command.spec_sha256);

        let mut unsafe_input = ValidationTemplateInput {
            validation_id: template.validation_id.clone(),
            kind: template.kind,
            command: template.command.clone(),
            source: template.source,
            source_sha256: template.source_sha256.clone(),
            source_path: template.source_path.clone(),
            user_input_approval_sha256: template.user_input_approval_sha256.clone(),
            execution_scope_sha256: template.execution_scope_sha256.clone(),
            parser: template.parser,
            parser_version: template.parser_version.clone(),
            parser_sha256: template.parser_sha256.clone(),
            minimum_test_count: template.minimum_test_count,
            focused: template.focused,
            fail_fast: template.fail_fast,
            failed_test_rerun_allowed: false,
            setup_idempotent: false,
            expected_artifacts: template.expected_artifacts.clone(),
        };
        unsafe_input.validation_id = "validation-unsafe".to_owned();
        let unsafe_template = seal_validation_template(unsafe_input).expect("unsafe template");
        assert_eq!(
            plan_failed_test_rerun(
                &unsafe_template,
                &receipt,
                "validation-attempt-rerun-0003",
                "9".repeat(64)
            ),
            Err(ValidationResultError::RerunForbidden)
        );
    }

    #[test]
    fn failure_classification_uses_permission_baseline_change_and_flake_evidence_order() {
        let template = template();
        let prepared = prepared(&template);
        let output = envelope("assertion_failed", 0, 1, 0, &["suite::fails"], 0, None);
        let assertion_command_receipt = command_receipt(
            &prepared,
            &output,
            &[],
            CommandTermination::Exited,
            Some(1),
            None,
            false,
            "fixture.exited",
        );
        let changed = path(&["src", "lib.rs"]);
        let mut changed_observation = observation(output.clone());
        changed_observation.changed_paths = vec![changed.clone()];
        changed_observation.affected_files = vec![ValidationAffectedFile {
            path: changed,
            preimage_sha256: Some("a".repeat(64)),
            postimage_sha256: Some("b".repeat(64)),
        }];
        let changed_receipt = normalize_validation_result(
            &template,
            &prepared,
            &assertion_command_receipt,
            changed_observation.clone(),
        )
        .expect("changed receipt");
        assert_eq!(
            changed_receipt.failure_classification,
            Some(ValidationFailureClassification::ChangeCaused)
        );

        changed_observation.baseline_failure_sha256s =
            vec![changed_receipt.failure_sha256.clone().expect("signature")];
        let baseline_receipt = normalize_validation_result(
            &template,
            &prepared,
            &assertion_command_receipt,
            changed_observation,
        )
        .expect("baseline receipt");
        assert_eq!(
            baseline_receipt.failure_classification,
            Some(ValidationFailureClassification::Baseline)
        );

        let permission_command_receipt = command_receipt(
            &prepared,
            &output,
            &[],
            CommandTermination::Exited,
            Some(1),
            None,
            false,
            "fixture.permission-denied",
        );
        let permission_receipt = normalize_validation_result(
            &template,
            &prepared,
            &permission_command_receipt,
            observation(output),
        )
        .expect("permission receipt");
        assert_eq!(
            permission_receipt.failure_classification,
            Some(ValidationFailureClassification::Permission)
        );

        let flaky = normalize(
            envelope("flaky", 1, 0, 0, &[], 1, Some("c".repeat(64))),
            CommandTermination::Exited,
            Some(0),
            None,
            false,
            "fixture.exited",
        );
        assert_eq!(
            flaky.failure_classification,
            Some(ValidationFailureClassification::Flaky)
        );
    }
}

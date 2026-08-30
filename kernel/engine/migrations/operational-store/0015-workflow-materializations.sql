CREATE UNIQUE INDEX runtime_runs_exact_session_idx
    ON runtime_runs(run_id, session_id);

CREATE UNIQUE INDEX runtime_events_exact_identity_idx
    ON runtime_events(run_id, sequence, event_id);

CREATE TABLE workflow_plan_step_policies (
    policy_id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL,
    plan_step_id TEXT NOT NULL,
    plan_revision INTEGER NOT NULL CHECK(plan_revision > 0),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    policy_sha256 TEXT NOT NULL UNIQUE CHECK(length(policy_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(plan_id) REFERENCES plans(plan_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE TABLE workflow_attempts (
    attempt_id TEXT PRIMARY KEY,
    step_execution_id TEXT NOT NULL,
    call_id TEXT NOT NULL,
    attempt_ordinal INTEGER NOT NULL CHECK(attempt_ordinal > 0),
    supersedes_attempt_id TEXT,
    grant_id TEXT NOT NULL,
    state TEXT NOT NULL CHECK(
        state IN ('started', 'succeeded', 'failed', 'denied', 'cancelled', 'timed_out', 'uncertain')
    ),
    receipt_id TEXT,
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    attempt_sha256 TEXT NOT NULL UNIQUE CHECK(length(attempt_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(supersedes_attempt_id) REFERENCES workflow_attempts(attempt_id),
    FOREIGN KEY(grant_id) REFERENCES grant_identities(grant_id),
    FOREIGN KEY(receipt_id) REFERENCES receipts(receipt_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE TABLE workflow_preflights (
    preflight_record_id TEXT PRIMARY KEY,
    step_execution_id TEXT NOT NULL,
    attempt_id TEXT,
    preflight_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    disposition TEXT NOT NULL CHECK(
        disposition IN ('passed', 'failed', 'stale', 'unavailable')
    ),
    observed_at_epoch_ms INTEGER NOT NULL CHECK(observed_at_epoch_ms > 0),
    valid_until_epoch_ms INTEGER CHECK(
        valid_until_epoch_ms IS NULL OR valid_until_epoch_ms > observed_at_epoch_ms
    ),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    preflight_sha256 TEXT NOT NULL UNIQUE CHECK(length(preflight_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(attempt_id) REFERENCES workflow_attempts(attempt_id),
    FOREIGN KEY(policy_id) REFERENCES workflow_plan_step_policies(policy_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE TABLE workflow_tool_calls (
    call_id TEXT PRIMARY KEY,
    step_execution_id TEXT NOT NULL,
    tool_call_id TEXT NOT NULL UNIQUE,
    tool_id TEXT NOT NULL,
    tool_version TEXT NOT NULL,
    validation_state TEXT NOT NULL CHECK(
        validation_state IN ('validated', 'repaired_then_validated', 'rejected')
    ),
    validated_arguments_sha256 TEXT CHECK(
        validated_arguments_sha256 IS NULL OR length(validated_arguments_sha256) = 64
    ),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    call_sha256 TEXT NOT NULL UNIQUE CHECK(length(call_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id),
    CHECK(
        (validation_state IN ('validated', 'repaired_then_validated')
            AND validated_arguments_sha256 IS NOT NULL)
        OR
        (validation_state = 'rejected' AND validated_arguments_sha256 IS NULL)
    )
) STRICT;

CREATE TABLE workflow_idempotency_keys (
    idempotency_record_id TEXT PRIMARY KEY,
    step_execution_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    key_sha256 TEXT NOT NULL CHECK(length(key_sha256) = 64),
    desired_state_sha256 TEXT CHECK(
        desired_state_sha256 IS NULL OR length(desired_state_sha256) = 64
    ),
    lifecycle_state TEXT NOT NULL CHECK(
        lifecycle_state IN ('fresh', 'consumed', 'invalidated')
    ),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(attempt_id) REFERENCES workflow_attempts(attempt_id),
    FOREIGN KEY(policy_id) REFERENCES workflow_plan_step_policies(policy_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE TABLE workflow_approvals (
    approval_record_id TEXT PRIMARY KEY,
    approval_id TEXT NOT NULL,
    step_execution_id TEXT NOT NULL,
    attempt_id TEXT,
    disposition TEXT NOT NULL CHECK(
        disposition IN ('requested', 'approved', 'denied', 'expired', 'cancelled')
    ),
    approval_sha256 TEXT NOT NULL CHECK(length(approval_sha256) = 64),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(attempt_id) REFERENCES workflow_attempts(attempt_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE TABLE workflow_receipts (
    receipt_record_id TEXT PRIMARY KEY,
    receipt_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK(
        outcome IN ('succeeded', 'denied', 'failed', 'cancelled', 'timed_out', 'uncertain')
    ),
    receipt_sha256 TEXT NOT NULL CHECK(length(receipt_sha256) = 64),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(attempt_id) REFERENCES workflow_attempts(attempt_id),
    FOREIGN KEY(receipt_id) REFERENCES receipts(receipt_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE TABLE workflow_verifications (
    verification_id TEXT PRIMARY KEY,
    step_execution_id TEXT NOT NULL,
    attempt_id TEXT,
    verification_result_id TEXT NOT NULL,
    verifier_policy_id TEXT NOT NULL,
    required INTEGER NOT NULL CHECK(required IN (0, 1)),
    current INTEGER NOT NULL CHECK(current IN (0, 1)),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    verification_sha256 TEXT NOT NULL UNIQUE CHECK(length(verification_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(attempt_id) REFERENCES workflow_attempts(attempt_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE TABLE workflow_consumed_budgets (
    consumption_id TEXT PRIMARY KEY,
    step_execution_id TEXT,
    attempt_id TEXT,
    budget_policy_id TEXT NOT NULL,
    dimension TEXT NOT NULL CHECK(
        dimension IN (
            'turns', 'tokens', 'duration_ms', 'tool_calls', 'attempts',
            'no_progress_events', 'output_bytes', 'memory_bytes', 'cost_minor_units'
        )
    ),
    consumed_amount INTEGER NOT NULL CHECK(consumed_amount > 0),
    consumed_total INTEGER NOT NULL CHECK(consumed_total >= consumed_amount),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    consumption_sha256 TEXT NOT NULL UNIQUE CHECK(length(consumption_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(attempt_id) REFERENCES workflow_attempts(attempt_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE TABLE workflow_recovery_decisions (
    decision_id TEXT PRIMARY KEY,
    step_execution_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    failure_class TEXT NOT NULL CHECK(
        failure_class IN (
            'transport', 'rate', 'timeout', 'crash', 'unavailable_service',
            'missing_command', 'invalid_arguments', 'authentication', 'permission',
            'policy_denial', 'deterministic_verification_failure',
            'malformed_model_output', 'context_overflow', 'user_rejection'
        )
    ),
    uncertain_outcome INTEGER NOT NULL CHECK(uncertain_outcome IN (0, 1)),
    decision TEXT NOT NULL CHECK(
        decision IN (
            'retry_new_attempt', 'reconcile_then_decide', 'request_approval',
            'request_user_decision', 'replan', 'defer', 'stop'
        )
    ),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    decision_sha256 TEXT NOT NULL UNIQUE CHECK(length(decision_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(attempt_id) REFERENCES workflow_attempts(attempt_id),
    FOREIGN KEY(policy_id) REFERENCES workflow_plan_step_policies(policy_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE TABLE workflow_state_fingerprints (
    fingerprint_record_id TEXT PRIMARY KEY,
    step_execution_id TEXT,
    fingerprint_sha256 TEXT NOT NULL CHECK(length(fingerprint_sha256) = 64),
    occurrence INTEGER NOT NULL CHECK(occurrence > 0),
    repeat_count INTEGER NOT NULL CHECK(repeat_count >= 0),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id),
    CHECK(occurrence = repeat_count + 1)
) STRICT;

CREATE TABLE workflow_terminal_diagnostics (
    diagnostic_id TEXT PRIMARY KEY,
    terminal_result_id TEXT NOT NULL,
    diagnostic_code TEXT NOT NULL,
    failure_class TEXT CHECK(
        failure_class IS NULL OR failure_class IN (
            'transport', 'rate', 'timeout', 'crash', 'unavailable_service',
            'missing_command', 'invalid_arguments', 'authentication', 'permission',
            'policy_denial', 'deterministic_verification_failure',
            'malformed_model_output', 'context_overflow', 'user_rejection'
        )
    ),
    safe_next_action_code TEXT,
    disclosure TEXT NOT NULL CHECK(
        disclosure IN ('content_free_codes', 'content_free_codes_with_artifact_reference')
    ),
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    diagnostic_sha256 TEXT NOT NULL UNIQUE CHECK(length(diagnostic_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE INDEX workflow_plan_step_policies_runtime_idx
    ON workflow_plan_step_policies(run_id, event_sequence, plan_step_id);
CREATE INDEX workflow_attempts_runtime_idx
    ON workflow_attempts(run_id, event_sequence, step_execution_id);
CREATE INDEX workflow_preflights_runtime_idx
    ON workflow_preflights(run_id, event_sequence, step_execution_id);
CREATE INDEX workflow_tool_calls_runtime_idx
    ON workflow_tool_calls(run_id, event_sequence, step_execution_id);
CREATE INDEX workflow_idempotency_keys_runtime_idx
    ON workflow_idempotency_keys(run_id, event_sequence, step_execution_id);
CREATE INDEX workflow_approvals_runtime_idx
    ON workflow_approvals(run_id, event_sequence, step_execution_id);
CREATE INDEX workflow_receipts_runtime_idx
    ON workflow_receipts(run_id, event_sequence, attempt_id);
CREATE INDEX workflow_verifications_runtime_idx
    ON workflow_verifications(run_id, event_sequence, step_execution_id);
CREATE INDEX workflow_consumed_budgets_runtime_idx
    ON workflow_consumed_budgets(run_id, event_sequence, step_execution_id);
CREATE INDEX workflow_recovery_decisions_runtime_idx
    ON workflow_recovery_decisions(run_id, event_sequence, step_execution_id);
CREATE INDEX workflow_state_fingerprints_runtime_idx
    ON workflow_state_fingerprints(run_id, event_sequence, step_execution_id);
CREATE INDEX workflow_terminal_diagnostics_runtime_idx
    ON workflow_terminal_diagnostics(run_id, event_sequence, terminal_result_id);

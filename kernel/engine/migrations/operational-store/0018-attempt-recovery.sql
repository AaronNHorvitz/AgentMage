CREATE TABLE workflow_attempt_checkpoints (
    attempt_checkpoint_id TEXT PRIMARY KEY,
    workflow_checkpoint_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    execution_id TEXT NOT NULL,
    step_execution_id TEXT NOT NULL,
    attempt_id TEXT,
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    effect_state TEXT NOT NULL CHECK(
        effect_state IN ('no_effect', 'verified_not_applied', 'verified_applied', 'uncertain')
    ),
    authority_consumed INTEGER NOT NULL CHECK(authority_consumed IN (0, 1)),
    checkpoint_sha256 TEXT NOT NULL UNIQUE CHECK(length(checkpoint_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(workflow_checkpoint_id) REFERENCES workflow_checkpoints(checkpoint_id),
    FOREIGN KEY(attempt_id) REFERENCES workflow_attempts(attempt_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id)
) STRICT;

CREATE INDEX workflow_attempt_checkpoints_current_idx
    ON workflow_attempt_checkpoints(workflow_id, event_sequence DESC, attempt_checkpoint_id);

CREATE TABLE workflow_attempt_resume_claims (
    attempt_checkpoint_id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    decision_sha256 TEXT NOT NULL CHECK(length(decision_sha256) = 64),
    claimed_at_epoch_ms INTEGER NOT NULL CHECK(claimed_at_epoch_ms > 0),
    FOREIGN KEY(attempt_checkpoint_id)
        REFERENCES workflow_attempt_checkpoints(attempt_checkpoint_id)
) STRICT;

CREATE TRIGGER workflow_attempt_checkpoint_update_forbidden
BEFORE UPDATE ON workflow_attempt_checkpoints
BEGIN
    SELECT RAISE(ABORT, 'workflow.attempt_checkpoint.append_only');
END;

CREATE TRIGGER workflow_attempt_checkpoint_delete_forbidden
BEFORE DELETE ON workflow_attempt_checkpoints
BEGIN
    SELECT RAISE(ABORT, 'workflow.attempt_checkpoint.append_only');
END;

CREATE TRIGGER workflow_attempt_resume_claim_update_forbidden
BEFORE UPDATE ON workflow_attempt_resume_claims
BEGIN
    SELECT RAISE(ABORT, 'workflow.attempt_resume_claim.immutable');
END;

CREATE TRIGGER workflow_attempt_resume_claim_delete_forbidden
BEFORE DELETE ON workflow_attempt_resume_claims
BEGIN
    SELECT RAISE(ABORT, 'workflow.attempt_resume_claim.immutable');
END;

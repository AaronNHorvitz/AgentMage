ALTER TABLE workflow_attempts ADD COLUMN receipt_sha256 TEXT
    CHECK(receipt_sha256 IS NULL OR length(receipt_sha256) = 64);

UPDATE workflow_attempts
SET receipt_sha256 = (
    SELECT terminal_receipt.receipt_sha256
    FROM workflow_receipts terminal_receipt
    WHERE terminal_receipt.attempt_id = workflow_attempts.attempt_id
      AND terminal_receipt.receipt_id = workflow_attempts.receipt_id
      AND terminal_receipt.outcome = workflow_attempts.state
      AND terminal_receipt.run_id = workflow_attempts.run_id
      AND terminal_receipt.session_id = workflow_attempts.session_id
)
WHERE state <> 'started';

CREATE UNIQUE INDEX workflow_attempt_step_ordinal_uq
    ON workflow_attempts(step_execution_id, attempt_ordinal);

CREATE UNIQUE INDEX workflow_idempotency_key_uq
    ON workflow_idempotency_keys(key_sha256);

CREATE UNIQUE INDEX workflow_receipt_identity_uq
    ON workflow_receipts(receipt_id);

CREATE UNIQUE INDEX workflow_receipt_attempt_uq
    ON workflow_receipts(attempt_id);

CREATE TABLE workflow_attempt_invariant_migration_guard (
    valid INTEGER NOT NULL CHECK(valid = 1)
) STRICT;

INSERT INTO workflow_attempt_invariant_migration_guard(valid)
SELECT CASE WHEN EXISTS (
    SELECT 1
    FROM workflow_attempts attempt
    WHERE
        (
            attempt.state = 'started'
            AND (attempt.receipt_id IS NOT NULL OR attempt.receipt_sha256 IS NOT NULL)
        )
        OR
        (
            attempt.state <> 'started'
            AND (
                attempt.receipt_id IS NULL
                OR attempt.receipt_sha256 IS NULL
                OR NOT EXISTS (
                    SELECT 1
                    FROM workflow_receipts terminal_receipt
                    JOIN receipts authority_receipt
                      ON authority_receipt.receipt_id = terminal_receipt.receipt_id
                     AND authority_receipt.receipt_sha256 = terminal_receipt.receipt_sha256
                    WHERE terminal_receipt.attempt_id = attempt.attempt_id
                      AND terminal_receipt.receipt_id = attempt.receipt_id
                      AND terminal_receipt.receipt_sha256 = attempt.receipt_sha256
                      AND terminal_receipt.outcome = attempt.state
                      AND terminal_receipt.run_id = attempt.run_id
                      AND terminal_receipt.session_id = attempt.session_id
                )
            )
        )
        OR
        (
            attempt.attempt_ordinal = 1
            AND attempt.supersedes_attempt_id IS NOT NULL
        )
        OR
        (
            attempt.attempt_ordinal > 1
            AND NOT EXISTS (
                SELECT 1
                FROM workflow_attempts prior
                WHERE prior.step_execution_id = attempt.step_execution_id
                  AND prior.attempt_ordinal = attempt.attempt_ordinal - 1
                  AND prior.attempt_id = attempt.supersedes_attempt_id
                  AND prior.state <> 'started'
            )
        )
) THEN 0 ELSE 1 END;

DROP TABLE workflow_attempt_invariant_migration_guard;

CREATE TRIGGER workflow_attempt_insert_started
BEFORE INSERT ON workflow_attempts
WHEN NEW.state <> 'started'
    OR NEW.receipt_id IS NOT NULL
    OR NEW.receipt_sha256 IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'workflow.attempt.must_start_without_receipt');
END;

CREATE TRIGGER workflow_attempt_insert_chain
BEFORE INSERT ON workflow_attempts
WHEN
    (
        NEW.attempt_ordinal = 1
        AND (
            NEW.supersedes_attempt_id IS NOT NULL
            OR EXISTS (
                SELECT 1 FROM workflow_attempts
                WHERE step_execution_id = NEW.step_execution_id
            )
        )
    )
    OR
    (
        NEW.attempt_ordinal > 1
        AND (
            NEW.supersedes_attempt_id IS NULL
            OR NOT EXISTS (
                SELECT 1
                FROM workflow_attempts prior
                WHERE prior.step_execution_id = NEW.step_execution_id
                  AND prior.attempt_ordinal = NEW.attempt_ordinal - 1
                  AND prior.attempt_id = NEW.supersedes_attempt_id
                  AND prior.state <> 'started'
            )
            OR EXISTS (
                SELECT 1 FROM workflow_attempts existing
                WHERE existing.step_execution_id = NEW.step_execution_id
                  AND existing.attempt_ordinal >= NEW.attempt_ordinal
            )
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'workflow.attempt.chain_mismatch');
END;

CREATE TRIGGER workflow_receipt_exact_existing_authority
BEFORE INSERT ON workflow_receipts
WHEN
    NOT EXISTS (
        SELECT 1
        FROM receipts authority_receipt
        WHERE authority_receipt.receipt_id = NEW.receipt_id
          AND authority_receipt.receipt_sha256 = NEW.receipt_sha256
    )
    OR NOT EXISTS (
        SELECT 1
        FROM workflow_attempts attempt
        WHERE attempt.attempt_id = NEW.attempt_id
          AND attempt.run_id = NEW.run_id
          AND attempt.session_id = NEW.session_id
    )
BEGIN
    SELECT RAISE(ABORT, 'workflow.receipt.authority_mismatch');
END;

CREATE TRIGGER workflow_attempt_terminal_transition
BEFORE UPDATE ON workflow_attempts
WHEN
    OLD.state <> 'started'
    OR NEW.state = 'started'
    OR NEW.attempt_id <> OLD.attempt_id
    OR NEW.step_execution_id <> OLD.step_execution_id
    OR NEW.call_id <> OLD.call_id
    OR NEW.attempt_ordinal <> OLD.attempt_ordinal
    OR NEW.supersedes_attempt_id IS NOT OLD.supersedes_attempt_id
    OR NEW.grant_id <> OLD.grant_id
    OR NEW.run_id <> OLD.run_id
    OR NEW.session_id <> OLD.session_id
    OR NEW.event_sequence <> OLD.event_sequence
    OR NEW.event_id <> OLD.event_id
    OR NEW.receipt_id IS NULL
    OR NEW.receipt_sha256 IS NULL
    OR NOT EXISTS (
        SELECT 1
        FROM workflow_receipts terminal_receipt
        WHERE terminal_receipt.attempt_id = OLD.attempt_id
          AND terminal_receipt.receipt_id = NEW.receipt_id
          AND terminal_receipt.receipt_sha256 = NEW.receipt_sha256
          AND terminal_receipt.outcome = NEW.state
          AND terminal_receipt.run_id = OLD.run_id
          AND terminal_receipt.session_id = OLD.session_id
    )
BEGIN
    SELECT RAISE(ABORT, 'workflow.attempt.terminal_mismatch');
END;

CREATE TRIGGER workflow_attempt_delete_forbidden
BEFORE DELETE ON workflow_attempts
BEGIN
    SELECT RAISE(ABORT, 'workflow.attempt.append_only');
END;

CREATE TRIGGER workflow_receipt_update_forbidden
BEFORE UPDATE ON workflow_receipts
BEGIN
    SELECT RAISE(ABORT, 'workflow.receipt.append_only');
END;

CREATE TRIGGER workflow_receipt_delete_forbidden
BEFORE DELETE ON workflow_receipts
BEGIN
    SELECT RAISE(ABORT, 'workflow.receipt.append_only');
END;

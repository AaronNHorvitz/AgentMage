CREATE TABLE answer_claim_ledgers (
    ledger_sha256 TEXT PRIMARY KEY CHECK(length(ledger_sha256) = 64),
    task_id TEXT NOT NULL,
    created_at_epoch_ms INTEGER NOT NULL CHECK(created_at_epoch_ms > 0),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304)
) STRICT;

CREATE INDEX answer_claim_ledgers_task_idx
    ON answer_claim_ledgers(task_id, created_at_epoch_ms, ledger_sha256);

CREATE TABLE receipt_integrity_ledgers (
    scope_id TEXT PRIMARY KEY,
    receipt_count INTEGER NOT NULL CHECK(receipt_count > 0),
    ledger_sha256 TEXT NOT NULL CHECK(length(ledger_sha256) = 64),
    ledger_json BLOB NOT NULL CHECK(length(ledger_json) > 0 AND length(ledger_json) <= 67108864)
) STRICT;

CREATE TABLE receipt_integrity_anchors (
    scope_id TEXT NOT NULL,
    receipt_count INTEGER NOT NULL CHECK(receipt_count > 0),
    head_receipt_sha256 TEXT NOT NULL CHECK(length(head_receipt_sha256) = 64),
    anchor_hmac_sha256 TEXT NOT NULL CHECK(length(anchor_hmac_sha256) = 64),
    observed_at_epoch_ms INTEGER NOT NULL CHECK(observed_at_epoch_ms > 0),
    previous_anchor_sha256 TEXT NOT NULL CHECK(length(previous_anchor_sha256) = 64),
    anchor_sha256 TEXT NOT NULL UNIQUE CHECK(length(anchor_sha256) = 64),
    PRIMARY KEY(scope_id, receipt_count),
    FOREIGN KEY(scope_id) REFERENCES receipt_integrity_ledgers(scope_id) ON DELETE CASCADE
) STRICT;

CREATE INDEX receipt_integrity_anchors_timeline_idx
    ON receipt_integrity_anchors(scope_id, observed_at_epoch_ms, receipt_count);

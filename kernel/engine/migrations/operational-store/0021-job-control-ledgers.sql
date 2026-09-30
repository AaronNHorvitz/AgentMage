-- Durable job control ledgers (CAP-35, Decision 0118). The owner appends each
-- entry in the same transaction that advances its job's head, so a committed
-- entry never exists without the entry count and last digest that end it.
CREATE TABLE job_ledger_roots (
    job_id TEXT PRIMARY KEY CHECK(length(job_id) > 0 AND length(job_id) <= 128),
    owner_id TEXT NOT NULL CHECK(length(owner_id) > 0 AND length(owner_id) <= 128),
    created_sha256 TEXT NOT NULL UNIQUE CHECK(length(created_sha256) = 64)
) STRICT;

CREATE TABLE job_ledger_entries (
    job_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK(sequence >= 0 AND sequence < 4096),
    prior_entry_sha256 TEXT NOT NULL CHECK(length(prior_entry_sha256) = 64),
    entry_sha256 TEXT NOT NULL UNIQUE CHECK(length(entry_sha256) = 64),
    entry_json BLOB NOT NULL CHECK(length(entry_json) > 0 AND length(entry_json) <= 4096),
    PRIMARY KEY(job_id, sequence),
    UNIQUE(job_id, sequence, entry_sha256),
    FOREIGN KEY(job_id) REFERENCES job_ledger_roots(job_id)
) STRICT;

CREATE TABLE job_ledger_heads (
    job_id TEXT PRIMARY KEY,
    entry_count INTEGER NOT NULL CHECK(entry_count > 0 AND entry_count <= 4096),
    last_sequence INTEGER NOT NULL CHECK(last_sequence = entry_count - 1),
    head_sha256 TEXT NOT NULL CHECK(length(head_sha256) = 64),
    FOREIGN KEY(job_id, last_sequence, head_sha256)
        REFERENCES job_ledger_entries(job_id, sequence, entry_sha256)
) STRICT;

CREATE TRIGGER job_ledger_root_update_forbidden
BEFORE UPDATE ON job_ledger_roots
BEGIN
    SELECT RAISE(ABORT, 'job.ledger.root.immutable');
END;

CREATE TRIGGER job_ledger_root_delete_forbidden
BEFORE DELETE ON job_ledger_roots
BEGIN
    SELECT RAISE(ABORT, 'job.ledger.root.immutable');
END;

CREATE TRIGGER job_ledger_entry_update_forbidden
BEFORE UPDATE ON job_ledger_entries
BEGIN
    SELECT RAISE(ABORT, 'job.ledger.entry.append_only');
END;

CREATE TRIGGER job_ledger_entry_delete_forbidden
BEFORE DELETE ON job_ledger_entries
BEGIN
    SELECT RAISE(ABORT, 'job.ledger.entry.append_only');
END;

CREATE TRIGGER job_ledger_head_delete_forbidden
BEFORE DELETE ON job_ledger_heads
BEGIN
    SELECT RAISE(ABORT, 'job.ledger.head.required');
END;

CREATE TRIGGER job_ledger_head_advance_only
BEFORE UPDATE ON job_ledger_heads
WHEN NEW.job_id != OLD.job_id OR NEW.entry_count <= OLD.entry_count
BEGIN
    SELECT RAISE(ABORT, 'job.ledger.head.advance_only');
END;

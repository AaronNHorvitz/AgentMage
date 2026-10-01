-- Durable run action histories (CAP-42, Decision 0129). Each owner appends a
-- record in the same transaction that advances its chain's head, so a
-- committed record never exists without the count and digest that end it.
-- A record may only change from kept to expired, a head only moves forward,
-- and a closed head never changes again.
CREATE TABLE run_action_history_roots (
    run_id TEXT NOT NULL CHECK(length(run_id) > 0 AND length(run_id) <= 128),
    chain TEXT NOT NULL CHECK(chain IN ('effects', 'job-control', 'routes')),
    owner_id TEXT NOT NULL CHECK(length(owner_id) > 0 AND length(owner_id) <= 128),
    PRIMARY KEY(run_id, chain)
) STRICT;

CREATE TABLE run_action_history_records (
    run_id TEXT NOT NULL,
    chain TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK(sequence >= 1 AND sequence <= 512),
    entry_sha256 TEXT NOT NULL CHECK(length(entry_sha256) = 64),
    expired INTEGER NOT NULL CHECK(expired IN (0, 1)),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 8192),
    PRIMARY KEY(run_id, chain, sequence),
    FOREIGN KEY(run_id, chain) REFERENCES run_action_history_roots(run_id, chain)
) STRICT;

CREATE TABLE run_action_history_heads (
    run_id TEXT NOT NULL,
    chain TEXT NOT NULL,
    entry_count INTEGER NOT NULL CHECK(entry_count >= 0 AND entry_count <= 512),
    head_sha256 TEXT NOT NULL CHECK(length(head_sha256) = 64),
    complete INTEGER NOT NULL CHECK(complete IN (0, 1)),
    closed INTEGER NOT NULL CHECK(closed IN (0, 1)),
    PRIMARY KEY(run_id, chain),
    FOREIGN KEY(run_id, chain) REFERENCES run_action_history_roots(run_id, chain)
) STRICT;

CREATE TRIGGER run_action_history_root_update_forbidden
BEFORE UPDATE ON run_action_history_roots
BEGIN
    SELECT RAISE(ABORT, 'run.action.history.root.immutable');
END;

CREATE TRIGGER run_action_history_root_delete_forbidden
BEFORE DELETE ON run_action_history_roots
BEGIN
    SELECT RAISE(ABORT, 'run.action.history.root.immutable');
END;

CREATE TRIGGER run_action_history_record_delete_forbidden
BEFORE DELETE ON run_action_history_records
BEGIN
    SELECT RAISE(ABORT, 'run.action.history.record.append_only');
END;

CREATE TRIGGER run_action_history_record_expire_only
BEFORE UPDATE ON run_action_history_records
WHEN NEW.run_id != OLD.run_id
    OR NEW.chain != OLD.chain
    OR NEW.sequence != OLD.sequence
    OR NEW.entry_sha256 != OLD.entry_sha256
    OR OLD.expired != 0
    OR NEW.expired != 1
BEGIN
    SELECT RAISE(ABORT, 'run.action.history.record.expire_only');
END;

CREATE TRIGGER run_action_history_head_delete_forbidden
BEFORE DELETE ON run_action_history_heads
BEGIN
    SELECT RAISE(ABORT, 'run.action.history.head.required');
END;

CREATE TRIGGER run_action_history_head_forward_only
BEFORE UPDATE ON run_action_history_heads
WHEN NEW.run_id != OLD.run_id
    OR NEW.chain != OLD.chain
    OR NEW.entry_count < OLD.entry_count
    OR NEW.entry_count > OLD.entry_count + 1
    OR (NEW.entry_count = OLD.entry_count AND NEW.head_sha256 != OLD.head_sha256)
    OR NEW.complete > OLD.complete
    OR NEW.closed < OLD.closed
    OR (OLD.closed = 1 AND (NEW.entry_count != OLD.entry_count OR NEW.complete != OLD.complete))
BEGIN
    SELECT RAISE(ABORT, 'run.action.history.head.forward_only');
END;

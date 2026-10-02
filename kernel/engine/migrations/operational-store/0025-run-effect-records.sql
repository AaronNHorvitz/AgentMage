-- Durable run effect records (Decision 0143). The effect owner of a coding
-- run appends what each executed call could do and how it ended, and each
-- change record it published, in the same transaction that advances the
-- run's head, so a committed record never exists without the count and
-- digest that end it. Each run belongs to one session at a fixed position.
-- Records are append-only, a head only moves forward to the record it names,
-- a close moves no count, and a closed head never changes again.
CREATE TABLE run_effect_record_roots (
    run_id TEXT NOT NULL PRIMARY KEY CHECK(length(run_id) > 0 AND length(run_id) <= 128),
    session_id TEXT NOT NULL CHECK(length(session_id) > 0 AND length(session_id) <= 128),
    task_id TEXT NOT NULL CHECK(length(task_id) > 0 AND length(task_id) <= 128),
    owner_id TEXT NOT NULL CHECK(length(owner_id) > 0 AND length(owner_id) <= 128),
    session_position INTEGER NOT NULL CHECK(session_position >= 1 AND session_position <= 64),
    UNIQUE(session_id, session_position)
) STRICT;

CREATE TABLE run_effect_records (
    run_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK(sequence >= 1 AND sequence <= 1024),
    entry_sha256 TEXT NOT NULL CHECK(length(entry_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4096),
    PRIMARY KEY(run_id, sequence),
    FOREIGN KEY(run_id) REFERENCES run_effect_record_roots(run_id)
) STRICT;

CREATE TABLE run_effect_record_heads (
    run_id TEXT NOT NULL PRIMARY KEY,
    entry_count INTEGER NOT NULL CHECK(entry_count >= 0 AND entry_count <= 1024),
    head_sha256 TEXT NOT NULL CHECK(length(head_sha256) = 64),
    complete INTEGER NOT NULL CHECK(complete IN (0, 1)),
    closed INTEGER NOT NULL CHECK(closed IN (0, 1)),
    FOREIGN KEY(run_id) REFERENCES run_effect_record_roots(run_id)
) STRICT;

CREATE TRIGGER run_effect_record_root_update_forbidden
BEFORE UPDATE ON run_effect_record_roots
BEGIN
    SELECT RAISE(ABORT, 'run.effect.record.root.immutable');
END;

CREATE TRIGGER run_effect_record_root_delete_forbidden
BEFORE DELETE ON run_effect_record_roots
BEGIN
    SELECT RAISE(ABORT, 'run.effect.record.root.immutable');
END;

CREATE TRIGGER run_effect_record_update_forbidden
BEFORE UPDATE ON run_effect_records
BEGIN
    SELECT RAISE(ABORT, 'run.effect.record.append_only');
END;

CREATE TRIGGER run_effect_record_delete_forbidden
BEFORE DELETE ON run_effect_records
BEGIN
    SELECT RAISE(ABORT, 'run.effect.record.append_only');
END;

CREATE TRIGGER run_effect_record_head_delete_forbidden
BEFORE DELETE ON run_effect_record_heads
BEGIN
    SELECT RAISE(ABORT, 'run.effect.record.head.required');
END;

CREATE TRIGGER run_effect_record_head_forward_only
BEFORE UPDATE ON run_effect_record_heads
WHEN NEW.run_id != OLD.run_id
    OR NEW.entry_count < OLD.entry_count
    OR NEW.entry_count > OLD.entry_count + 1
    OR (NEW.entry_count = OLD.entry_count AND NEW.head_sha256 != OLD.head_sha256)
    OR NEW.complete > OLD.complete
    OR NEW.closed < OLD.closed
    OR (OLD.closed = 1 AND (NEW.entry_count != OLD.entry_count OR NEW.complete != OLD.complete))
    OR (NEW.closed != OLD.closed AND NEW.entry_count != OLD.entry_count)
    OR (NEW.entry_count = OLD.entry_count + 1 AND NOT EXISTS (
        SELECT 1 FROM run_effect_records
        WHERE run_id = NEW.run_id AND sequence = NEW.entry_count
          AND entry_sha256 = NEW.head_sha256))
BEGIN
    SELECT RAISE(ABORT, 'run.effect.record.head.forward_only');
END;

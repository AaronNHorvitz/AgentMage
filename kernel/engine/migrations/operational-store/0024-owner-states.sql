-- Durable owner states (Decision 0131). Each owner commits its complete next
-- state under the revision it read: a first state has revision one, each
-- later state raises the revision by exactly one and keeps the owner's
-- identity, and a state is never deleted. The store checks each state's
-- digest; the owner decodes and re-verifies its meaning.
CREATE TABLE owner_states (
    owner_id TEXT NOT NULL PRIMARY KEY CHECK(length(owner_id) > 0 AND length(owner_id) <= 64),
    revision INTEGER NOT NULL CHECK(revision >= 1),
    state BLOB NOT NULL CHECK(length(state) > 0 AND length(state) <= 67108864),
    state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64)
) STRICT;

CREATE TRIGGER owner_state_insert_first_revision
BEFORE INSERT ON owner_states
WHEN NEW.revision != 1
BEGIN
    SELECT RAISE(ABORT, 'owner.state.forward_only');
END;

CREATE TRIGGER owner_state_forward_only
BEFORE UPDATE ON owner_states
WHEN NEW.owner_id != OLD.owner_id
    OR NEW.revision != OLD.revision + 1
BEGIN
    SELECT RAISE(ABORT, 'owner.state.forward_only');
END;

CREATE TRIGGER owner_state_delete_forbidden
BEFORE DELETE ON owner_states
BEGIN
    SELECT RAISE(ABORT, 'owner.state.required');
END;

CREATE TABLE source_materialization_states (
    source_artifact_id TEXT PRIMARY KEY,
    lifecycle_state TEXT NOT NULL CHECK(
        lifecycle_state IN ('current', 'stale', 'released', 'deleted')
    ),
    revision INTEGER NOT NULL CHECK(revision >= 0),
    head_event_sha256 TEXT NOT NULL CHECK(length(head_event_sha256) = 64),
    updated_at_epoch_ms INTEGER NOT NULL CHECK(updated_at_epoch_ms >= 0),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id)
) STRICT;

INSERT INTO source_materialization_states(
    source_artifact_id, lifecycle_state, revision, head_event_sha256, updated_at_epoch_ms
)
SELECT source_artifact_id, 'current', 0,
       '0000000000000000000000000000000000000000000000000000000000000000', 0
FROM source_manifests;

CREATE TRIGGER source_materialization_state_insert
AFTER INSERT ON source_manifests
BEGIN
    INSERT INTO source_materialization_states(
        source_artifact_id, lifecycle_state, revision, head_event_sha256, updated_at_epoch_ms
    ) VALUES (
        NEW.source_artifact_id, 'current', 0,
        '0000000000000000000000000000000000000000000000000000000000000000', 0
    );
END;

CREATE TABLE source_materialization_events (
    source_artifact_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    lifecycle_state TEXT NOT NULL CHECK(
        lifecycle_state IN ('stale', 'released', 'deleted')
    ),
    reason_code TEXT NOT NULL,
    occurred_at_epoch_ms INTEGER NOT NULL CHECK(occurred_at_epoch_ms >= 0),
    previous_event_sha256 TEXT NOT NULL CHECK(length(previous_event_sha256) = 64),
    event_sha256 TEXT NOT NULL UNIQUE CHECK(length(event_sha256) = 64),
    PRIMARY KEY(source_artifact_id, revision),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id)
) STRICT;

CREATE TABLE source_dependencies (
    upstream_source_artifact_id TEXT NOT NULL,
    dependent_source_artifact_id TEXT NOT NULL,
    dependency_sha256 TEXT NOT NULL UNIQUE CHECK(length(dependency_sha256) = 64),
    recorded_at_epoch_ms INTEGER NOT NULL CHECK(recorded_at_epoch_ms >= 0),
    PRIMARY KEY(upstream_source_artifact_id, dependent_source_artifact_id),
    FOREIGN KEY(upstream_source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    FOREIGN KEY(dependent_source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    CHECK(upstream_source_artifact_id <> dependent_source_artifact_id)
) STRICT;

CREATE TRIGGER source_dependencies_cycle_insert
BEFORE INSERT ON source_dependencies
WHEN EXISTS (
    WITH RECURSIVE downstream(source_artifact_id) AS (
        SELECT NEW.dependent_source_artifact_id
        UNION
        SELECT d.dependent_source_artifact_id
        FROM source_dependencies d
        JOIN downstream x ON d.upstream_source_artifact_id = x.source_artifact_id
    )
    SELECT 1 FROM downstream WHERE source_artifact_id = NEW.upstream_source_artifact_id
)
BEGIN
    SELECT RAISE(ABORT, 'source dependency cycle');
END;

CREATE TRIGGER source_dependencies_update
BEFORE UPDATE ON source_dependencies
BEGIN
    SELECT RAISE(ABORT, 'source dependencies are immutable');
END;

CREATE TABLE source_refreshes (
    refresh_id TEXT PRIMARY KEY,
    replaced_source_artifact_id TEXT NOT NULL UNIQUE,
    replacement_source_artifact_id TEXT NOT NULL UNIQUE,
    invalidated_source_count INTEGER NOT NULL CHECK(invalidated_source_count > 0),
    reason_code TEXT NOT NULL,
    occurred_at_epoch_ms INTEGER NOT NULL CHECK(occurred_at_epoch_ms >= 0),
    refresh_sha256 TEXT NOT NULL UNIQUE CHECK(length(refresh_sha256) = 64),
    FOREIGN KEY(replaced_source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    FOREIGN KEY(replacement_source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    CHECK(replaced_source_artifact_id <> replacement_source_artifact_id)
) STRICT;

CREATE TRIGGER source_refreshes_identity_insert
BEFORE INSERT ON source_refreshes
WHEN NOT EXISTS (
    SELECT 1
    FROM source_manifests old
    JOIN source_manifests new
      ON new.source_artifact_id = NEW.replacement_source_artifact_id
    WHERE old.source_artifact_id = NEW.replaced_source_artifact_id
      AND old.request_id = new.request_id
      AND old.authority_id = new.authority_id
      AND old.reference_id = new.reference_id
)
BEGIN
    SELECT RAISE(ABORT, 'source refresh identity mismatch');
END;

CREATE TRIGGER source_refreshes_update
BEFORE UPDATE ON source_refreshes
BEGIN
    SELECT RAISE(ABORT, 'source refreshes are immutable');
END;

CREATE TABLE source_retention_deadlines (
    retention_id TEXT PRIMARY KEY,
    expires_at_epoch_ms INTEGER NOT NULL CHECK(expires_at_epoch_ms > 0),
    deadline_sha256 TEXT NOT NULL UNIQUE CHECK(length(deadline_sha256) = 64),
    FOREIGN KEY(retention_id) REFERENCES source_retentions(retention_id)
) STRICT;

CREATE TRIGGER source_retention_deadlines_insert
BEFORE INSERT ON source_retention_deadlines
WHEN NOT EXISTS (
    SELECT 1 FROM source_retentions
    WHERE retention_id = NEW.retention_id
      AND retention_class = 'policy_persisted'
      AND lifecycle_state = 'active'
)
BEGIN
    SELECT RAISE(ABORT, 'source retention deadline mismatch');
END;

CREATE TRIGGER source_retention_deadlines_update
BEFORE UPDATE ON source_retention_deadlines
BEGIN
    SELECT RAISE(ABORT, 'source retention deadlines are immutable');
END;

CREATE TABLE source_retention_holds (
    retention_id TEXT NOT NULL,
    hold_kind TEXT NOT NULL CHECK(hold_kind IN ('user', 'legal')),
    active INTEGER NOT NULL CHECK(active IN (0, 1)),
    revision INTEGER NOT NULL CHECK(revision > 0),
    head_event_sha256 TEXT NOT NULL CHECK(length(head_event_sha256) = 64),
    updated_at_epoch_ms INTEGER NOT NULL CHECK(updated_at_epoch_ms >= 0),
    PRIMARY KEY(retention_id, hold_kind),
    FOREIGN KEY(retention_id) REFERENCES source_retentions(retention_id)
) STRICT;

CREATE TABLE source_retention_hold_events (
    retention_id TEXT NOT NULL,
    hold_kind TEXT NOT NULL CHECK(hold_kind IN ('user', 'legal')),
    revision INTEGER NOT NULL CHECK(revision > 0),
    action TEXT NOT NULL CHECK(action IN ('applied', 'released')),
    reason_code TEXT NOT NULL,
    occurred_at_epoch_ms INTEGER NOT NULL CHECK(occurred_at_epoch_ms >= 0),
    previous_event_sha256 TEXT NOT NULL CHECK(length(previous_event_sha256) = 64),
    event_sha256 TEXT NOT NULL UNIQUE CHECK(length(event_sha256) = 64),
    PRIMARY KEY(retention_id, hold_kind, revision),
    FOREIGN KEY(retention_id, hold_kind)
        REFERENCES source_retention_holds(retention_id, hold_kind)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TABLE source_released_payloads (
    retention_id TEXT PRIMARY KEY,
    source_artifact_id TEXT NOT NULL UNIQUE,
    physical_artifact_id TEXT NOT NULL UNIQUE,
    payload_sha256 TEXT NOT NULL CHECK(length(payload_sha256) = 64),
    byte_size INTEGER NOT NULL CHECK(byte_size >= 0 AND byte_size <= 104857600),
    released_at_epoch_ms INTEGER NOT NULL CHECK(released_at_epoch_ms >= 0),
    deleted_at_epoch_ms INTEGER CHECK(
        deleted_at_epoch_ms IS NULL OR deleted_at_epoch_ms >= released_at_epoch_ms
    ),
    release_sha256 TEXT NOT NULL UNIQUE CHECK(length(release_sha256) = 64),
    FOREIGN KEY(retention_id) REFERENCES source_retentions(retention_id),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    FOREIGN KEY(physical_artifact_id, payload_sha256, byte_size)
        REFERENCES runtime_artifacts(artifact_id, payload_sha256, byte_size)
) STRICT;

CREATE VIEW current_source_sections AS
SELECT s.*
FROM source_sections s
JOIN source_materialization_states m USING(source_artifact_id)
WHERE m.lifecycle_state = 'current';

CREATE VIEW current_source_extractions AS
SELECT e.*
FROM source_extractions e
JOIN source_materialization_states m USING(source_artifact_id)
WHERE m.lifecycle_state = 'current';

CREATE VIEW current_source_cache_inputs AS
SELECT c.*
FROM source_cache_inputs c
JOIN source_materialization_states m USING(source_artifact_id)
WHERE m.lifecycle_state = 'current';

CREATE VIEW current_source_lexical_indexes AS
SELECT i.*
FROM source_lexical_indexes i
JOIN source_materialization_states m USING(source_artifact_id)
WHERE m.lifecycle_state = 'current';

CREATE VIEW current_source_context_dispositions AS
SELECT d.*
FROM source_context_dispositions d
JOIN source_materialization_states m USING(source_artifact_id)
WHERE m.lifecycle_state = 'current';

CREATE INDEX source_dependencies_dependent_idx
    ON source_dependencies(dependent_source_artifact_id, upstream_source_artifact_id);
CREATE INDEX source_materialization_states_lifecycle_idx
    ON source_materialization_states(lifecycle_state, source_artifact_id);
CREATE INDEX source_retention_holds_active_idx
    ON source_retention_holds(active, retention_id, hold_kind);
CREATE INDEX source_released_payloads_payload_idx
    ON source_released_payloads(payload_sha256, deleted_at_epoch_ms, retention_id);

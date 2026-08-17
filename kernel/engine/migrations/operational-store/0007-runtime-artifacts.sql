CREATE TABLE runtime_payloads (
    payload_sha256 TEXT PRIMARY KEY CHECK(length(payload_sha256) = 64),
    byte_size INTEGER NOT NULL CHECK(byte_size > 0),
    lifecycle_state TEXT NOT NULL CHECK(
        lifecycle_state IN ('active', 'quarantined', 'deleted')
    ),
    active_reference_count INTEGER NOT NULL CHECK(active_reference_count >= 0),
    created_at_epoch_ms INTEGER NOT NULL CHECK(created_at_epoch_ms > 0),
    updated_at_epoch_ms INTEGER NOT NULL CHECK(updated_at_epoch_ms >= created_at_epoch_ms)
) STRICT;

CREATE TABLE runtime_artifacts (
    artifact_id TEXT PRIMARY KEY,
    manifest_sha256 TEXT NOT NULL UNIQUE CHECK(length(manifest_sha256) = 64),
    payload_sha256 TEXT NOT NULL CHECK(length(payload_sha256) = 64),
    byte_size INTEGER NOT NULL CHECK(byte_size > 0),
    media_type TEXT NOT NULL,
    artifact_kind TEXT NOT NULL CHECK(
        artifact_kind IN (
            'patch', 'standard_output', 'standard_error', 'test_log',
            'generated_file', 'report', 'model_output'
        )
    ),
    sensitivity TEXT NOT NULL CHECK(
        sensitivity IN ('public', 'internal', 'private', 'restricted')
    ),
    retention_kind TEXT NOT NULL CHECK(
        retention_kind IN ('session', 'until_expiration', 'user_hold')
    ),
    retention_expires_at_epoch_ms INTEGER CHECK(
        retention_expires_at_epoch_ms IS NULL
        OR retention_expires_at_epoch_ms > created_at_epoch_ms
    ),
    session_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    producer_run_id TEXT NOT NULL,
    producer_turn_id TEXT,
    producer_operation_id TEXT,
    receipt_id TEXT,
    policy_id TEXT NOT NULL,
    policy_sha256 TEXT NOT NULL CHECK(length(policy_sha256) = 64),
    created_at_epoch_ms INTEGER NOT NULL CHECK(created_at_epoch_ms > 0),
    manifest_json BLOB NOT NULL,
    UNIQUE(artifact_id, manifest_sha256),
    FOREIGN KEY(payload_sha256) REFERENCES runtime_payloads(payload_sha256),
    FOREIGN KEY(producer_run_id) REFERENCES runtime_runs(run_id),
    CHECK(receipt_id IS NULL OR producer_operation_id IS NOT NULL),
    CHECK(producer_operation_id IS NULL OR producer_turn_id IS NOT NULL)
) STRICT;

CREATE TABLE runtime_artifact_states (
    artifact_id TEXT PRIMARY KEY,
    revision INTEGER NOT NULL CHECK(revision > 0),
    lifecycle_state TEXT NOT NULL CHECK(
        lifecycle_state IN ('active', 'quarantined', 'released', 'deleted')
    ),
    integrity_state TEXT NOT NULL CHECK(
        integrity_state IN ('verified', 'quarantined', 'missing', 'corrupt', 'deleted')
    ),
    reason_code TEXT NOT NULL,
    updated_at_epoch_ms INTEGER NOT NULL CHECK(updated_at_epoch_ms > 0),
    state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64),
    FOREIGN KEY(artifact_id) REFERENCES runtime_artifacts(artifact_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE runtime_artifact_events (
    artifact_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    lifecycle_state TEXT NOT NULL CHECK(
        lifecycle_state IN ('active', 'quarantined', 'released', 'deleted')
    ),
    integrity_state TEXT NOT NULL CHECK(
        integrity_state IN ('verified', 'quarantined', 'missing', 'corrupt', 'deleted')
    ),
    reason_code TEXT NOT NULL,
    occurred_at_epoch_ms INTEGER NOT NULL CHECK(occurred_at_epoch_ms > 0),
    previous_event_sha256 TEXT NOT NULL CHECK(length(previous_event_sha256) = 64),
    state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64),
    event_sha256 TEXT NOT NULL UNIQUE CHECK(length(event_sha256) = 64),
    PRIMARY KEY(artifact_id, revision),
    FOREIGN KEY(artifact_id) REFERENCES runtime_artifacts(artifact_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE runtime_resume_bindings (
    checkpoint_sha256 TEXT PRIMARY KEY CHECK(length(checkpoint_sha256) = 64),
    checkpoint_id TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    event_sha256 TEXT NOT NULL CHECK(length(event_sha256) = 64),
    binding_sha256 TEXT NOT NULL UNIQUE CHECK(length(binding_sha256) = 64),
    record_json BLOB NOT NULL,
    FOREIGN KEY(checkpoint_sha256) REFERENCES session_checkpoints(checkpoint_sha256),
    FOREIGN KEY(run_id, event_sequence) REFERENCES runtime_events(run_id, sequence)
) STRICT;

CREATE TABLE runtime_resume_artifacts (
    checkpoint_sha256 TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    artifact_id TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL CHECK(length(manifest_sha256) = 64),
    payload_sha256 TEXT NOT NULL CHECK(length(payload_sha256) = 64),
    byte_size INTEGER NOT NULL CHECK(byte_size > 0),
    media_type TEXT NOT NULL,
    PRIMARY KEY(checkpoint_sha256, ordinal),
    UNIQUE(checkpoint_sha256, artifact_id),
    FOREIGN KEY(checkpoint_sha256)
        REFERENCES runtime_resume_bindings(checkpoint_sha256) ON DELETE CASCADE,
    FOREIGN KEY(artifact_id, manifest_sha256)
        REFERENCES runtime_artifacts(artifact_id, manifest_sha256)
) STRICT;

CREATE INDEX runtime_artifacts_payload_idx
    ON runtime_artifacts(payload_sha256, artifact_id);
CREATE INDEX runtime_artifacts_session_idx
    ON runtime_artifacts(session_id, task_id, created_at_epoch_ms, artifact_id);
CREATE INDEX runtime_artifact_events_timeline_idx
    ON runtime_artifact_events(occurred_at_epoch_ms, artifact_id, revision);
CREATE INDEX runtime_resume_bindings_run_idx
    ON runtime_resume_bindings(run_id, event_sequence, checkpoint_sha256);

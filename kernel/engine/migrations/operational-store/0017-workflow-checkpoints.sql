CREATE TABLE workflow_checkpoints (
    checkpoint_id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64),
    journal_sequence INTEGER NOT NULL CHECK(journal_sequence >= 0),
    source_manifest_sha256 TEXT NOT NULL CHECK(length(source_manifest_sha256) = 64),
    policy_sha256 TEXT NOT NULL CHECK(length(policy_sha256) = 64),
    environment_sha256 TEXT NOT NULL CHECK(length(environment_sha256) = 64),
    route_sha256 TEXT NOT NULL CHECK(length(route_sha256) = 64),
    tool_catalog_sha256 TEXT NOT NULL CHECK(length(tool_catalog_sha256) = 64),
    session_checkpoint_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    event_sequence INTEGER NOT NULL CHECK(event_sequence >= 0),
    event_id TEXT NOT NULL,
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(session_checkpoint_id) REFERENCES session_checkpoints(checkpoint_id),
    FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id),
    FOREIGN KEY(run_id, event_sequence, event_id)
        REFERENCES runtime_events(run_id, sequence, event_id),
    CHECK(journal_sequence = event_sequence)
) STRICT;

CREATE INDEX workflow_checkpoints_runtime_idx
    ON workflow_checkpoints(run_id, event_sequence, workflow_id);

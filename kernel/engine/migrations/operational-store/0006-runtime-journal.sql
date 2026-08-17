CREATE TABLE runtime_runs (
    run_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    request_sha256 TEXT NOT NULL CHECK(length(request_sha256) = 64),
    first_event_sha256 TEXT NOT NULL CHECK(length(first_event_sha256) = 64),
    last_sequence INTEGER NOT NULL CHECK(last_sequence >= 0),
    last_event_id TEXT NOT NULL,
    last_event_sha256 TEXT NOT NULL CHECK(length(last_event_sha256) = 64),
    event_count INTEGER NOT NULL CHECK(event_count > 0),
    terminal INTEGER NOT NULL CHECK(terminal IN (0, 1)),
    terminal_state TEXT,
    terminal_outcome_sha256 TEXT CHECK(
        terminal_outcome_sha256 IS NULL OR length(terminal_outcome_sha256) = 64
    ),
    created_at_epoch_ms INTEGER NOT NULL CHECK(created_at_epoch_ms > 0),
    updated_at_epoch_ms INTEGER NOT NULL CHECK(updated_at_epoch_ms >= created_at_epoch_ms),
    CHECK(
        (terminal = 0 AND terminal_state IS NULL AND terminal_outcome_sha256 IS NULL)
        OR
        (terminal = 1 AND terminal_state IS NOT NULL AND terminal_outcome_sha256 IS NOT NULL)
    )
) STRICT;

CREATE TABLE runtime_events (
    run_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK(sequence >= 0),
    event_id TEXT NOT NULL UNIQUE,
    occurred_at_epoch_ms INTEGER NOT NULL CHECK(occurred_at_epoch_ms > 0),
    persistence_class TEXT NOT NULL CHECK(
        persistence_class IN ('correctness', 'progress', 'metric')
    ),
    sensitivity TEXT NOT NULL CHECK(
        sensitivity IN ('public', 'internal', 'private', 'restricted')
    ),
    retention_kind TEXT NOT NULL CHECK(
        retention_kind IN ('session', 'until_expiration', 'user_hold')
    ),
    retention_expires_at_epoch_ms INTEGER CHECK(
        retention_expires_at_epoch_ms IS NULL OR retention_expires_at_epoch_ms > occurred_at_epoch_ms
    ),
    event_sha256 TEXT NOT NULL UNIQUE CHECK(length(event_sha256) = 64),
    previous_event_sha256 TEXT NOT NULL CHECK(length(previous_event_sha256) = 64),
    payload_artifact_id TEXT,
    payload_sha256 TEXT CHECK(payload_sha256 IS NULL OR length(payload_sha256) = 64),
    payload_byte_size INTEGER CHECK(payload_byte_size IS NULL OR payload_byte_size > 0),
    payload_media_type TEXT,
    record_json BLOB NOT NULL,
    PRIMARY KEY(run_id, sequence),
    FOREIGN KEY(run_id) REFERENCES runtime_runs(run_id) ON DELETE CASCADE,
    CHECK(
        (payload_artifact_id IS NULL AND payload_sha256 IS NULL
            AND payload_byte_size IS NULL AND payload_media_type IS NULL)
        OR
        (payload_artifact_id IS NOT NULL AND payload_sha256 IS NOT NULL
            AND payload_byte_size IS NOT NULL AND payload_media_type IS NOT NULL)
    )
) STRICT;

CREATE INDEX runtime_events_session_timeline_idx
    ON runtime_runs(session_id, updated_at_epoch_ms, run_id);
CREATE INDEX runtime_events_task_timeline_idx
    ON runtime_runs(task_id, updated_at_epoch_ms, run_id);

CREATE TABLE engineering_sessions (
    session_id TEXT PRIMARY KEY,
    snapshot_sha256 TEXT NOT NULL CHECK(length(snapshot_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304)
) STRICT;

CREATE TABLE engineering_events (
    session_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK(sequence >= 0),
    event_id TEXT NOT NULL UNIQUE,
    event_sha256 TEXT NOT NULL UNIQUE CHECK(length(event_sha256) = 64),
    previous_event_sha256 TEXT NOT NULL CHECK(length(previous_event_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    PRIMARY KEY(session_id, sequence),
    FOREIGN KEY(session_id) REFERENCES engineering_sessions(session_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE engineering_artifact_payloads (
    source_sha256 TEXT PRIMARY KEY CHECK(length(source_sha256) = 64),
    payload BLOB NOT NULL CHECK(length(payload) > 0 AND length(payload) <= 67108864)
) STRICT;

CREATE TABLE engineering_artifacts (
    artifact_id TEXT PRIMARY KEY,
    upload_id TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL,
    source_sha256 TEXT NOT NULL,
    receipt_sha256 TEXT NOT NULL UNIQUE CHECK(length(receipt_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(session_id) REFERENCES engineering_sessions(session_id) ON DELETE CASCADE,
    FOREIGN KEY(source_sha256) REFERENCES engineering_artifact_payloads(source_sha256)
) STRICT;

CREATE INDEX engineering_artifacts_session_idx
    ON engineering_artifacts(session_id, artifact_id);

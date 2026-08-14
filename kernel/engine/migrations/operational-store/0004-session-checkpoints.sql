ALTER TABLE store_metadata
ADD COLUMN session_checkpoint_sha256 TEXT NOT NULL
    DEFAULT '0000000000000000000000000000000000000000000000000000000000000000'
    CHECK(length(session_checkpoint_sha256) = 64);

ALTER TABLE checkpoints
ADD COLUMN session_checkpoint_sha256 TEXT NOT NULL
    DEFAULT '0000000000000000000000000000000000000000000000000000000000000000'
    CHECK(length(session_checkpoint_sha256) = 64);

CREATE TABLE session_checkpoints (
    generation INTEGER PRIMARY KEY CHECK(generation > 0),
    checkpoint_id TEXT NOT NULL UNIQUE,
    checkpoint_sha256 TEXT NOT NULL UNIQUE CHECK(length(checkpoint_sha256) = 64),
    record_json BLOB NOT NULL,
    FOREIGN KEY(generation) REFERENCES checkpoints(generation)
) STRICT;

CREATE TABLE write_checkpoints (
    transaction_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK(sequence >= 0),
    generation INTEGER NOT NULL CHECK(generation > 0),
    checkpoint_id TEXT NOT NULL UNIQUE,
    action_id TEXT NOT NULL,
    phase TEXT NOT NULL,
    checkpoint_sha256 TEXT NOT NULL UNIQUE CHECK(length(checkpoint_sha256) = 64),
    previous_checkpoint_sha256 TEXT NOT NULL CHECK(length(previous_checkpoint_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    PRIMARY KEY(transaction_id, sequence),
    FOREIGN KEY(generation) REFERENCES checkpoints(generation)
) STRICT;

CREATE TABLE write_checkpoint_heads (
    transaction_id TEXT PRIMARY KEY,
    sequence INTEGER NOT NULL CHECK(sequence >= 0),
    checkpoint_sha256 TEXT NOT NULL UNIQUE CHECK(length(checkpoint_sha256) = 64),
    FOREIGN KEY(transaction_id, sequence)
        REFERENCES write_checkpoints(transaction_id, sequence)
) STRICT;

CREATE INDEX write_checkpoints_generation_idx
    ON write_checkpoints(generation, transaction_id, sequence);

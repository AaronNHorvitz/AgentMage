CREATE TABLE repository_map_cache (
    scope_sha256 TEXT NOT NULL CHECK(length(scope_sha256) = 64),
    key_sha256 TEXT NOT NULL CHECK(length(key_sha256) = 64),
    key_json BLOB NOT NULL CHECK(length(key_json) > 0 AND length(key_json) <= 65536),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    payload_sha256 TEXT NOT NULL CHECK(length(payload_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    created_at_epoch_ms INTEGER NOT NULL CHECK(created_at_epoch_ms > 0),
    expires_at_epoch_ms INTEGER NOT NULL CHECK(expires_at_epoch_ms >= created_at_epoch_ms),
    PRIMARY KEY(scope_sha256, key_sha256)
) STRICT;

CREATE INDEX repository_map_cache_expiration_idx
    ON repository_map_cache(expires_at_epoch_ms, scope_sha256, key_sha256);

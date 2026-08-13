ALTER TABLE retention ADD COLUMN hold_kind TEXT NOT NULL DEFAULT 'none'
    CHECK(hold_kind IN ('none', 'user', 'legal'));
ALTER TABLE retention ADD COLUMN prior_disposition TEXT
    CHECK(prior_disposition IS NULL OR prior_disposition IN ('session', 'retained'));
ALTER TABLE retention ADD COLUMN revision INTEGER NOT NULL DEFAULT 1
    CHECK(revision > 0);
ALTER TABLE retention ADD COLUMN updated_at_epoch_ms INTEGER NOT NULL DEFAULT 0
    CHECK(updated_at_epoch_ms >= 0);
ALTER TABLE retention ADD COLUMN erased_at_epoch_ms INTEGER
    CHECK(erased_at_epoch_ms IS NULL OR erased_at_epoch_ms >= 0);
CREATE TABLE retention_events (
    retention_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    event_kind TEXT NOT NULL CHECK(event_kind IN (
        'assigned',
        'user_hold_applied',
        'legal_hold_applied',
        'hold_released',
        'expired',
        'deleted'
    )),
    occurred_at_epoch_ms INTEGER NOT NULL CHECK(occurred_at_epoch_ms >= 0),
    previous_event_sha256 TEXT NOT NULL CHECK(length(previous_event_sha256) = 64),
    state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64),
    event_sha256 TEXT NOT NULL UNIQUE CHECK(length(event_sha256) = 64),
    PRIMARY KEY(retention_id, revision),
    FOREIGN KEY(retention_id) REFERENCES retention(retention_id)
) STRICT;
CREATE INDEX retention_hold_expiry_idx
    ON retention(hold_kind, disposition, expires_at_epoch_ms);

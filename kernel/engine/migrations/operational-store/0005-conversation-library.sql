CREATE TABLE conversations (
    conversation_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    sensitivity TEXT NOT NULL CHECK(sensitivity IN ('ephemeral', 'operational', 'durable', 'restricted')),
    created_at_epoch_ms INTEGER NOT NULL CHECK(created_at_epoch_ms >= 0),
    updated_at_epoch_ms INTEGER NOT NULL CHECK(updated_at_epoch_ms >= created_at_epoch_ms),
    local_date TEXT NOT NULL CHECK(length(local_date) = 10),
    local_timezone TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    project_id TEXT,
    model_profile_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('active', 'completed', 'cancelled', 'failed', 'archived')),
    parent_conversation_id TEXT,
    branch_from_turn_id TEXT,
    current_turn_id TEXT,
    retention_kind TEXT NOT NULL CHECK(retention_kind IN ('session', 'until_expiration', 'user_hold')),
    retention_expires_at_epoch_ms INTEGER CHECK(retention_expires_at_epoch_ms IS NULL OR retention_expires_at_epoch_ms >= 0),
    retention_policy_sha256 TEXT NOT NULL CHECK(length(retention_policy_sha256) = 64),
    pinned INTEGER NOT NULL CHECK(pinned IN (0, 1)),
    persistence_enabled INTEGER NOT NULL CHECK(persistence_enabled IN (0, 1)),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    FOREIGN KEY(parent_conversation_id) REFERENCES conversations(conversation_id),
    FOREIGN KEY(branch_from_turn_id) REFERENCES conversation_turns(turn_id),
    FOREIGN KEY(current_turn_id) REFERENCES conversation_turns(turn_id)
) STRICT;

CREATE TABLE conversation_turns (
    turn_id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal > 0),
    role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant', 'tool')),
    sensitivity TEXT NOT NULL CHECK(sensitivity IN ('ephemeral', 'operational', 'durable', 'restricted')),
    created_at_epoch_ms INTEGER NOT NULL CHECK(created_at_epoch_ms >= 0),
    local_date TEXT NOT NULL CHECK(length(local_date) = 10),
    text_sha256 TEXT NOT NULL CHECK(length(text_sha256) = 64),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    UNIQUE(conversation_id, ordinal),
    FOREIGN KEY(conversation_id) REFERENCES conversations(conversation_id)
) STRICT;

CREATE TABLE conversation_tags (
    conversation_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY(conversation_id, tag),
    FOREIGN KEY(conversation_id) REFERENCES conversations(conversation_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE conversation_turn_attachments (
    turn_id TEXT NOT NULL,
    reference_id TEXT NOT NULL,
    content_sha256 TEXT NOT NULL CHECK(length(content_sha256) = 64),
    PRIMARY KEY(turn_id, reference_id),
    FOREIGN KEY(turn_id) REFERENCES conversation_turns(turn_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE conversation_turn_grants (
    turn_id TEXT NOT NULL,
    grant_id TEXT NOT NULL,
    PRIMARY KEY(turn_id, grant_id),
    FOREIGN KEY(turn_id) REFERENCES conversation_turns(turn_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE conversation_turn_receipts (
    turn_id TEXT NOT NULL,
    receipt_id TEXT NOT NULL,
    PRIMARY KEY(turn_id, receipt_id),
    FOREIGN KEY(turn_id) REFERENCES conversation_turns(turn_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE conversation_turn_checkpoints (
    turn_id TEXT PRIMARY KEY,
    checkpoint_id TEXT NOT NULL UNIQUE,
    FOREIGN KEY(turn_id) REFERENCES conversation_turns(turn_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE conversation_turn_citations (
    turn_id TEXT NOT NULL,
    citation_id TEXT NOT NULL,
    PRIMARY KEY(turn_id, citation_id),
    FOREIGN KEY(turn_id) REFERENCES conversation_turns(turn_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE conversation_turn_sources (
    turn_id TEXT NOT NULL,
    source_sha256 TEXT NOT NULL CHECK(length(source_sha256) = 64),
    PRIMARY KEY(turn_id, source_sha256),
    FOREIGN KEY(turn_id) REFERENCES conversation_turns(turn_id) ON DELETE CASCADE
) STRICT;

CREATE INDEX conversations_date_idx ON conversations(local_date, conversation_id);
CREATE INDEX conversations_workspace_idx ON conversations(workspace_id, conversation_id);
CREATE INDEX conversations_project_idx ON conversations(project_id, conversation_id);
CREATE INDEX conversations_model_idx ON conversations(model_profile_id, conversation_id);
CREATE INDEX conversations_status_idx ON conversations(status, conversation_id);
CREATE INDEX conversations_parent_idx ON conversations(parent_conversation_id, conversation_id);
CREATE INDEX conversation_turns_timeline_idx ON conversation_turns(conversation_id, ordinal);

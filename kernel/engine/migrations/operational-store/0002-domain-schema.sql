CREATE TABLE sessions (
    session_id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('active', 'completed', 'cancelled', 'failed')),
    created_at_epoch_ms INTEGER NOT NULL CHECK(created_at_epoch_ms >= 0),
    updated_at_epoch_ms INTEGER NOT NULL CHECK(updated_at_epoch_ms >= created_at_epoch_ms),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL
) STRICT;
CREATE TABLE objectives (
    objective_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal > 0),
    status TEXT NOT NULL CHECK(status IN ('proposed', 'active', 'completed', 'cancelled', 'blocked', 'failed')),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    UNIQUE(session_id, ordinal),
    FOREIGN KEY(session_id) REFERENCES sessions(session_id)
) STRICT;
CREATE TABLE plans (
    plan_id TEXT PRIMARY KEY,
    objective_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    status TEXT NOT NULL CHECK(status IN ('draft', 'active', 'superseded', 'completed', 'cancelled')),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    UNIQUE(objective_id, revision),
    FOREIGN KEY(objective_id) REFERENCES objectives(objective_id)
) STRICT;
CREATE TABLE tasks (
    task_id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL,
    parent_task_id TEXT,
    ordinal INTEGER NOT NULL CHECK(ordinal > 0),
    status TEXT NOT NULL CHECK(status IN ('pending', 'active', 'completed', 'blocked', 'cancelled', 'failed')),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    UNIQUE(plan_id, ordinal),
    UNIQUE(task_id, plan_id),
    FOREIGN KEY(plan_id) REFERENCES plans(plan_id),
    FOREIGN KEY(parent_task_id, plan_id) REFERENCES tasks(task_id, plan_id)
) STRICT;
CREATE TABLE actions (
    action_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    transaction_id TEXT,
    ordinal INTEGER NOT NULL CHECK(ordinal > 0),
    operation_class TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('proposed', 'approved', 'denied', 'running', 'succeeded', 'failed', 'cancelled', 'uncertain')),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    UNIQUE(task_id, ordinal),
    UNIQUE(action_id, task_id),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id),
    FOREIGN KEY(transaction_id) REFERENCES transaction_identities(transaction_id)
) STRICT;
CREATE TABLE evidence (
    evidence_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    action_id TEXT,
    ordinal INTEGER NOT NULL CHECK(ordinal > 0),
    evidence_kind TEXT NOT NULL,
    classification TEXT NOT NULL CHECK(classification IN ('observed', 'derived', 'inferred', 'unknown_blocked')),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    UNIQUE(task_id, ordinal),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id),
    FOREIGN KEY(action_id, task_id) REFERENCES actions(action_id, task_id)
) STRICT;
CREATE TABLE decisions (
    decision_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal > 0),
    disposition TEXT NOT NULL CHECK(disposition IN ('proposed', 'accepted', 'rejected', 'superseded')),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    UNIQUE(task_id, ordinal),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id)
) STRICT;
CREATE TABLE files (
    file_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    object_identity_sha256 TEXT NOT NULL CHECK(length(object_identity_sha256) = 64),
    relative_path_sha256 TEXT NOT NULL CHECK(length(relative_path_sha256) = 64),
    content_sha256 TEXT NOT NULL CHECK(length(content_sha256) = 64),
    status TEXT NOT NULL CHECK(status IN ('observed', 'stale', 'removed')),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL,
    UNIQUE(session_id, object_identity_sha256),
    FOREIGN KEY(session_id) REFERENCES sessions(session_id)
) STRICT;
CREATE TABLE retention (
    retention_id TEXT PRIMARY KEY,
    record_family TEXT NOT NULL CHECK(record_family IN ('sessions', 'objectives', 'plans', 'tasks', 'actions', 'evidence', 'decisions', 'grants', 'receipts', 'checkpoints', 'files')),
    record_id TEXT NOT NULL,
    sensitivity TEXT NOT NULL CHECK(sensitivity IN ('public', 'internal', 'private', 'restricted')),
    disposition TEXT NOT NULL CHECK(disposition IN ('ephemeral', 'session', 'retained', 'held', 'expired', 'deleted')),
    expires_at_epoch_ms INTEGER CHECK(expires_at_epoch_ms IS NULL OR expires_at_epoch_ms >= 0),
    legal_hold INTEGER NOT NULL CHECK(legal_hold IN (0, 1)),
    policy_sha256 TEXT NOT NULL CHECK(length(policy_sha256) = 64),
    UNIQUE(record_family, record_id)
) STRICT;
CREATE INDEX objectives_session_idx ON objectives(session_id);
CREATE INDEX plans_objective_idx ON plans(objective_id);
CREATE INDEX tasks_plan_idx ON tasks(plan_id);
CREATE INDEX actions_task_idx ON actions(task_id);
CREATE INDEX actions_transaction_idx ON actions(transaction_id);
CREATE INDEX evidence_task_idx ON evidence(task_id);
CREATE INDEX decisions_task_idx ON decisions(task_id);
CREATE INDEX files_session_idx ON files(session_id);
CREATE INDEX retention_disposition_idx ON retention(disposition, expires_at_epoch_ms);

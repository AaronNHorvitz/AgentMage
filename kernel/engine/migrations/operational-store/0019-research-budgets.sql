CREATE TABLE research_budget_roots (
    task_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    origin_run_id TEXT NOT NULL,
    plan_artifact_id TEXT NOT NULL,
    plan_manifest_sha256 TEXT NOT NULL CHECK(length(plan_manifest_sha256) = 64),
    root_sha256 TEXT NOT NULL UNIQUE CHECK(length(root_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 24576),
    FOREIGN KEY(origin_run_id) REFERENCES runtime_runs(run_id),
    FOREIGN KEY(plan_artifact_id, plan_manifest_sha256)
        REFERENCES runtime_artifacts(artifact_id, manifest_sha256)
) STRICT;

CREATE TABLE research_budget_revisions (
    task_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision >= 0 AND revision < 128),
    previous_sha256 TEXT NOT NULL CHECK(length(previous_sha256) = 64),
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 16384),
    PRIMARY KEY(task_id, revision),
    UNIQUE(task_id, revision, record_sha256),
    FOREIGN KEY(task_id) REFERENCES research_budget_roots(task_id)
) STRICT;

CREATE TABLE research_budget_heads (
    task_id TEXT PRIMARY KEY,
    revision INTEGER NOT NULL CHECK(revision >= 0 AND revision < 128),
    record_sha256 TEXT NOT NULL CHECK(length(record_sha256) = 64),
    FOREIGN KEY(task_id, revision, record_sha256)
        REFERENCES research_budget_revisions(task_id, revision, record_sha256)
) STRICT;

CREATE TRIGGER research_budget_root_update_forbidden
BEFORE UPDATE ON research_budget_roots
BEGIN
    SELECT RAISE(ABORT, 'research.budget.root.immutable');
END;

CREATE TRIGGER research_budget_root_delete_forbidden
BEFORE DELETE ON research_budget_roots
BEGIN
    SELECT RAISE(ABORT, 'research.budget.root.immutable');
END;

CREATE TRIGGER research_budget_revision_update_forbidden
BEFORE UPDATE ON research_budget_revisions
BEGIN
    SELECT RAISE(ABORT, 'research.budget.revision.append_only');
END;

CREATE TRIGGER research_budget_revision_delete_forbidden
BEFORE DELETE ON research_budget_revisions
BEGIN
    SELECT RAISE(ABORT, 'research.budget.revision.append_only');
END;

CREATE TRIGGER research_budget_head_delete_forbidden
BEFORE DELETE ON research_budget_heads
BEGIN
    SELECT RAISE(ABORT, 'research.budget.head.required');
END;

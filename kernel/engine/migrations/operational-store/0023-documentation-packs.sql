-- Durable documentation pack catalog (CAP-22, Decision 0130). The catalog's
-- owner commits each change of kept versions, their files and the deletion
-- record in one transaction together with the head that names the revision
-- it read. A kept version may only become superseded, a file never changes,
-- a version and its files go only with a later deletion row naming them,
-- deletions are append-only in sequence, and the head only moves forward.
CREATE TABLE doc_pack_catalog_heads (
    catalog_id TEXT NOT NULL PRIMARY KEY CHECK(catalog_id = 'documentation'),
    revision INTEGER NOT NULL CHECK(revision >= 1),
    last_changed_on TEXT CHECK(
        last_changed_on IS NULL
        OR (length(last_changed_on) = 10
            AND last_changed_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]')
    ),
    state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64)
) STRICT;

CREATE TABLE doc_pack_versions (
    pack_id TEXT NOT NULL CHECK(length(pack_id) > 0 AND length(pack_id) <= 64),
    major INTEGER NOT NULL CHECK(major >= 0 AND major <= 4294967295),
    minor INTEGER NOT NULL CHECK(minor >= 0 AND minor <= 4294967295),
    patch INTEGER NOT NULL CHECK(patch >= 0 AND patch <= 4294967295),
    manifest_sha256 TEXT NOT NULL CHECK(length(manifest_sha256) = 64),
    manifest_json BLOB NOT NULL CHECK(
        length(manifest_json) > 0 AND length(manifest_json) <= 4194304
    ),
    superseded_on TEXT CHECK(
        superseded_on IS NULL
        OR (length(superseded_on) = 10
            AND superseded_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]')
    ),
    deletions_before INTEGER NOT NULL CHECK(deletions_before >= 0),
    PRIMARY KEY(pack_id, major, minor, patch)
) STRICT;

CREATE TABLE doc_pack_files (
    pack_id TEXT NOT NULL,
    major INTEGER NOT NULL,
    minor INTEGER NOT NULL,
    patch INTEGER NOT NULL,
    path TEXT NOT NULL CHECK(length(path) > 0 AND length(path) <= 4096),
    sha256 TEXT NOT NULL CHECK(length(sha256) = 64),
    content BLOB NOT NULL CHECK(length(content) > 0 AND length(content) <= 8388608),
    PRIMARY KEY(pack_id, major, minor, patch, path),
    FOREIGN KEY(pack_id, major, minor, patch)
        REFERENCES doc_pack_versions(pack_id, major, minor, patch)
) STRICT;

CREATE TABLE doc_pack_deletions (
    sequence INTEGER NOT NULL PRIMARY KEY CHECK(sequence >= 1),
    pack_id TEXT NOT NULL CHECK(length(pack_id) > 0 AND length(pack_id) <= 64),
    major INTEGER NOT NULL CHECK(major >= 0 AND major <= 4294967295),
    minor INTEGER NOT NULL CHECK(minor >= 0 AND minor <= 4294967295),
    patch INTEGER NOT NULL CHECK(patch >= 0 AND patch <= 4294967295),
    manifest_sha256 TEXT NOT NULL CHECK(length(manifest_sha256) = 64),
    deleted_on TEXT NOT NULL CHECK(
        length(deleted_on) = 10
        AND deleted_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
    ),
    reason TEXT NOT NULL CHECK(reason IN ('person', 'retention'))
) STRICT;

CREATE TRIGGER doc_pack_head_insert_first_revision
BEFORE INSERT ON doc_pack_catalog_heads
WHEN NEW.revision != 1
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.head.forward_only');
END;

CREATE TRIGGER doc_pack_head_forward_only
BEFORE UPDATE ON doc_pack_catalog_heads
WHEN NEW.catalog_id != OLD.catalog_id
    OR NEW.revision != OLD.revision + 1
    OR (OLD.last_changed_on IS NOT NULL
        AND (NEW.last_changed_on IS NULL OR NEW.last_changed_on < OLD.last_changed_on))
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.head.forward_only');
END;

CREATE TRIGGER doc_pack_head_delete_forbidden
BEFORE DELETE ON doc_pack_catalog_heads
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.head.required');
END;

CREATE TRIGGER doc_pack_version_insert_after_deletions
BEFORE INSERT ON doc_pack_versions
WHEN NEW.deletions_before != (SELECT COUNT(*) FROM doc_pack_deletions)
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.version.deletions_before');
END;

CREATE TRIGGER doc_pack_version_supersede_only
BEFORE UPDATE ON doc_pack_versions
WHEN NEW.pack_id != OLD.pack_id
    OR NEW.major != OLD.major
    OR NEW.minor != OLD.minor
    OR NEW.patch != OLD.patch
    OR NEW.manifest_sha256 != OLD.manifest_sha256
    OR NEW.manifest_json != OLD.manifest_json
    OR NEW.deletions_before != OLD.deletions_before
    OR OLD.superseded_on IS NOT NULL
    OR NEW.superseded_on IS NULL
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.version.supersede_only');
END;

CREATE TRIGGER doc_pack_version_delete_requires_record
BEFORE DELETE ON doc_pack_versions
WHEN NOT EXISTS (
        SELECT 1 FROM doc_pack_deletions
        WHERE pack_id = OLD.pack_id
          AND major = OLD.major
          AND minor = OLD.minor
          AND patch = OLD.patch
          AND manifest_sha256 = OLD.manifest_sha256
          AND sequence > OLD.deletions_before
    )
    OR EXISTS (
        SELECT 1 FROM doc_pack_files
        WHERE pack_id = OLD.pack_id
          AND major = OLD.major
          AND minor = OLD.minor
          AND patch = OLD.patch
    )
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.version.deletion_record');
END;

CREATE TRIGGER doc_pack_file_update_forbidden
BEFORE UPDATE ON doc_pack_files
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.file.immutable');
END;

CREATE TRIGGER doc_pack_file_delete_requires_record
BEFORE DELETE ON doc_pack_files
WHEN NOT EXISTS (
    SELECT 1 FROM doc_pack_versions AS version
    JOIN doc_pack_deletions AS deletion
      ON deletion.pack_id = version.pack_id
     AND deletion.major = version.major
     AND deletion.minor = version.minor
     AND deletion.patch = version.patch
     AND deletion.manifest_sha256 = version.manifest_sha256
     AND deletion.sequence > version.deletions_before
    WHERE version.pack_id = OLD.pack_id
      AND version.major = OLD.major
      AND version.minor = OLD.minor
      AND version.patch = OLD.patch
)
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.file.deletion_record');
END;

CREATE TRIGGER doc_pack_deletion_insert_in_sequence
BEFORE INSERT ON doc_pack_deletions
WHEN NEW.sequence != (SELECT COUNT(*) FROM doc_pack_deletions) + 1
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.deletion.sequence');
END;

CREATE TRIGGER doc_pack_deletion_update_forbidden
BEFORE UPDATE ON doc_pack_deletions
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.deletion.append_only');
END;

CREATE TRIGGER doc_pack_deletion_delete_forbidden
BEFORE DELETE ON doc_pack_deletions
BEGIN
    SELECT RAISE(ABORT, 'doc.pack.deletion.append_only');
END;

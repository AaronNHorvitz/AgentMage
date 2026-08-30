CREATE UNIQUE INDEX source_manifests_physical_identity_idx
    ON source_manifests(physical_artifact_id)
    WHERE physical_artifact_id IS NOT NULL;

CREATE UNIQUE INDEX source_retentions_physical_identity_idx
    ON source_retentions(physical_artifact_id)
    WHERE physical_artifact_id IS NOT NULL;

CREATE TRIGGER source_origins_identity_update
BEFORE UPDATE ON source_origins
BEGIN
    SELECT RAISE(ABORT, 'source origins are immutable');
END;

CREATE TRIGGER source_references_identity_update
BEFORE UPDATE ON source_references
BEGIN
    SELECT RAISE(ABORT, 'source references are immutable');
END;

CREATE TRIGGER source_manifests_identity_insert
BEFORE INSERT ON source_manifests
WHEN NOT EXISTS (
        SELECT 1
        FROM source_origins
        WHERE origin_id = NEW.origin_id
          AND request_id = NEW.request_id
          AND authority_id = NEW.authority_id
    )
    OR NOT EXISTS (
        SELECT 1
        FROM source_references
        WHERE reference_id = NEW.reference_id
          AND request_id = NEW.request_id
          AND authority_id = NEW.authority_id
    )
    OR EXISTS (
        SELECT 1
        FROM source_provenance
        WHERE provenance_id = NEW.provenance_id
          AND (
              source_artifact_id <> NEW.source_artifact_id
              OR reference_id <> NEW.reference_id
              OR origin_id <> NEW.origin_id
              OR classification <> NEW.classification
              OR freshness_state <> NEW.freshness_state
              OR provenance_sha256 <> NEW.provenance_sha256
          )
    )
BEGIN
    SELECT RAISE(ABORT, 'source manifest identity mismatch');
END;

CREATE TRIGGER source_manifests_identity_update
BEFORE UPDATE ON source_manifests
BEGIN
    SELECT RAISE(ABORT, 'source manifests are immutable');
END;

CREATE TRIGGER source_provenance_identity_insert
BEFORE INSERT ON source_provenance
WHEN EXISTS (
        SELECT 1
        FROM source_manifests
        WHERE source_artifact_id = NEW.source_artifact_id
    )
    AND NOT EXISTS (
        SELECT 1
        FROM source_manifests
        WHERE source_artifact_id = NEW.source_artifact_id
          AND provenance_id = NEW.provenance_id
          AND provenance_sha256 = NEW.provenance_sha256
          AND reference_id = NEW.reference_id
          AND origin_id = NEW.origin_id
          AND classification = NEW.classification
          AND freshness_state = NEW.freshness_state
    )
BEGIN
    SELECT RAISE(ABORT, 'source provenance identity mismatch');
END;

CREATE TRIGGER source_provenance_identity_update
BEFORE UPDATE ON source_provenance
BEGIN
    SELECT RAISE(ABORT, 'source provenance is immutable');
END;

CREATE TRIGGER source_retentions_identity_insert
BEFORE INSERT ON source_retentions
WHEN NOT EXISTS (
        SELECT 1
        FROM source_manifests
        WHERE source_artifact_id = NEW.source_artifact_id
          AND request_id = NEW.request_id
          AND authority_id = NEW.authority_id
    )
    OR (
        NEW.retention_class = 'policy_persisted'
        AND NOT EXISTS (
            SELECT 1
            FROM source_manifests
            WHERE source_artifact_id = NEW.source_artifact_id
              AND physical_artifact_id = NEW.physical_artifact_id
              AND payload_sha256 = NEW.payload_sha256
              AND byte_size = NEW.byte_size
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'source retention identity mismatch');
END;

CREATE TRIGGER source_retentions_identity_update
BEFORE UPDATE ON source_retentions
WHEN NEW.retention_id <> OLD.retention_id
    OR NEW.source_artifact_id <> OLD.source_artifact_id
    OR NEW.request_id <> OLD.request_id
    OR NEW.authority_id <> OLD.authority_id
    OR NEW.owner_class <> OLD.owner_class
    OR NEW.owner_id <> OLD.owner_id
    OR (
        NEW.retention_class = 'policy_persisted'
        AND (
            OLD.retention_class <> 'policy_persisted'
            OR NEW.retention_policy_id IS NOT OLD.retention_policy_id
            OR NEW.retention_expires_at IS NOT OLD.retention_expires_at
            OR NEW.physical_artifact_id IS NOT OLD.physical_artifact_id
            OR NEW.payload_sha256 IS NOT OLD.payload_sha256
            OR NEW.byte_size IS NOT OLD.byte_size
        )
    )
    OR NOT EXISTS (
        SELECT 1
        FROM source_manifests
        WHERE source_artifact_id = NEW.source_artifact_id
          AND request_id = NEW.request_id
          AND authority_id = NEW.authority_id
    )
    OR (
        NEW.retention_class = 'policy_persisted'
        AND NOT EXISTS (
            SELECT 1
            FROM source_manifests
            WHERE source_artifact_id = NEW.source_artifact_id
              AND physical_artifact_id = NEW.physical_artifact_id
              AND payload_sha256 = NEW.payload_sha256
              AND byte_size = NEW.byte_size
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'source retention identity mismatch');
END;

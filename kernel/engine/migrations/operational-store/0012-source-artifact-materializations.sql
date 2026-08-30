CREATE UNIQUE INDEX runtime_artifacts_source_binding_idx
    ON runtime_artifacts(artifact_id, payload_sha256, byte_size);

CREATE TABLE source_origins (
    origin_id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    authority_id TEXT NOT NULL,
    origin_class TEXT NOT NULL CHECK(
        origin_class IN (
            'paste', 'request_reference', 'file', 'uri', 'directory',
            'archive', 'tool_output', 'unsupported'
        )
    ),
    captured_at TEXT NOT NULL,
    origin_sha256 TEXT NOT NULL UNIQUE CHECK(length(origin_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304)
) STRICT;

CREATE TABLE source_references (
    reference_id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    authority_id TEXT NOT NULL,
    reference_class TEXT NOT NULL CHECK(
        reference_class IN (
            'paste', 'request_reference', 'file_path', 'virtual_uri', 'remote_uri',
            'directory', 'archive', 'unsupported'
        )
    ),
    support_state TEXT NOT NULL CHECK(
        support_state IN ('supported', 'unsupported', 'ambient_prohibited')
    ),
    reference_sha256 TEXT NOT NULL UNIQUE CHECK(length(reference_sha256) = 64),
    collected_at TEXT NOT NULL,
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    CHECK(
        (reference_class = 'unsupported' AND support_state = 'unsupported')
        OR
        (reference_class <> 'unsupported' AND support_state IN ('supported', 'ambient_prohibited'))
    )
) STRICT;

CREATE TABLE source_manifests (
    source_artifact_id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    authority_id TEXT NOT NULL,
    reference_id TEXT NOT NULL,
    origin_id TEXT NOT NULL,
    provenance_id TEXT NOT NULL,
    provenance_sha256 TEXT NOT NULL CHECK(length(provenance_sha256) = 64),
    declared_media_type TEXT NOT NULL,
    classification TEXT NOT NULL CHECK(
        classification IN ('public', 'internal', 'confidential', 'restricted')
    ),
    freshness_state TEXT NOT NULL CHECK(
        freshness_state IN (
            'fresh', 'stale', 'renamed', 'replaced', 'missing', 'unavailable', 'unsupported'
        )
    ),
    capture_state TEXT NOT NULL CHECK(
        capture_state IN ('captured', 'unavailable', 'unsupported', 'denied', 'failed')
    ),
    physical_artifact_id TEXT,
    payload_sha256 TEXT CHECK(payload_sha256 IS NULL OR length(payload_sha256) = 64),
    byte_size INTEGER CHECK(byte_size IS NULL OR (byte_size >= 0 AND byte_size <= 104857600)),
    collected_at TEXT NOT NULL,
    source_artifact_sha256 TEXT NOT NULL UNIQUE CHECK(length(source_artifact_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    UNIQUE(provenance_id, provenance_sha256),
    FOREIGN KEY(reference_id) REFERENCES source_references(reference_id),
    FOREIGN KEY(origin_id) REFERENCES source_origins(origin_id),
    FOREIGN KEY(physical_artifact_id, payload_sha256, byte_size)
        REFERENCES runtime_artifacts(artifact_id, payload_sha256, byte_size),
    FOREIGN KEY(provenance_id, provenance_sha256)
        REFERENCES source_provenance(provenance_id, provenance_sha256)
        DEFERRABLE INITIALLY DEFERRED,
    CHECK(
        (capture_state = 'captured'
            AND freshness_state IN ('fresh', 'renamed')
            AND physical_artifact_id IS NOT NULL
            AND payload_sha256 IS NOT NULL
            AND byte_size IS NOT NULL)
        OR
        (capture_state <> 'captured'
            AND physical_artifact_id IS NULL
            AND payload_sha256 IS NULL
            AND byte_size IS NULL)
    )
) STRICT;

CREATE TABLE source_provenance (
    provenance_id TEXT PRIMARY KEY,
    source_artifact_id TEXT NOT NULL UNIQUE,
    reference_id TEXT NOT NULL,
    origin_id TEXT NOT NULL,
    classification TEXT NOT NULL CHECK(
        classification IN ('public', 'internal', 'confidential', 'restricted')
    ),
    freshness_state TEXT NOT NULL CHECK(
        freshness_state IN (
            'fresh', 'stale', 'renamed', 'replaced', 'missing', 'unavailable', 'unsupported'
        )
    ),
    observed_at TEXT NOT NULL,
    collected_at TEXT NOT NULL,
    provenance_sha256 TEXT NOT NULL UNIQUE CHECK(length(provenance_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    UNIQUE(provenance_id, provenance_sha256),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(reference_id) REFERENCES source_references(reference_id),
    FOREIGN KEY(origin_id) REFERENCES source_origins(origin_id)
) STRICT;

CREATE TABLE source_extractions (
    extraction_id TEXT PRIMARY KEY,
    source_artifact_id TEXT NOT NULL,
    extractor_id TEXT NOT NULL,
    extractor_version TEXT NOT NULL,
    source_sha256 TEXT NOT NULL CHECK(length(source_sha256) = 64),
    output_sha256 TEXT CHECK(output_sha256 IS NULL OR length(output_sha256) = 64),
    media_type TEXT NOT NULL,
    disposition TEXT NOT NULL CHECK(
        disposition IN (
            'captured', 'parsed', 'partially_parsed', 'unsupported',
            'denied', 'unavailable', 'failed', 'omitted'
        )
    ),
    truncated INTEGER NOT NULL CHECK(truncated IN (0, 1)),
    reproducible INTEGER NOT NULL CHECK(reproducible IN (0, 1)),
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    CHECK(
        (disposition IN ('captured', 'parsed', 'partially_parsed') AND output_sha256 IS NOT NULL)
        OR
        (disposition IN ('unsupported', 'denied', 'unavailable', 'failed', 'omitted')
            AND output_sha256 IS NULL)
    )
) STRICT;

CREATE TABLE source_sections (
    section_id TEXT PRIMARY KEY,
    source_artifact_id TEXT NOT NULL,
    extraction_id TEXT NOT NULL,
    parent_section_id TEXT,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    section_kind TEXT NOT NULL CHECK(
        section_kind IN (
            'document_root', 'heading', 'paragraph', 'list_item', 'table', 'code_block',
            'image_region', 'page', 'sheet', 'cell', 'log_cluster', 'unknown'
        )
    ),
    start_byte INTEGER NOT NULL CHECK(start_byte >= 0),
    end_byte_exclusive INTEGER NOT NULL CHECK(end_byte_exclusive >= start_byte),
    start_line INTEGER CHECK(start_line IS NULL OR start_line >= 0),
    end_line_exclusive INTEGER CHECK(
        end_line_exclusive IS NULL OR (start_line IS NOT NULL AND end_line_exclusive >= start_line)
    ),
    token_count INTEGER NOT NULL CHECK(token_count >= 0),
    content_sha256 TEXT NOT NULL CHECK(length(content_sha256) = 64),
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    UNIQUE(extraction_id, ordinal),
    UNIQUE(section_id, source_artifact_id),
    UNIQUE(section_id, extraction_id),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    FOREIGN KEY(extraction_id) REFERENCES source_extractions(extraction_id),
    FOREIGN KEY(parent_section_id, extraction_id)
        REFERENCES source_sections(section_id, extraction_id),
    CHECK((start_line IS NULL) = (end_line_exclusive IS NULL))
) STRICT;

CREATE TABLE source_cache_inputs (
    cache_input_id TEXT PRIMARY KEY,
    source_artifact_id TEXT NOT NULL,
    extraction_id TEXT,
    source_sha256 TEXT NOT NULL CHECK(length(source_sha256) = 64),
    parser_identity TEXT NOT NULL,
    parser_version TEXT NOT NULL,
    parser_configuration_sha256 TEXT NOT NULL CHECK(length(parser_configuration_sha256) = 64),
    schema_sha256 TEXT NOT NULL CHECK(length(schema_sha256) = 64),
    policy_sha256 TEXT NOT NULL CHECK(length(policy_sha256) = 64),
    cache_key_sha256 TEXT NOT NULL UNIQUE CHECK(length(cache_key_sha256) = 64),
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    FOREIGN KEY(extraction_id) REFERENCES source_extractions(extraction_id)
) STRICT;

CREATE TABLE source_lexical_indexes (
    lexical_index_id TEXT PRIMARY KEY,
    source_artifact_id TEXT NOT NULL,
    extraction_id TEXT NOT NULL,
    cache_input_id TEXT NOT NULL,
    indexer_id TEXT NOT NULL,
    indexer_version TEXT NOT NULL,
    index_sha256 TEXT NOT NULL UNIQUE CHECK(length(index_sha256) = 64),
    entry_count INTEGER NOT NULL CHECK(entry_count >= 0),
    token_count INTEGER NOT NULL CHECK(token_count >= 0),
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    FOREIGN KEY(extraction_id) REFERENCES source_extractions(extraction_id),
    FOREIGN KEY(cache_input_id) REFERENCES source_cache_inputs(cache_input_id)
) STRICT;

CREATE TABLE source_context_dispositions (
    disposition_id TEXT PRIMARY KEY,
    context_manifest_id TEXT NOT NULL,
    source_artifact_id TEXT NOT NULL,
    section_id TEXT,
    disposition TEXT NOT NULL CHECK(
        disposition IN (
            'included', 'summarized', 'truncated', 'duplicate', 'stale',
            'unsupported', 'unavailable', 'restricted', 'omitted'
        )
    ),
    token_count INTEGER NOT NULL CHECK(token_count >= 0),
    ranges_sha256 TEXT NOT NULL CHECK(length(ranges_sha256) = 64),
    terminal INTEGER NOT NULL CHECK(terminal = 1),
    record_sha256 TEXT NOT NULL UNIQUE CHECK(length(record_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    UNIQUE(context_manifest_id, source_artifact_id, section_id),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    FOREIGN KEY(section_id, source_artifact_id)
        REFERENCES source_sections(section_id, source_artifact_id)
) STRICT;

CREATE TABLE source_retentions (
    retention_id TEXT PRIMARY KEY,
    source_artifact_id TEXT NOT NULL UNIQUE,
    request_id TEXT NOT NULL,
    authority_id TEXT NOT NULL,
    owner_class TEXT NOT NULL CHECK(owner_class IN ('session', 'task', 'request')),
    owner_id TEXT NOT NULL,
    retention_class TEXT NOT NULL CHECK(
        retention_class IN ('policy_persisted', 'memory_only', 'released', 'deleted')
    ),
    retention_policy_id TEXT,
    retention_expires_at TEXT,
    physical_artifact_id TEXT,
    payload_sha256 TEXT CHECK(payload_sha256 IS NULL OR length(payload_sha256) = 64),
    byte_size INTEGER CHECK(byte_size IS NULL OR (byte_size >= 0 AND byte_size <= 104857600)),
    encryption_state TEXT NOT NULL CHECK(
        encryption_state IN ('not_persisted', 'encrypted_at_rest')
    ),
    protected_metadata_sha256 TEXT NOT NULL CHECK(length(protected_metadata_sha256) = 64),
    lifecycle_state TEXT NOT NULL CHECK(
        lifecycle_state IN ('active', 'quarantined', 'released', 'deleted')
    ),
    reason_code TEXT,
    revision INTEGER NOT NULL CHECK(revision > 0),
    recorded_at TEXT NOT NULL,
    source_retention_sha256 TEXT NOT NULL UNIQUE CHECK(length(source_retention_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    UNIQUE(retention_id, revision),
    FOREIGN KEY(source_artifact_id) REFERENCES source_manifests(source_artifact_id),
    FOREIGN KEY(physical_artifact_id, payload_sha256, byte_size)
        REFERENCES runtime_artifacts(artifact_id, payload_sha256, byte_size),
    CHECK(
        (retention_class = 'policy_persisted'
            AND retention_policy_id IS NOT NULL
            AND retention_expires_at IS NOT NULL
            AND physical_artifact_id IS NOT NULL
            AND payload_sha256 IS NOT NULL
            AND byte_size IS NOT NULL
            AND encryption_state = 'encrypted_at_rest')
        OR
        (retention_class <> 'policy_persisted'
            AND retention_policy_id IS NULL
            AND retention_expires_at IS NULL
            AND physical_artifact_id IS NULL
            AND payload_sha256 IS NULL
            AND byte_size IS NULL
            AND encryption_state = 'not_persisted')
    ),
    CHECK(
        (lifecycle_state = 'active' AND reason_code IS NULL)
        OR
        (lifecycle_state <> 'active' AND reason_code IS NOT NULL)
    ),
    CHECK(
        (retention_class IN ('memory_only', 'policy_persisted')
            AND lifecycle_state IN ('active', 'quarantined'))
        OR (retention_class = 'released' AND lifecycle_state = 'released')
        OR (retention_class = 'deleted' AND lifecycle_state = 'deleted')
    )
) STRICT;

CREATE TABLE source_lifecycle_events (
    retention_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    lifecycle_state TEXT NOT NULL CHECK(
        lifecycle_state IN ('active', 'quarantined', 'released', 'deleted')
    ),
    reason_code TEXT,
    occurred_at TEXT NOT NULL,
    previous_event_sha256 TEXT NOT NULL CHECK(length(previous_event_sha256) = 64),
    source_retention_sha256 TEXT NOT NULL CHECK(length(source_retention_sha256) = 64),
    event_sha256 TEXT NOT NULL UNIQUE CHECK(length(event_sha256) = 64),
    record_json BLOB NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 4194304),
    PRIMARY KEY(retention_id, revision),
    FOREIGN KEY(retention_id) REFERENCES source_retentions(retention_id) ON DELETE CASCADE,
    CHECK(
        (lifecycle_state = 'active' AND reason_code IS NULL)
        OR
        (lifecycle_state <> 'active' AND reason_code IS NOT NULL)
    )
) STRICT;

CREATE INDEX source_manifests_payload_idx
    ON source_manifests(payload_sha256, source_artifact_id);
CREATE INDEX source_manifests_request_idx
    ON source_manifests(request_id, source_artifact_id);
CREATE INDEX source_extractions_source_idx
    ON source_extractions(source_artifact_id, extraction_id);
CREATE INDEX source_sections_source_idx
    ON source_sections(source_artifact_id, extraction_id, ordinal);
CREATE INDEX source_context_dispositions_manifest_idx
    ON source_context_dispositions(context_manifest_id, disposition_id);
CREATE INDEX source_retentions_owner_idx
    ON source_retentions(owner_class, owner_id, source_artifact_id);

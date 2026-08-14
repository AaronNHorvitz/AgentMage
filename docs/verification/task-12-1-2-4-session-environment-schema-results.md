# Task 12.1.2.4 Session Environment Schema Results

## Result

Pass for the strict session environment capture schema and canonical fixture.

Focused cases closed: **5 of 5**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `ESC-01` | Record closure | Require every capture field, reject unknown fields, and retain schema, record, capture, and descriptive-authority identities | Pass |
| `ESC-02` | Workspace paths | Admit canonical workspace-scoped paths, require root-only authorized roots, and reject traversal and malformed components | Pass |
| `ESC-03` | Repository provenance | Close branch and detached-head variants and bind workspace, root, repository digest, and 40- or 64-character Git object | Pass |
| `ESC-04` | Metadata and configuration | Bound attachment metadata, platform identities, profile identities, configuration digest, and model-enabled state | Pass |
| `ESC-05` | Validation scope | Run in strict AJV mode with no coercion, defaults, unknown-field removal, private data, live execution, or network use | Pass |

## Evidence

- [`session-environment-capture.schema.json`](../../schemas/runtime/session-environment-capture.schema.json) defines the closed Draft 2020-12 record.
- [`session-environment-capture.valid.json`](../../schemas/runtime/examples/session-environment-capture.valid.json) is the canonical synthetic fixture.
- [`session_environment.rs`](../../kernel/engine/src/session_environment.rs) remains the authoritative implementation for validation, capture identity, cross-field affinity, and descriptive authority.
- [`validate_planning_schemas.mjs`](../../scripts/validate_planning_schemas.mjs) registers the schema in the shared strict runtime-schema validator.
- [`test_planning_schemas.mjs`](../../tests/test_planning_schemas.mjs) rejects field, timestamp, timezone, root, traversal, Git, head-variant, attachment, digest, platform, and authority mutations.
- [`session_environment_schema_evidence.py`](../../scripts/session_environment_schema_evidence.py) binds the review sources and command results to an immutable revision.

## Limits

- JSON Schema validates the normalized record shape; the Rust kernel remains responsible for active-workspace, repository-workspace, and duplicate attachment-identity relationships.
- The schema validates digest syntax but does not recompute `capture_sha256` from the capture material.
- Production wire serialization, encrypted persistence, restart reconstruction, and UI consumption remain later integration work.
- The canonical fixture is synthetic and contains no private user data, ambient absolute path, remote URL, file content, or credential.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.

# Story 11.2 Source-Materialization Schema Evidence

This record covers Sub-task 11.2.1.1 only. Migration `0012` adds normalized encrypted-store
materializations for source manifests, origins, references, extractions, sections, provenance,
lexical indexes, context dispositions, cache inputs, retention, and lifecycle events.

## Authority boundary

Migration `0007` remains the physical artifact authority. A persisted source manifest or retention
record binds the existing `runtime_artifacts` identity, payload SHA-256, and byte size as one foreign
key tuple. Migration `0012` creates no source-byte or payload table and source rows retain only
bounded metadata and record JSON inside the already encrypted operational store.

The current Verified Chat capture tables from migration `0011` remain unchanged. This increment does
not promote them into a second canonical source store and does not migrate or duplicate their bytes.

## Structural closure

All eleven tables are `STRICT`, bound by closed enum checks, digest shapes, record-size limits, and
foreign keys. Manifest/provenance records form one deferred transaction-safe cycle. A child section
cannot cross extraction identity, a context disposition cannot name a section from another source,
and persisted retention cannot separate the physical artifact from its payload digest or size.

Focused encrypted SQLCipher tests prove the exact table closure, migration history from schema 1
through the current schema 14, recovery of an interrupted migration campaign, one complete synthetic normalized
family over exactly one pre-existing runtime payload, and rejection of unsupported-reference,
non-producing-extraction, nonterminal-disposition, and mismatched-retention mutations. Strict Clippy
also passes.

## Deliberately open scope

This is a structural schema increment. It does not itself claim typed publication or content-hash
deduplication semantics; the latter is covered by the separate Sub-task 11.2.1.2 evidence. Refresh
and transitive invalidation, expiry/deletion/hold/garbage-collection transactions, complete
migration matrices, backup/export/restore expansion, and exhaustive crash campaigns remain assigned
to Sub-tasks 11.2.1.3 through 11.2.4.3. Story, Sprint, platform, and release completion are not
claimed.

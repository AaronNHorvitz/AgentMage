# Story 11.2 new-family storage lifecycle evidence

Sub-task 11.2.3.3 is locally complete for the canonical SQLCipher operational store. The derived
JSON Lines export now covers every one of the 31 persisted source, source-lifecycle, workflow, and
terminal-diagnostic tables introduced by schema versions 12 through 15. Each projection retains
only a hashed logical identity, a nonnegative revision or ordinal, and the table's canonical record
or event digest. Raw `record_json`, source/content digests, payload identities, idempotency keys,
approval and receipt identities, and state fingerprints are excluded.

The coverage test derives the expected family set directly from the current SQLite schema, rejects
duplicates and omissions, compiles every three-column projection, and rejects secret- or
source-bearing fields. Populated source and workflow fixtures prove their rows actually materialize
as content-free exports. Existing focused proofs exercise whole-store encrypted backup, verified
fresh-candidate restore, whole-store cryptographic erasure, synthetic-canary exclusion from the
encrypted database, backup, export, and diagnostics, and atomic source refresh/hold/expiry/delete
retention behavior. Because backup, restore, and erasure operate on the complete encrypted database
and key scope rather than a record allowlist, their coverage automatically includes all 31 families.

This evidence is local Linux evidence and does not claim the exhaustive crash campaign in Task
11.2.4, Story or Sprint completion, platform coverage, packaging readiness, or release readiness.
Exact commands and artifact hashes are retained in
`artifacts/sprints/sprint-11/story-11.2/storage-new-family-lifecycle-report.json`; combined output is
retained beside it in `storage-new-family-lifecycle-results.log`.

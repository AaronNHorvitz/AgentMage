# Story 11.2 storage migration compatibility evidence

Sub-task 11.2.3.1 is locally complete for the canonical SQLCipher operational store. The retained
schema-21 fixture independently pins the ordered hashes for all twenty-one forward migrations and the
closed table inventory. Fresh-store and version-one upgrade tests compare the live database to that
fixture. Historical fixtures and migration bytes remain unchanged.

Decision 0118 adds migration 21 for the durable job control ledgers: three new tables with
append-only and forward-only triggers. An actual encrypted version-20 fixture upgrades to version
21 with its records and full history preserved and no ledger created. Corrupt history is refused
before the tables are added. A migration that fails part way leaves the store at version 20 with its
history and records unchanged, and it succeeds once the conflict is removed.

Decision 0087 adds an atomic compatibility epoch without changing the tables. An actual encrypted
version-19 fixture preserves its records and full history; corrupt history is refused before
advancement, and a failed version-20 transaction remains retryable at version 19. Restoring a
version-19 backup is refused without changing its bytes or creating a candidate. The retained-draft
reader-ceiling probe is separate component evidence, not execution of an installed older binary.

Focused tests also prove that failed version-two and version-three migrations roll back their schema
and history changes; the seeded subprocess campaign interrupts the migration boundary before and
after commit and then recovers without repeating a completed transition. Future schema versions,
tampered migration history, and corrupted encrypted pages are refused. Backup and restore preserve
pre-existing destination bytes rather than replacing an occupied path. Strict kernel Clippy is part
of the retained command set.

The evidence uses synthetic records and does not claim the downgrade-record behavior in 11.2.3.2,
the complete new-family lifecycle matrix in 11.2.3.3, Story or Sprint completion, platform coverage,
packaging readiness, or release readiness. The exact commands and artifact hashes are retained in
`artifacts/sprints/sprint-11/story-11.2/storage-migration-compatibility-report.json`; their combined
output is retained beside it in `storage-migration-compatibility-results.log`.

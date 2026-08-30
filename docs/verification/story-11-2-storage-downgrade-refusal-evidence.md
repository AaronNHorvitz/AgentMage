# Story 11.2 storage downgrade-refusal evidence

Sub-task 11.2.3.2 is locally complete for the canonical SQLCipher operational store. Store open now
reads and checks `user_version` immediately after keyed connection setup. A schema newer than the
client supports is refused before exclusive writer acquisition, WAL selection, migration, integrity
projection, or record loading.

The focused older-client test creates a current encrypted store with a synthetic newer record, closes
it, and attempts to open it with a client one schema version behind. Refusal leaves the encrypted
database byte-for-byte identical and creates no WAL or shared-memory sidecars. The current client
then reopens the same store and reads the exact preserved digest and record bytes. The broader
Engineering Runtime contract test proves every versioned record family rejects unsupported versions
rather than dropping fields or partially decoding them.

Automatic reverse migration remains unsupported. The evidence does not claim the complete new-family
lifecycle matrix in 11.2.3.3, Story or Sprint completion, platform coverage, packaging readiness, or
release readiness. Exact commands and artifact hashes are retained in
`artifacts/sprints/sprint-11/story-11.2/storage-downgrade-refusal-report.json`; combined output is
retained beside it in `storage-downgrade-refusal-results.log`.

# Bounded Local Structured Database

Sprint 68 exposes only fixed, parameterized SQLite templates over caller-supplied synthetic or
already-admitted AgentMage-owned records. The adapter creates an in-memory projection, migrates it in
one immediate transaction, and enables SQLite query-only mode before reads. There is no raw SQL,
path, URI, attachment, extension, pragma, live connector, credential, network, or arbitrary mutation
entry point.

Schema inspection, row reads, and fixture construction are independent grant bits. Query requests
bind an exact report scope and source identity, a closed template, typed parameters, canonical
redaction columns, and row/column/byte ceilings. Cancellation is checked before statement execution
and during row projection. Every statement is reclassified with SQLite's read-only check.

Results preserve NULL, signed integer, IEEE-754 bit, and UTF-8 identities; blobs are represented by
digest and byte count. Redacted values retain only a digest. Rows are stable by primary identity and
the receipt binds source kind/hash/freshness, adapter, report scope, schema, parameters, rows,
resource totals, truncation limitations, read-only state, and denied file/network effects.

PostgreSQL, live/remote/credentialed sources, caller paths, and external fixture files remain
disabled. This local contract does not promote a database, platform, product, or release.

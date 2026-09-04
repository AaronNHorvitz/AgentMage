# Local Database Review

Confirm that the source is `synthetic_fixture` or `agent_mage_owned`, the report scope and source ID
match the grant, and the query uses one closed template. Schema inspection must not imply row access;
fixture construction must not imply either read permission.

Review the exact source/schema/parameter/row hashes, freshness, typed values, redacted hashes,
row/column/byte ceilings, completeness, limitations, and receipt digest. Reject raw SQL, multiple
statements, write/pragma/attachment/extension requests, unbounded results, unsorted redaction fields,
wrong scopes, paths, URIs, credentials, external sources, PostgreSQL, or any file/network effect.

Local fixture success is not native parity, live database support, external account access, or
release approval.

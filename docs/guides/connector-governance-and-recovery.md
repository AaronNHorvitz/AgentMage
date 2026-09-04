# Connector Governance and Recovery

Email, messaging, calendar, document-repository, archive, database, and cloud connectors each have
their own threat model, capability manifest, data classification, credential scope, rate limit,
publication rule, recovery plan, and acceptance suite. Connector, account, workspace, task,
credential, grant, cache, and context identities are exact partitions and never transfer authority.

Every connector begins with read-only discovery and a local draft. A write requires a separate
current user review and binds one recipient or object, payload, ordered attachments, visibility,
expected effect, disclosure preview, expiry, grant, and idempotency key. Only a digest reference to
a narrow short-lived credential derived by the platform secret store enters the contract.

Remote identity, state, permission, and freshness are re-read before submission. Drift invalidates
the old preview. Receipts retain request and response classifications, external identity,
freshness, changed-state truth, failure, retry count, and rollback or compensation identity.
Verified non-effect requires a new request; partial and unknown outcomes stop for reconciliation
and are never blindly retried.

The current boundary is inert: it owns no secret store, connector client, filesystem, process,
database, or network executor. Native connector writes, provider failure campaigns, remote
snapshots, lifecycle scans, rollback/compensation execution, and independent review remain required.

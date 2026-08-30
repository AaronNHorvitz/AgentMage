# Story 11.2 Workflow-Materialization Schema Evidence

This record covers Sub-task 11.2.2.1 only. Migration `0015` adds normalized SQLCipher metadata
rows for plan-step policy, attempt, preflight, tool call, idempotency key, approval, receipt,
verification, consumed budget, recovery decision, state fingerprint, and terminal diagnostic
families.

## Existing authority bindings

Every row carries one existing runtime run, its exact session, and the exact append-only journal
event that established the metadata. Composite foreign keys prevent substituting another session
for a run or another event identity at the same sequence. Plan-step policy rows bind the existing
plan identity. Attempt rows bind an existing capability-grant identity, and receipt
materializations bind the existing kernel receipt rather than creating a second executor-receipt
authority.

The migration adds no workflow payload table, artifact service, or journal. Bounded canonical
record JSON is encrypted inside the existing operational store, while large or content-bearing
payload authority remains with the migration-`0007` runtime artifact service.

## Focused proof

One encrypted-store test creates a complete synthetic runtime owner and exact correctness event,
then commits one row in all twelve normalized workflow families. It proves that a substituted
event, a run/session mismatch, and an unknown executor receipt fail closed. It also verifies that no
workflow payload table exists. Schema-1 upgrade through schema 16, seeded interrupted-migration
recovery, and warning-denying kernel Clippy pass.

## Deliberately open scope

This leaf establishes the normalized record surfaces and their existing-authority anchors. It does
not yet claim monotonic attempt ordering, idempotency-key uniqueness, one terminal state, exact
receipt/outcome reconciliation, an atomic event/projection/checkpoint transaction, typed
publication APIs, exhaustive workflow crash injection, complete Story 11.2 or Sprint 11
acceptance, platform acceptance, packaging, or release readiness. Those remain assigned to later
numbered tasks.

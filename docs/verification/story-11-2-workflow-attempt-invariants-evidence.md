# Story 11.2 Workflow-Attempt Invariant Evidence

This record covers Sub-task 11.2.2.2 only. Immutable operational-store migration `0016` adds
database enforcement for attempt identity, retry ordering, idempotency identity, terminal outcome,
and executor-receipt linkage on the normalized workflow materializations introduced by migration
`0015`.

## Fail-closed storage contract

Every attempt is inserted in `started` state without a receipt. A retry can use only the next
ordinal for the same step execution, must name the exact preceding attempt, and cannot begin until
that predecessor is terminal. Unique indexes prevent repeated step ordinals, global idempotency-key
digests, workflow receipt identities, and workflow receipts for the same attempt.

A terminal transition is accepted exactly once and only when an immutable workflow-receipt row
binds the same attempt, run, session, executor receipt identity, receipt digest, and outcome. The
workflow receipt must first match an existing kernel receipt by both identity and digest. Attempts
and workflow receipts are append-only after terminalization, so an uncertain outcome cannot later
be rewritten as success.

## Focused proof

The encrypted-store test rejects a duplicate attempt identity, ordinal gap, early successor,
duplicate idempotency digest, receipt-outcome mismatch, receipt substitution, and an attempted
uncertain-to-success rewrite. It then verifies a two-attempt terminal chain with exact receipts.
Schema-1 upgrade through schema 16, the existing seeded interrupted-migration campaign, and
warning-denying kernel Clippy also pass.

## Deliberately open scope

This leaf establishes storage invariants. It does not yet claim a typed workflow publication API,
atomic correctness-event/materialization/checkpoint publication, exhaustive workflow-specific
crash injection, complete Story 11.2 or Sprint 11 acceptance, platform acceptance, packaging, or
release readiness. Those remain assigned to later numbered tasks.

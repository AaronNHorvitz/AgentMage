# Story 11.2 Source-Lifecycle Transaction Evidence

This record covers Sub-task 11.2.1.3 only. Migration `0014` and the kernel
`source_lifecycle` module make source refresh, dependency invalidation, retention holds, expiry,
release, deletion, and physical garbage collection explicit without creating another source-byte
authority.

## Atomic lifecycle boundary

Refresh runs in one immediate SQLCipher transaction. It verifies the replacement has the same
request, authority, and reference identity, computes the complete recursive dependent closure, and
marks the replaced source and every dependent stale with revisioned hash-bound events. Current-only
views exclude stale sections, extractions, cache inputs, lexical indexes, and context dispositions.
Dependency cycles and endpoint mutation are rejected.

User and legal holds are independent, revisioned, hash-chained projections. Expiry releases every
due unheld source in one transaction and reports held rows without weakening them. Explicit release
and deletion require exact authority and revision expectations. They update normalized retention,
source lifecycle, and the existing runtime-artifact reference in one transaction; failures roll the
whole transition back.

## Sole payload authority

Migration `0007` remains the only physical payload authority. Source release retains only the
content address and former runtime-artifact identity required for reconciliation. Deletion first
commits the logical lifecycle transition. Garbage collection then delegates to the existing
runtime-artifact reconciler, which removes an object only after its authoritative active reference
count reaches zero. Equal bytes still occupy one payload object, and collection of a deleted source
cannot remove bytes referenced by another live artifact.

## Focused proof

Two encrypted-store tests exercise refresh across a transitive dependency chain, current-view
filtering, hold-blocked expiry, hold release, expiry, deletion, final collection, stale-revision
rollback, and active-hold rollback without runtime reference drift. A
focused runtime-artifact test independently proves that collection preserves a shared payload until
its last live reference releases, then removes the orphan. The same evidence reruns schema-1 upgrade
through schema 14, seeded interrupted-migration recovery, and warning-denying Clippy. Five evidence
mutation tests reject missing transaction surfaces, a second payload authority, omitted test
results, failures, and completion overclaims.

## Deliberately open scope

This increment does not claim typed source publication, an exhaustive source-specific process-kill
campaign, workflow materializations, complete Story 11.2 or Sprint 11 acceptance, platform
acceptance, packaging, or release readiness. Those remain assigned to later numbered tasks.

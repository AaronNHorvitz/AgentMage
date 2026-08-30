# Story 11.2 Atomic Workflow-Publication Evidence

This record covers Sub-task 11.2.2.3 only. The durable authority runtime now accepts one typed,
bounded workflow-state materialization together with the correctness event that establishes it, a
safe session checkpoint, the cursor for that newly appended event, and exact references to already
verified runtime artifacts.

## One canonical transaction

The publication reuses the operational store's immediate SQLCipher snapshot transaction and
generation compare-and-swap. It appends the correctness event, inserts the workflow-state row,
then verifies and records the checkpoint cursor and its ordered artifact-reference set before the
single commit. The workflow row and resume binding must name the same run, session, event identity,
sequence, and digest; malformed counters, fingerprints, duplicate record identities, non-
correctness events, stale predecessor cursors, and artifact substitution fail closed.

## Focused proof

The success test starts one durable run and co-publishes its next correctness event, one state
fingerprint, one checkpoint, the new event cursor, and one exact existing artifact reference. The
rollback test injects failure at workflow-state insertion and verifies that the event, state row,
checkpoint, resume binding, artifact-reference projection, and generation all remain at their old
state. The pre-existing atomic checkpoint/reopen test and warning-denying kernel Clippy also pass.

## Deliberately open scope

This leaf proves the canonical transaction boundary. It does not claim exhaustive crash injection
at every workflow record family, migration compatibility fixtures beyond the current applicable
tests, complete Story 11.2 or Sprint 11 acceptance, platform acceptance, packaging, or release
readiness. Those remain assigned to later numbered tasks.

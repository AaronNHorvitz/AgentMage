# Story 11.3 Durable Workflow Resume Evidence

## Result

The repository-controlled Story 11.3 scope passes locally. Canonical workflow checkpoints are
published in the same encrypted operational-store transaction as their session checkpoint, runtime
journal cursor, resume binding, immutable artifacts, and workflow-state fingerprint. Verified
restart accepts the exact record; projection failure rolls the entire publication back; record or
identity tampering blocks restart.

Restart reconciliation compares workflow state, source manifest, plan and active step, tool
catalog, model route, policy, environment, journal cursor, receipts, consumed grants, verifier
evidence, artifacts, schema, budget, lifecycle, and independently observed effect truth. Only an
exact current checkpoint with no applied effect can permit a later authority transaction. Stale
identity replans, uncertainty remains blocked, a verified completed effect is finalized without
replay, exhausted budgets stop, and terminal state remains terminal.

The retained crash campaigns cover before/after journal queue, flush, correctness-transaction, and
subscriber boundaries plus prepared, grant-consumed, attempt-recorded, launch-commit,
worker-returned, and result-reconciled authority boundaries. A persistent-supervisor reconstruction
test proves that client/view lifetime does not own task truth.

## Commands

Run `npm run story-11.3:resume:evidence:check` to validate the retained record. Run
`npm run story-11.3:resume:evidence:build` to re-execute the focused Rust tests, process-stop crash
matrix, strict Clippy gate, and regenerate the bound report.

## Scope boundary

This is the Story 11.3-owned local slice of `RV-52`, the reviewer protocol assigned by the security
review and engineering-runtime traceability manifest. It uses deterministic local fixtures. Native
installed-product execution and the `RV-52` scenarios assigned to Stories 16.4, 21.4, and 22.5 are
not claimed, nor are cross-platform packaging, independent release review, or release readiness.

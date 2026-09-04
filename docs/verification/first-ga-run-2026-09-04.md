# First GA Unattended Run — 2026-09-04

Continuation of [`first-ga-run-2026-09-03.md`](first-ga-run-2026-09-03.md). Current Git state at
the date boundary was branch `build/agentmage-ga`, checkpoint `61302255`, three commits ahead of
upstream while the sole worker was completing the Batch 78 full gate. Batch numbering continues
from the prior ledger.

## Batch 78 — Sprint 72 hosted repository and source evidence contracts

### Completed

- Closed 23 TASKS rows: Task 72.1.1 and all 8 implementation sub-tasks; Task 72.1.2 and all 4
  artifact sub-tasks; verification Sub-tasks 72.1.3.1/.2/.3; both Story AC; and all 5 Sprint AC.
  The pure projection covers 25 discovery/search/source/revision/release/rules/workflow/security
  families with immutable source coordinates, six explicit coverage states, pagination,
  permission/unavailable state, local-revision comparison, inert content, zero secret values, and
  fixed zero hosted/local effects. Promotions: 0. Substitutions: 0. Cumulative closed items: 328.
- Commits: `a6df5898` (hosted evidence contracts, two schemas, docs, 64-case corpus, truthful
  closures, and final supply-chain carrier), `438d7621` (source-bound 8-command Sprint 72 report),
  and `61302255` (one affected evidence regeneration pass). Commits including log: 4. Commits per
  closed item: 0.17. Review pins advanced: 0; complete `REVIEWED_PATHS` intersection across 31
  batch paths and all pin-bearing gates: empty.

### Validation and self-recovery

- Focused validation: 4 hosted-repository Rust cases, 1 artifact contract test, 3 evidence mutation
  tests, 83 runtime/planning schema tests, strict capability Clippy, format, supply-chain
  currentness, product-CI contract, configuration startup/result, component inventory, Story 3.1
  security/gate, planning scope, traceability, contract boundary/evidence, Stories 2.1/2.2/2.4,
  and Sprint 2 pass. Supply-chain builds: 1. Local report builds: 1. Downstream evidence
  regeneration passes: 1. Recovery iterations: 2 — corrected the enumerated governance corpus
  count from 24 to its exact 23, and renamed a zero-content evidence metric whose original field
  name is intentionally prohibited by the recorder. The full chain ran 689.74 seconds and stopped
  only at the retained Story 6.1 rootless-Podman prerequisite after every preceding gate passed.
  Recorded gate wall seconds: 690.
- Exact hosted reads and hosted/local comparison remain blocked on the Sub-task 72.1.3.4 tuple.
  Native hostile-content, permission, immutable-link resolution, manual fuzzing, and independent
  review remain blocked on the Sub-task 72.1.3.5 tuple. `substitution_set=empty` for both.
- Exact full-chain carrier blocker: `blocked: host change required — run npm run -s docs:check
  outside the restricted filesystem sandbox with the current user's /run/user/1000/libpod
  writable`; `substitution_set=empty`.

Exact next action: all remaining Sprint 72 rows depend on exact external tuples; continue Decision
0021 ordering at Sprint 73 GitHub issues, pull requests, checks, and reviews. The 325-item checkpoint
handoff is required after this batch push.

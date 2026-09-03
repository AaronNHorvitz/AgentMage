# First GA Unattended Run — 2026-09-03

Continuation of [`first-ga-run-2026-09-02.md`](first-ga-run-2026-09-02.md). Current Git state at
the date boundary was branch `build/agentmage-ga`, checkpoint `be901a82`, with no concurrent
worker. Batch numbering continues from the prior ledger.

## Batch 12 — Story 13.4 exact profile campaign

### Completed

- Closed 6 TASKS rows: Task 13.4.3, Sub-tasks 13.4.3.1 and 13.4.3.2, Task 13.4.4,
  Sub-task 13.4.4.2, and Sprint AC 13.AC10.
- Retained 8 exact synthetic context ledgers and 11 workflow cases under one fake identity. The
  ledger records extraction/accounting, coverage, omission, malformed-call, repair, tool-call,
  verified-completion, false-completion, attempt, diagnosis, logical-latency, no-model-process
  memory, and cancellation counts without aggregating tuples.
- Bound the exact rejected Muse, Gemma E4B, and Gemma 12B records separately. Rejected profile
  metrics remain null; admitted profile count: 0; future admitted profile count: 0; automatic
  fallback: false; profile, story, sprint, and release promotions: 0.
- Commits: `0ac52152`, `be901a82`, `baf0d79f`, `3e96adb2`, and `733f6c46`. Commits: 5. Commits
  per closed item: 0.83. External rows closed by substitution: 0.

### Validation and self-recovery

- Story 13.4 tests: 6/6 pass. Story 5.1 tests: 7/7 pass. Sprint 5 tests: 7/7 pass.
  Requirement/current-applicability tests: 45/45 pass. Markdown: 412 files, 0 issues.
  Review-provenance, traceability, task-graph, schema, Clippy, and supply-chain checks pass.
- Supply-chain regeneration count: 1; output delta: 0. Evidence regeneration count: 1 before
  self-recovery. Review-path audit initially matched ordinary references textually; an AST audit
  found 0 initial `REVIEWED_PATHS` intersections. The mistaken working-tree pin edits were fully
  restored before commit.
- Self-recovery regenerated stale Story 3.1 and Story 4.1 security dependencies, then found the
  resulting Story 5.1 security map and Story 5.2/5.3 reports were genuinely reviewed inputs.
  Section 7 required 2 ordered aggregate checkpoints: Story 5.1 pins `baf0d79f`; Sprint 5 pins
  the resulting story checkpoint `3e96adb2`. Pins advanced: 2. No human reviewer claim was made.
- Gate wall seconds: 450 across explicitly timed supply-chain, evidence, recovery, and document
  checks. The known Podman checkpoint was not retried after its 5 prior identical failures.
- Remaining checkpoint blocker: `blocked: host change required — run npm run -s docs:check,
npm run -s evidence:story6.1-security:build, npm run -s evidence:story7.1-security:build, and
python3 scripts/sprint_60_evidence.py --write --source-revision 733f6c46 outside the restricted
filesystem sandbox with the current user's /run/user/1000/libpod writable`; substitution set:
  empty.

Exact next action: continue the first authoritative unblocked frozen-scope gate after Story 13.4;
do not activate any model or borrow the retained rejected-candidate results.

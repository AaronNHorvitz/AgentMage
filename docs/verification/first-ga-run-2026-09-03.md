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

## Batch 13 — Sprint 9 security-review prerequisite audit

### Completed

- Closed rows: 0. Promotions: 0. External rows closed by substitution: 0.
- Commits: `9ec52de1` and `c813ed06`. Commits: 2. Commits per closed item: undefined
  because 0 rows closed.

### Validation and self-recovery

- The initial audit correctly identified that AGENTS.md section 7 permits automated review-gate
  provenance, but focused Story 9.1 and 9.2 validators found lower prerequisite artifacts stale
  before either security map could be regenerated.
- Story 9.1 requires a fresh Linux package-lifecycle artifact; Story 9.2 requires a fresh Docker
  prerequisite artifact. Both repository build commands require Podman and therefore the same
  writable `/run/user/1000/libpod` state already exhausted in 5 recovery attempts. No sixth
  attempt, weakened validator, manual artifact edit, or support substitution was made.
- The provisional automated-review source and 8 provisional TASKS closures were reversed in a
  forward commit. Both affected story rows now record exact host-change blockers; the retained
  security maps remain unchanged. Supply-chain regeneration count: 1; output delta: 0. Evidence
  regeneration count: 0. Review pins advanced: 0. Gate wall seconds: 0.
- Exact blockers: `blocked: host change required — run npm run -s
evidence:story9.1-linux-package:build outside the restricted filesystem sandbox with the current
user's /run/user/1000/libpod writable` and `blocked: host change required — run npm run -s
evidence:story9.2-docker-prerequisite:build outside the restricted filesystem sandbox with the
current user's /run/user/1000/libpod writable`; substitution sets: empty.

Exact next action: continue with Sprint 10 repository-controlled strict-local work while the
Sprint 9 package-evidence checkpoint remains blocked on the host change.

## Batch 14 — Frozen-scope local breadth criteria

### Completed

- Closed 5 TASKS rows: Sprint AC 10.AC4 and Sprint AC 11.AC2 through AC5.
- Added one hash-bound aggregate over 8 retained strict-local, crash, encryption, canary,
  derived-export, storage-security, and durable-resume artifacts. Promotions: 0. External rows
  closed by substitution: 0.
- Commits: `00a7c851` and `76b4341b`. Commits: 2. Commits per closed item: 0.40.

### Validation and self-recovery

- Aggregate mutation tests: 3/3 pass. Bound criteria: 5/5. Markdown: 413 files, 0 issues.
  Traceability, task graph, planning scope, and report checks pass.
- Product truth remains false for installed-product completion, native cross-platform completion,
  Story/Sprint 10 completion, Story 11.1/Sprint 11 completion, release, and external-evidence
  substitution. Review-path intersections: 0; review pins advanced: 0.
- Supply-chain regeneration count: 1; output delta: 0. Evidence regeneration count: 1. Gate wall
  seconds: 75. Known Podman checkpoint retries: 0.

Exact next action: continue with the next dependency-independent frozen-scope gate after the
remaining Sprint 10 and 11 installed-product dependencies.

## Batch 15 — Candidate-neutral gateway story closure

### Completed

- Closed 2 TASKS rows: Story 13.5 and Story 13.6.
- Story 13.5 retains enabled candidate count 0, selected route count 0, live endpoint
  qualification false, and `story_completion_claim: true`. Story 13.6 retains live remote
  execution false, default fallback false, 0 silent transitions, and
  `story_completion_claim: true`. Promotions beyond the bounded story contracts: 0.
- Commit: `6021c8f2`. Commits: 1. Commits per closed item: 0.50. External rows closed by
  substitution: 0.

### Validation and self-recovery

- Story 13.5 tests: 4/4 pass. Story 13.6 tests: 3/3 pass. Markdown: 413 files, 0 issues.
  Traceability, task graph, planning scope, and both retained evidence checks pass.
- Supply-chain regeneration count: 1; output delta: 0. Evidence regeneration count: 1.
  Review-path intersections: 0; review pins advanced: 0. Gate wall seconds: 75. Known Podman
  checkpoint retries: 0.

Exact next action: continue at the next incomplete dependency gate in Sprint 13, preserving zero
admitted profiles and the open native-platform/profile-parity blockers.

## Batch 16 — Sprint 14 inventory evidence renewal audit

### Completed

- Closed 0 TASKS rows. Promotions: 0. External rows closed by substitution: 0.
- Audited the 416-entry frozen source inventory and locally passing inventory/security boundary.
  Four provisional closures were not retained because the mandatory Sprint 14 regeneration reruns
  the stale Story 9.2 Docker prerequisite validator.
- Commits: `12178066`, `1aa9fe9e`, and `c2c2453d`. The first two preserve the failed attempt and
  its forward reversal; the third records the exact blocker. Commits per closed item: undefined
  (0 items).

### Validation and self-recovery

- Supply-chain regeneration count: 1; output delta: 0. Sprint 14 evidence regeneration attempts:
  1. The retained report was restored only by the forward revert, not by reset, checkout, manual
  artifact editing, or history rewriting.
- The failing command was the 37-test installer-package closure: 36 passed and
  `tests.test_linux_docker_prerequisite_evidence.test_exact_report_is_valid` failed with
  `Docker prerequisite component closure changed`. The isolated 4-test prerequisite suite
  reproduced 3 passes and the same 1 failure. Known Podman checkpoint retries: 0.
- After the forward reversal, the committed Sprint 14 local report again validates. Markdown:
  413 files, 0 issues. Documentation invariants, task graph, requirement registry, planning scope,
  and traceability pass. Review-path intersections: 0; review pins advanced: 0. Gate wall seconds:
  75.
- Exact blocker on Tasks 14.2.1 and 14.2.3 and their candidate closure rows:
  `blocked: host change required — run npm run -s evidence:story9.2-docker-prerequisite:build and
  then python3 scripts/sprint_14_evidence.py --write outside the restricted filesystem sandbox
  with the current user's /run/user/1000/libpod writable`; substitution set: empty.

Exact next action: continue with the next dependency-independent frozen-scope gate; do not retry
the exhausted Story 9.2/Sprint 14 Podman checkpoint in this environment.

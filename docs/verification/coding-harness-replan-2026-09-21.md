# Coding Harness Replan and Restart Handoff

## Operator State

The owner requested stopping AgentMage and revising the architecture around a
standalone coding harness on 2026-09-21. The prior `agentmage-claude:0.0` worker
was at a prompt with `start 76.2.1.1` entered. Its Claude process and four
background polling shells were terminated; their process IDs were checked gone,
and the tmux session was closed. The session history remains in Claude's normal
local records. The stop marker is `~/.local/share/agentmage-run/STOP-CLAUDE`.

AgentMagik, CodingMage and USTE workers were left running. No other project was
edited. Do not automatically recreate AgentMage's session or remove its marker.
An operator restart is required before implementation resumes.

## Baseline and Scope

- Repository branch: `demo/fedora-local-docs`.
- Clean pre-edit revision: `6f90f81cbeaa930674a2f98e8ee27bfd8814a59b`.
- OpenCode source reference: `e059ac5918f3e2c798de029b9df4cede617466ed`.
- This batch changes design, product requirements, execution tasks, priority
  selection and associated planning validation, not the product runtime.
- No upstream engine is vendored or installed. No model is downloaded,
  activated or qualified. No runtime/independent-review/release gate is closed.
- No commit or push is performed as part of this revision unless separately
  recorded below. Prior branch and publication permissions are not broadened.

## Read Before Restart

1. `AGENTS.md`, including Section 8; Decision 0061 and retained Decision 0054
   resource/authority limits.
2. PRD Section 40 and Implementation Plan Section 15.
3. `docs/architecture/standalone-coding-harness.md` in full.
4. The new unchecked Tasks 48.2.4, 48.2.5, 48.2.6 and 50.2.4 in `TASKS.md`.
5. Current Git state, source entry points, exact prerequisite artifacts and the
   regenerated `remaining-plan-blocker-audit.json`; verify, do not trust summaries.

## First Implementation Unit

Start with Sub-task **48.2.4.1**: determine the real launch, trust, state/key,
confinement, model/runtime/codec and resource prerequisites. Produce a concrete
activation/composition plan and exact dependency owners. Do not simply remove
`TransportFailed` or `platform_activation_required`.

Then implement the connected coding foundation and live controls as a coherent
batch. Produce the exact local coding-model prerequisite alongside that work;
do not wait for the whole Sprint 49 gate or replace it with the successful
document-QA demo. Scripted-provider integration can proceed before real-model
qualification, but its report must label that distinction.

Do not resume desktop-shell Story 76.2 merely because the previous worker's
handoff selected it. Do not launch a competing OpenCode backend. Preserve the
existing coordinator, native dispatcher, kernel authorization and canonical
stores. Other stack projects remain outside this workstream.

## Acceptance and Stop Rules

Use the architecture's executable acceptance matrix. Real binaries and real
effects in disposable synthetic repositories are required. Preserve existing
human work and demonstrate actual failure/correction, denial, stale-state refusal
and cancellation/descendant cleanup. Keep single-session and daily-use thresholds
distinct. The implementing agent cannot supply its own independent review.

Observe `STOP-CLAUDE` before beginning and at every increment boundary after an
authorized restart. No planning priority authorizes spending, secrets, new
accounts, production signing, publication, releases, force-pushes or default-branch
merges. Keep systemd memory caps and the existing resource rule. Preserve any
new user changes discovered at restart.

## Verification Record

Planning validation and current-source evidence binding renewals from this
revision are recorded below. Nothing in this section is new coding-workflow
runtime acceptance evidence.

### Focused Validation

- The 42 tests in `tests.test_remaining_plan_blocker_audit`,
  `tests.test_task_graph`, `tests.test_planning_scope` and
  `tests.test_context_safety_registration` pass. The selector suite includes
  nine new regression tests for coding priority, external/unknown/unresolved
  prerequisites, cycle handling, completed work, exact task-prefix scope and
  exclusion of trailing story-gate commentary from the new row dependencies.
- The 30 new rows (four tasks and 26 sub-tasks) are all unchecked, have resolved
  references and an acyclic transitive prerequisite graph. All 5,686 pre-existing
  task/acceptance lines remain byte-for-byte unchanged. Existing historical
  story-level graph limitations were not represented as globally repaired.
- The generated blocker register selects `execute-local:48.2.4.1` and binds the
  Decision 0061 hash. Its `unattended_ready` field means dependency readiness,
  not permission to ignore the operator stop marker.
- `requirements:current-check`, `planning-scope:check`, `policy:check`,
  task-graph validation, Markdown lint, Mermaid validation, documentation policy
  validation and `git diff --check` pass. The accepted 294 stable requirements,
  53 normative mappings and 1,407 protected inventory checklist entries remain.
- Regenerated policy/traceability metadata binds the current PRD, task locations
  and existing inventory bytes. The inventory source and normative-map baseline
  are unchanged. No first-party runtime crate, dependency manifest, lockfile or
  product status record changed, and historical immutable evidence was not edited.
- Checks ran under the existing systemd scope limits: `MemoryHigh=5G`,
  `MemoryMax=6G`, `MemorySwapMax=512M`. The full documentation gate also executes
  existing Rust contract fixtures. No live-model coding, installed coding-workflow
  acceptance or independent review was performed for this planning batch.

### Documentation Pipeline

Every step in the repository's `docs:check` script passed across staged runs.
The initial `npm run docs:check` invocation passed Markdown, Mermaid,
documentation policy, task graph, current requirements, schema and dependency
disposition checks, then failed the schema-evolution authority digest check.
It did not exit successfully as a single command.

The revision changed two whole-file documentary authorities. Only their digest
fields were renewed in `architecture/schema-evolution-and-rollback.json` and
`architecture/parser-ocr-platform-placement.json`. A structured comparison
confirmed all other fields were unchanged. The current contract boundary and
evidence bundle was regenerated using `contract-boundary:build` and
`contract-evidence:build`; both passed. No historical immutable evidence,
threshold or validator was weakened.

The pipeline was resumed at `schema-evolution:check`. Schema evolution, parser
placement, contract evidence, Story 6.1, Sprint 6, the macOS manifest and Story
7.1 checks passed. The final segment, from `evidence:sprint7-gate:check` through
`evidence:story8.1-hosted-compatibility-source:check`, was rerun and exited zero,
with all 18 checks passing. This is staged validation of the full script, not
a claim of a fresh uninterrupted `npm run docs:check` pass. Platform source
contract fixtures are not native-platform or installed-product qualification.

### Final State

The AgentMage tmux session remains absent and its stop marker remains present.
AgentMagik, CodingMage and USTE sessions were left untouched. The initial handoff
was local and uncommitted. The owner subsequently authorized committing and
pushing this revision to `origin/demo/fedora-local-docs`. Publication does not
authorize restarting the worker. Runtime implementation starts only after an
operator restart, beginning with Sub-task 48.2.4.1 above.

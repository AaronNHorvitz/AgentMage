# Context and Crash-Safe Resume

## Boundary

Sprint 22 adds a deterministic context composer, checked continuity summaries, metadata-only
session checkpoints, drift revalidation, and SQLCipher-backed atomic publication. These records are
descriptive. They do not grant authority, open paths, read files, select a model, or permit an
effect. Existing platform, path, policy, grant, and authority-transaction boundaries remain
authoritative.

## Context Composition

`compose_context` accepts already-minimized candidates classified as instructions, newest request,
active objective, plan step, corrections, approvals, blockers, evidence, memory, expected output,
or supporting context. Every candidate carries an exact token count from a pinned counter, byte
count, source identity, source revision, content digest, sensitivity, admission state, and an
essential flag.

Composition is deterministic across input order:

1. denied and stale candidates are excluded visibly;
2. authoritative evidence wins exact-source deduplication;
3. essential and higher-priority categories consume the inclusive byte and token budgets first;
4. nonessential overflow is recorded as a budget omission;
5. essential eligible overflow fails closed instead of silently dropping current intent; and
6. the packet includes content-free accounting for every input identity.

The debug accounting includes labels, counts, source identities, revisions, digests, and omission
reasons. It contains no excluded excerpt. A restricted denied canary test proves prohibited content
does not enter the packet or its debug view.

## Checked Summaries

`CheckedContextSummary` preserves bounded continuity prose plus paths, errors, identifiers,
commands, decisions, unresolved questions, evidence IDs, citation IDs, and receipt IDs. It stores a
digest of the checked source set, not source evidence itself. Current summaries may be used only as
non-authoritative context. Stale, disputed, or insufficient summaries require reopening original
sources.

## Safe-Boundary Checkpoint

`SessionCheckpoint` contains identities and digests for the current objective, plan revision, plan
step, next action, workspace, repository snapshot and branch, structural map, required files,
instructions, permission profile, policy, exact model profile/manifest/runtime, evidence,
citations, blockers, context packet, action state, consumed grant, and terminal receipt. It contains
no source bytes, prompt text, credentials, grant nonce, approval secret, or ambient path authority.

The canonical checkpoint digest is computed with its digest field zeroed. Unknown fields, malformed
identities, duplicate references, impossible option pairs, noncanonical hashes, digest drift, and
future schema versions fail closed.

## Atomic Publication

Operational-store schema version 4 adds immutable `session_checkpoints` and binds the current
checkpoint digest into both `store_metadata` and every authority-state generation. The terminal
effect path publishes the terminal action state, consumed grant revision, receipt, evidence IDs,
and next checkpoint in one `BEGIN IMMEDIATE` SQLCipher transaction. A failed checkpoint build or
publication poisons the in-process runtime and requires reopening canonical state.

All checkpoint history rows are parsed and hash-verified during open. The publication generation
must point to the same checkpoint digest. Record, link, schema, and digest tampering block startup.
An injected database failure proves authority metadata and the checkpoint roll back together.

Ephemeral checkpoints are valid in-memory continuity records but are rejected before durable
publication. The ephemeral test leaves the operational-store generation unchanged and retains no
session-checkpoint row.

## Resume Revalidation

Before another action, `revalidate_resume` compares the checkpoint with caller-held current
observations across exactly nine material dimensions:

| Dimension | Compared material |
|---|---|
| Workspace | Workspace identity and complete environment-capture digest |
| File | Required object, content, and revision identities |
| Instruction | Effective instruction digest |
| Branch | Exact branch or detached-head marker |
| Repository map | Pinned structural-map digest |
| Citation | Complete citation-set digest |
| Model | Exact profile, manifest, and runtime digests |
| Permission | Effective profile identity and digest |
| Policy | Deterministic policy identity and digest |

No drift returns a descriptive Continue directive. Any drift returns the complete sorted dimension
set and requires an explicit Continue, Restart, or Cancel choice. Continue means re-checkpoint the
accepted current state; it is not direct action authority.

## Crash Evidence

The deterministic subprocess campaign performs 126 forced exits, covering before and after each of
nine store boundaries seven times. The new session-checkpoint boundary recovers to exactly one
checkpoint whether termination occurred before or after publication. Existing authority crash tests
also cover every effect transition. A malformed terminal checkpoint after worker completion poisons
the runtime; restart terminalizes from durable reconciliation state and replay starts no worker.

The native Linux coordinator campaign adds 100 no-unwind restarts: 25 each immediately before and
after the durable tool-terminal event commit and immediately before and after the checkpoint commit.
Every encrypted-authority reopen retains one terminal receipt and one durable worker launch. A
terminal event appears only after its transaction commits, and a checkpoint appears only after its
transaction commits. The source-bound report and redacted trace are retained under
`artifacts/sprints/sprint-22/story-22.1/`. This does not claim a stop inside a filesystem or SQLite
syscall, physical power loss, installed-package behavior, another platform, or independent review.

The native long-session campaign runs the production coordinator to the longest safe-boundary
prefix admitted by its unchanged fixture profile: seven read-only tool turns, receipts, checkpoints,
and continuation artifacts. Reopen rejects repository, policy, workspace, configuration,
model-profile, model-manifest, and model-runtime changes before action. Exact state restores the
canonical artifact set, objective, and receipt/history counts and completes without an eighth
worker launch. Its source-bound report and redacted trace are retained beside the native crash
matrix. The deterministic model and exact model/runtime records do not substitute for installed
real-model, clean-image, cross-platform, or independent-review evidence.

## Current Product Canary Boundary

The Story 22.1 canary campaign composes eight existing checks across every currently named source
surface that can carry or derive session content: context admission, persistence minimization,
encrypted store/export/crash diagnostics, transcript/event/diagnostic/metric projections,
owner-bound private artifact reads, model/tool runtime events, native staging/object ciphertext, and
the strict-local external-telemetry audit. Its source-bound report and repository-root-redacted raw
trace are retained as `current-product-canary-sweep.json` and `.log` under the Story 22.1 evidence
directory.

This is a current Linux source-surface boundary, not a forensic or release claim. Exact authorized
private-artifact reads remain an intentional content-returning boundary. The campaign does not
inspect process memory, swap, SSD remanence, filesystem snapshots, external host instrumentation,
an installed real model, an installed package, another supported platform, or an independently
reviewed build. A new content-bearing source surface invalidates this coverage until the campaign
is extended and rerun.

## Traceability

| Requirement | Implementation | Local verification |
|---|---|---|
| `S-020-I01`, `S-020-I02` | `compose_context`, complete accounting, deterministic priority and dedupe | Empty, nominal, maximum, overflow, duplicate, stale, denied, and canary fixtures |
| `S-020-I03`, `S-020-I04` | `CheckedContextSummary`, `evaluate_checked_summary` | Current, stale, disputed, insufficient, duplicate-reference fixtures |
| `S-020-I05` | `SessionCheckpoint`, `finalize_checkpoint`, `verify_checkpoint` | Valid, corrupt, future-version, mismatched-field, and tamper fixtures |
| `S-020-I06` | Operational-store migration 4 and terminal checkpoint execution path | Atomic rollback, terminal receipt binding, reopen, and replay tests |
| `S-020-I07`, `S-020-I08` | `revalidate_resume`, explicit drift decisions | Every drift dimension and all three decisions |
| `AT-CRASH-001`, `AT-RESUME-001` | Platform-neutral store campaign, native Linux crash and long-session campaigns, and deterministic revalidation | 126 store-boundary exits, 100 native tool-terminal/checkpoint exits, seven-checkpoint exact resume with seven drift classes, and 100 repeated no-drift comparisons |
| `SR-DAT-002`, `SR-DAT-003`, `SR-AI-008`, `SR-AI-010`, `SR-OPS-003` | Current source-surface canary composition | Eight hash-bound Linux checks spanning context, persistence, encrypted and projected artifacts, model/tool events, native ciphertext, and telemetry closure |

# Durable Attempt Recovery and Resume

Status: Story 22.4 local implementation

AgentMage resumes interrupted work from runtime-owned records, never from transcript text or model
output. `CanonicalAttemptCheckpoint` refines the existing session and workflow checkpoint at one
exact correctness-event cursor. It contains only identifiers, digests, counters, and closed state;
it deliberately cannot encode tool arguments, executable calls, grant contents, or prose that
could reconstruct an old effect.

## Durable binding

The attempt checkpoint binds all recovery-relevant state:

| Family | Authoritative comparison on resume |
| --- | --- |
| source, plan, active step | exact source, plan/revision, execution, and step identities |
| model and context | exact model profile/runtime and model-visible context digests |
| tools and policy | admitted catalog and effective policy digests |
| grants and approvals | ordered ledgers, including consumed authority |
| preflights and attempts | current typed facts and complete ordered attempt history |
| receipts and artifacts | terminal receipt ledger and immutable artifact-set digest |
| verifier | current verifier/evidence identity |
| budgets and progress | immutable budget policy, every consumed counter, complete repeated-state history and count |
| journal and environment | exact correctness cursor and trusted environment digest |
| effect truth | no effect, verified not applied, verified applied, or uncertain |

`seal_attempt_checkpoint` hashes canonical JSON with the seal field zeroed. The encrypted
operational store migration 18 persists the sealed record append-only and verifies its canonical
bytes, indexed fields, workflow checkpoint, runtime run, session, and event foreign keys whenever
the store opens. Missing or mismatched authority blocks restart.

The budget tuple retains parser repairs, targeted model repairs, step attempts, all 14 error-class
counters, total workflow work, and replans. The checkpoint also retains the repeated-state history
digest and current count. Restart therefore cannot reset one retry family or forget a no-progress
loop.

## Recovery decision

`decide_attempt_recovery` revalidates every family and selects exactly one closed action:
`continue`, `fresh_attempt`, `deterministic_repair`, `replan`, `await_approval`,
`await_dependency`, `reconcile_effect`, `cancel`, or `terminate_diagnosed`.

The decision carries no authority and always sets `prior_effect_replay_allowed` to false. A
`fresh_attempt` requires the existing retry-admission and workflow-identity boundaries to issue a
new attempt, call, tool-call, grant, and per-attempt approval identity. A checkpointed
`verified_applied` or `uncertain` effect can only enter reconciliation. Policy, grant, or approval
drift awaits fresh approval; other material identity drift replans; inconclusive postconditions
reconcile before any successor.

Budget exhaustion, repeated state, invalid durable state, and an already-terminal execution produce
one content-free terminal diagnosis. It names the failed step, last checkpoint, attempt-history and
verifier identities, every exhausted budget, blocked reason, uncertainty, approval need, and safe
resume action. Neither a model nor a client can convert this diagnosis into success.

## Concurrent callers and interruption

Clients sharing one open store first pass the in-runtime ownership gate. The winning decision is
then claimed in `workflow_attempt_resume_claims` with a unique checkpoint primary key. The claim is
immutable and survives restart, so a second Chat, CLI, or headless caller cannot become another
owner or publish a competing current decision.

The local evidence campaign interrupts a separate process before or after preflight, approval,
dispatch, effect, receipt, artifact, verification, retry decision, recovery decision, checkpoint,
and terminal commit across 100 deterministic seeds. It reloads and verifies every retained
checkpoint and proves zero permitted replay. Separate tests exercise 32 concurrent clients,
encrypted-store reopen, append-only enforcement, all identity mutations, all nine actions, budget
and repeated-state exhaustion, and closed JSON schemas.

Retained evidence is under `artifacts/sprints/sprint-22/story-22.4/`. It maps local results to
`RV-12`, `RV-16`, `RV-17`, `RV-18`, and `RV-25`. Installed-package Windows runs, physical
power-loss/storage-fault testing, and independent human review remain external qualification and
are not represented as complete.

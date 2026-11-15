# Thinking-disabled Muse checkpoint — 20 of 30 development tasks

Date: 2026-11-15 UTC. Source: `a896f055256e96baec09966c71c5d0a7f1714d96`.
**20/30 attempted; ten remain.** There are **13 strict ungraded proposals**;
first-attempt and within-budget task passes remain **unavailable**. This is an
incomplete repeat, not a full-suite result. T-303/T-306 remain open.

## Observed outcomes

| Outcome / cause | Tasks |
| --- | --- |
| `ungraded-proposal` | 13 |
| `strict-edit-refused:code.proposal.edit` | 2 |
| `strict-edit-refused:code.proposal.format` | 2 |
| `generation-refused:error:chat.transport.timeout` | 3 |

Request/check time totals **3304.704 seconds**. Reserved-work timers total
**3733.673 seconds / 62.228 minutes**, including verification, model loading,
calls and cleanup after frozen-input validation. Queues, wrapper/pre-entry
overhead and released pauses are excluded. Active GPU kernel time and continuous
end-to-end elapsed time are unmeasured. Backend counters cover **17/20**:
**111197 input / 99327 output tokens**, including non-answer output. Missing
counters for the three timeouts are unavailable, never zero or reconstructed.
Seventeen response captures are retained; missing captures are `dev-004`,
`dev-011` and `dev-012`. All available request, event and backend records remain.

## Path, inputs and pending work

The measured partial path selects context from each statement and editable
frozen base source, calls the credential-owning chat API, then checks the answer
with the unchanged Rust strict checker. Native executable admission and the
original grading-control receipt still block full repository retrieval, native
effects, tests, feedback and retries. No candidate, hidden or held-out test ran.
No original control is reconstructed; accepted proposals are not solved tasks.

Context is **32768**; output allowance is **24395–29826**, using
`min(32768, 32768 - rendered input tokens - 32)`. Input preflight is
**2910–8341**; **does not fit: 0**. These are the largest allowances of this
prepared profile and transport. Changed variable remains trusted API
`thinking_mode: disabled`, encoded upstream as
`chat_template_kwargs: {"enable_thinking": false}`. The prepared Muse template
retains `reasoning_strength: medium`. This records the request without claiming
model support. All 17 retained response captures contain non-empty reasoning output;
the three timeouts have no capture. Complete framing, JSON-object output,
prompts, checker, profile,
300-second idle/total deadlines and budget formula remain fixed.

Frozen-input SHA-256:
`85dc3c08ba406fc4fae5cc3c49bcad9684bd1fe27c1743c5aa05b0acfea85abd`.
The audit verifies all **2283 repository / 20 private bindings**, model/runtime
identity, request bodies, log hashes, selected source hashes and counters.
Inherited policy labels still describe preceding transport/format changes;
`collector_policy.changed_variable` also calls the server template default.
Actual source and thinking-option requests are bound by the manifest and
captures; those frozen labels affect no count and remain T-309 backlog.

The controller uses separate two-task jobs through both resource wrappers,
900-second execution caps after GPU admission and 30-second released pauses.
Two jobs completed before the November 14 stop; eight after the November 15
restart. Earlier queued sessions and an interrupted pre-task preparation remain
excluded from attempt totals. This session advanced from 18 to 20 attempts.
Completed attempts were never replayed. Every completed pair revoked settings
and reaped API/model owners; keys and sockets were absent at inspection. Exact
process checks cover only this PID namespace, with no global-idle claim.

The next pair, tasks 21–22, had waited **1332.798 seconds** for a shared build
slot at the retained observation, with no job directory or task attempt. That
wait exceeds the normal twenty-minute yielding interval and is separate from
model failures. The same exact owner remained queued at inherited nice 19,
16 GiB/four CPUs and low build priority; no resource increase or other-worker
signal occurred. Finish all ten remaining tasks before tuning. This cumulative
checkpoint is included in a later full repeat, never added to it.

## Retention and verification

A read-only prefix audit retains the original full-repeat validations, changing
only the iteration prefix, expected task set and reported count to twenty.
It executes no candidate, test or inference. Original-helper reproduction is
**not rerun for this checkpoint** because the build reservation is unavailable;
no new reproduction or full-repeat verification pass is claimed.

The private `thinking-disabled-muse-checkpoint20-20261115-v2` capsule contains the frozen inputs,
requests, available responses, records, logs, one declared source per task and
read-only verification methods. Archive SHA-256:
`8239b473e41fa1bb9ebd9fda8f0cb73388e7e871d9615483d34e6a29517d7bc9`.
The verifier streams all **2616 files**, checks hashes and independently
recomputes the reported twenty-task totals without extracting or executing
archived code. It verifies exactly twenty selected candidate source files.
An initial retention method included complete projected candidate folders;
its replacement narrows that scope before export. The original private capsule
and methods are preserved, and no candidate code was executed. A launcher
suffix assertion also failed before process creation and was corrected before
this session's two attempts; both versions and the correction receipt remain.

The reporting fast gate, Markdown rendering and preservation/privacy checks
are recorded separately. All 507 ledger table lines and the scoreboard prefix
must remain unchanged. The retained independent review for exact
`45b13def95fa4604ba189532dd408f21668ba20c` is PASS with no high/medium finding;
its low request-path finding is addressed by review commands using only retained
review artifacts, without live builder state. This checkpoint requires a new
independent core review and closes no row.

The inherited source gate remains **1167 passed / 53 failed / 19 ignored**,
not rerun here. A green documentation gate does not make that source gate green.
No source, prompt, task, hidden test, threshold, profile, permission or SBOM
changes; no model qualification, release or owner-readiness claim follows.

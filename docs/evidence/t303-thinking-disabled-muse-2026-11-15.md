# Thinking-disabled Muse development repeat — 2026-11-14–15 UTC

All 30 frozen development tasks ran at source `a896f055256e96baec09966c71c5d0a7f1714d96`:
**19/30 strict ungraded proposals**. First-attempt and within-budget task passes
remain **unavailable**. T-303/T-306 remain open; independent core review is required.

## Measurement

The available path selects context from the statement and editable frozen base
file through the existing Rust helper, calls the credential-owning chat API and
checks the answer with the unchanged Rust strict checker. Native executable
artifact admission and the original grading-control receipt still block full
repository retrieval, native effects, tests, feedback and retries. No original
control is reconstructed. No candidate, hidden or held-out test ran. Proposals
remain isolated, unapplied and untested; they are not solved tasks.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 4 |
| `strict-edit-refused:code.proposal.edit` | 3 |
| `strict-edit-refused:code.proposal.format` | 4 |
| `ungraded-proposal` | 19 |

Request/check time totals **4798.465 seconds**; reserved-work timers total
**5430.492 seconds / 90.508 minutes**. These timers start after frozen-input
validation and include model/runtime verification, loading, calls and cleanup.
Queues, wrapper/pre-entry overhead and pauses are excluded. Active GPU kernel
time and continuous end-to-end elapsed time are unmeasured. Backend counters
cover **26/30**, reporting **166355 input / 150086 output tokens** including
non-answer output. Raw per-task times, tokens and causes are retained. Missing
counters are unavailable, never zero. Backend terminations:
`{"stopped-below-token-limits": 26, "unobserved-or-ambiguous": 4}`.
Upstream response captures cover **27/30** attempts; missing:
`["dev-004", "dev-011", "dev-012"]`. Available request, event and backend records remain;
missing bodies or counters are never reconstructed. Captures comprise
**26 complete JSON responses and 1 truncated prefix**.
Task `dev-027` timed out after a retained 16,384-byte prefix (SHA-256
`fda17450727426cd325401a00a1ae57051d1f92e2bde45d8bb760211045c9d20`).
That incomplete JSON is not a completed response and supplies no final counters.
It is retained and hash-checked, never repaired or reconstructed.

## Fixed inputs and changed variable

Changed variable: trusted API `thinking_mode: disabled`, encoded in every
captured upstream request as `chat_template_kwargs: {"enable_thinking": false}`.
The prepared server template retains `reasoning_strength: medium`. This records
what was requested; it does not establish that this model supports or obeys the
thinking extension. Captures show non-empty reasoning output in
**26/26** complete retained responses. Reasoning
for the partial response and the 3 missing captures is unavailable. Complete framing, JSON-object output, prompts, strict
checks, profiles, 300-second idle/total deadlines, temperature 0, seed 42,
four threads, Q8 caches, batch 256 and microbatch 128 stay fixed.
Context is 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)`, ranging **24395–29826**.
Input ranges **2910–8341**; **does not fit: 0**, separate from refusals.
These are the largest allowances in this prepared profile and transport.

Source freeze SHA-256:
`85dc3c08ba406fc4fae5cc3c49bcad9684bd1fe27c1743c5aa05b0acfea85abd` (2,283 repository and 20 private bindings).
API SHA-256: `ba623b925414e0b390533b18c59b060ad7058c8996d44741420a29b433d05ac1`.
Helper SHA-256: `852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
The development profile is a frozen repository input; its original model hash
and runtime are verified before every model launch. Model SHA-256:
`4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e` (16756683904 bytes).
All source and private bindings pass before every job and in the audit.
No weight download, model role adoption or driver/profile qualification follows.

Erratum: inherited private policy labels `chat_transport_binary_source` and
`comparison` still describe the preceding source and text-to-JSON-object change.
The inherited `collector_policy.changed_variable` also calls the template
arguments default; this profile retains `reasoning_strength: medium`. The actual
API source is `a896f055256e96baec09966c71c5d0a7f1714d96` and the changed variable is the trusted thinking
option, established by source fields, API hash and captured requests.
These labels affect no number. Frozen policy and previous sealed capsules stay
unchanged; label hygiene remains T-309 backlog.

## Verification and review

The measurement runs 15 separate two-task jobs through both resource wrappers,
with a 900-second execution cap after GPU admission and 30-second pauses outside
reservations. Two jobs (four tasks) completed on November 14 before the operator
stopped the lane while the next pair waited for the build slot. That waiting
wrapper and its owned controller were stopped before the next job directory
existed; no running model or other worker was stopped. The original controller
log ends with that cancellation, not successful completion.

After the owner's November 15 restart, each resumed segment revalidated all
frozen inputs, the four copied profiles, completed-job cleanup and the saved
attempt hashes before skipping completed pairs. Earlier resumed segments
completed tasks five through eighteen; segment 29 completed tasks nineteen and twenty; segment 31 resumed
from task twenty-one after verifying that the prior owner was absent and all
twenty completed record hashes were unchanged. Unique controller and reservation logs, launches, exact-argument
and start-tick observations, and absence receipts remain hash-bound. Several
namespaces ended queued before any new job directory or attempt existed. Empty
controller output and original queue logs distinguish these waits from attempts.
No completed attempt was replayed, and no other worker was signalled. Resource
limits and low queue priority stayed unchanged. Segment 25 inherited nice 19;
its method prose incorrectly retained nice-10 wording and a prior segment's
task-15/16 completion. Its numeric field and actual output establish nice 19
and task-17/18 completion, with the preceding segment queued only. Segment 26
inherited nice 10 but said 19 in method prose. Before task-19 admission, exact
controller and queue descendants were lowered to nice 19; the separate correction
receipt binds their identities and priorities. Both original launches remain
unchanged. Segment 26 then ended queued, with no task-19 directory or
attempt; segment 27 inherited nice 19 and resumed from task nineteen.
Segment 27 also ended queued before an attempt; segment 28 resumed the same
unattempted pair with inherited nice 19.
Segment 28 likewise ended queued before an attempt; segment 29 resumed task
nineteen with inherited nice 19. Its launch preflight assertion was corrected
before any process or attempt started; both method versions are retained.
Segment 29 ended queued before task twenty-one. Segments 30 and 31 preserve its
completed20 log, original queue log and checkpoint receipt; no earlier attempt
was replayed. Segment 30 ended queued with no new attempt; segment 31
verified its absence, retained all twenty record hashes and resumed the same pair.
Segment 31 completed tasks twenty-one and twenty-two, then ended queued
before task twenty-three. Segment 32 verified exact prior-owner absence and all
22 record hashes before resuming task twenty-three. An initial launch preflight
refused its inherited nice value before launching; the subsequent nice-19 launch
started one controller but its metadata write failed because a local variable
held a closed file instead of the frozen-input object. A separate read-only
method verified that exact live argv, start tick, priority, frozen inputs and
22 record hashes, then recovered the launch receipt without restarting it.
Both methods are retained. The earlier20 and22 checkpoints are cumulative and
included here, never added.
No resource ceiling or queue priority was raised. Completed attempts
and all numerical results follow the bound records.

One namespace ended after admission during model/runtime file verification.
Its only retained preparation file is an empty `model-state/gpu.lock`; no
per-launch model directory, API key, server log, API, upstream request or task
attempt existed. After an exact-owner absence scan, its directory was moved to
a retained preparation path. Its logs and empty lock remain hash-bound.
Interrupted preparation duration is unavailable and excluded from completed-job
totals. This preparation is not a task attempt.

Audit and original-helper reproduction cover all 30 tasks without executing
candidate code. All 15 jobs revoke settings and reap API/model owners; model
keys and sockets are absent and all 30 API tokens are retired. Exact-argument
process scanning covers this PID namespace only, with no global-idle claim.
Sampled total GPU peaks range **17457–18572 MiB**.

The private `thinking-disabled-muse-20261115` review capsule retains frozen
bindings, requests, available response captures, task records, selected source
proposals and verification methods. The read/hash-only verifier extracts nothing
and executes no archived code. It checks every archived file and independently
recomputes requests, strict outcomes, counters and totals. Bindings and the
post-commit receipt identify the exact files and committed blobs. No executable
candidate tree or credential is retained. Reporting fast gate, direct Markdown
lint, actual table rendering and added-content privacy checks pass. All 507
ledger table lines and the previous scoreboard prefix are preserved. Earlier
sealed review artifacts remain unchanged. Model work and helper reproduction
use the build reservation; model work also uses the GPU reservation.

The source gate remains failing with 1,167 passed, 53 inherited failures and
19 ignored, as the retained source review reproduced. This reporting increment
changes no source, prompt, task, threshold, permission or SBOM and claims no
overall-green source gate. All six profiles have fresh post-change development measurements. The next
work follows review findings and the largest measured refusal cause. No held-out run, task-quality claim, row closure, release or
owner readiness follows.

The first report writer stopped while trying to parse the truncated task-27 capture as complete JSON. The failed method and receipt are retained. The corrected reporting method verifies the exact timeout, absent answer and final counters, then reports the unchanged prefix separately. The archive verifier checks its literal bytes and hash and the same refusal conditions; it never repairs the prefix. Audit, original-helper reproduction, cleanup and the six-profile summary were already successful and were preserved rather than rerun. No measurement, candidate, source or threshold changed.

All six profiles now cover 180 development attempts: 117/180 strict ungraded proposals. The largest refusal cause is `strict-edit-refused:code.proposal.format` (27/180). That measured cause sets the next work after high/medium review findings. Task correctness and retries remain unavailable; no held-out task ran.

The retained [independent checkpoint-22 review](../reviews/d1b408016c28-thinking-disabled-muse-checkpoint22.txt) returns **Verdict: PASS** for exact `d1b408016c284e3aaf36e7546f0631ef89a19b0c`, with no findings. It accepted the cumulative22 checkpoint only; this full-repeat increment requires independent core review. No row closes.

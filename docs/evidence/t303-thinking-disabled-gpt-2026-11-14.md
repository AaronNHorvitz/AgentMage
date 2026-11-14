# Thinking-disabled GPT-OSS development repeat — 2026-11-14 UTC

All 30 frozen development tasks ran at source `a896f055256e96baec09966c71c5d0a7f1714d96`:
**11/30 strict ungraded proposals**. First-attempt and within-budget task passes
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
| `generation-refused:error:chat.response.incomplete` | 8 |
| `strict-edit-refused:code.proposal.edit` | 8 |
| `strict-edit-refused:code.proposal.format` | 3 |
| `ungraded-proposal` | 11 |

Request/check time totals **2287.191 seconds**; reserved-work timers total
**2482.685 seconds / 41.378 minutes**. These timers start after frozen-input
validation and include model/runtime verification, loading, calls and cleanup.
Queues, wrapper/pre-entry overhead and pauses are excluded. Active GPU kernel
time and continuous end-to-end elapsed time are unmeasured. Backend counters
cover **30/30**, reporting **188409 input / 431625 output tokens** including
non-answer output. Raw per-task times, tokens and causes are retained. Missing
counters are unavailable, never zero. Backend terminations:
`{"output-cap": 8, "stopped-below-token-limits": 22}`.
Upstream response captures cover **30/30** attempts; missing:
`[]`. Available request, event and backend records remain;
missing bodies or counters are never reconstructed.

## Fixed inputs and changed variable

Changed variable: trusted API `thinking_mode: disabled`, encoded in every
captured upstream request as `chat_template_kwargs: {"enable_thinking": false}`.
The prepared server template retains `reasoning_effort: medium`. This records
what was requested; it does not establish that this model supports or obeys the
thinking extension. Captures show reasoning output in all 30 responses; the
eight output-cap refusals produced no answer text. Complete framing, JSON-object
output, prompts, strict
checks, profiles, 300-second idle/total deadlines, temperature 0, seed 42,
four threads, Q8 caches, batch 256 and microbatch 128 stay fixed.
Context is 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)`, ranging **24219–29748**.
Input ranges **2988–8517**; **does not fit: 0**, separate from refusals.
These are the largest allowances in this prepared profile and transport.

Source freeze SHA-256:
`85dc3c08ba406fc4fae5cc3c49bcad9684bd1fe27c1743c5aa05b0acfea85abd` (2,283 repository and 20 private bindings).
API SHA-256: `ba623b925414e0b390533b18c59b060ad7058c8996d44741420a29b433d05ac1`.
Helper SHA-256: `852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
The development profile is a frozen repository input; its original model hash
and runtime are verified before every model launch. Model SHA-256:
`27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901` (12109566624 bytes).
All source and private bindings pass before every job and in the audit.
No weight download, model role adoption or driver/profile qualification follows.

Erratum: inherited private policy labels `chat_transport_binary_source` and
`comparison` still describe the preceding source and text-to-JSON-object change.
The inherited `collector_policy.changed_variable` also calls the template
arguments default; this profile retains `reasoning_effort: medium`. The actual
API source is `a896f055256e96baec09966c71c5d0a7f1714d96` and the changed variable is the trusted thinking
option, established by source fields, API hash and captured requests.
These labels affect no number. Frozen policy and previous sealed capsules stay
unchanged; label hygiene remains T-309 backlog.

## Verification and review

The existing controller runs 15 separate two-task jobs through both resource
wrappers, with a 900-second execution cap after GPU admission and 30-second
pauses outside reservations. The first controller waited 1,017.196 seconds
for a shared build slot, then checkpointed before GPU entry. No task, model
or candidate job started; this resource wait is separate from model failures.
A second controller waited 559.455 seconds for the build slot and likewise
checkpointed before GPU entry with zero attempts. Both launches, cancellation
receipts and original logs remain retained. Only the builder's own pre-entry
controllers and wrappers were stopped; no other worker was signalled.
Before the completing controller launched, all job and candidate directories
were absent, all frozen bindings and copied profiles passed, and the previous
controller was absent. A bounded observation confirmed the resumed controller's
exact arguments and start ticks. Its only difference is a unique reservation-log
suffix. It refuses partial replay; each task runs once. Queues are waited
normally and no other worker is interrupted.

Audit and original-helper reproduction cover all 30 tasks without executing
candidate code. All 15 jobs revoke settings and reap API/model owners; model
keys and sockets are absent and all 30 API tokens are retired. Exact-argument
process scanning covers this PID namespace only, with no global-idle claim.
Sampled total GPU peaks range **13304–14166 MiB**.

The private `thinking-disabled-gpt-20261114` review capsule retains frozen
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

Initial report rendering failed before writing files because it inferred
response-capture positions from task identifiers. The corrected formatter uses
recorded batch positions and checks complete task coverage. The original failed
log and formatter are retained. Reporting resumes from rendering; the passing
audit, original-helper reproduction and cleanup receipts stay unchanged.

The source gate remains failing with 1,167 passed, 53 inherited failures and
19 ignored, as the retained source review reproduced. This reporting increment
changes no source, prompt, task, threshold, permission or SBOM and claims no
overall-green source gate. One profile still needs a fresh post-change repeat
before tuning. No held-out run, task-quality claim, row closure, release or
owner readiness follows.

# Thinking-disabled Qwen-label development repeat — 2026-11-14 UTC

All 30 frozen development tasks ran at source `a896f055256e96baec09966c71c5d0a7f1714d96`:
**29/30 strict ungraded proposals**. First-attempt and within-budget task passes
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
| `generation-refused:error:chat.transport.timeout` | 1 |
| `ungraded-proposal` | 29 |

Request/check time totals **1427.169 seconds**; reserved-work timers total
**1846.469 seconds / 30.774 minutes**. These timers start after frozen-input
validation and include model/runtime verification, loading, calls and cleanup.
Queues, wrapper/pre-entry overhead and session gaps are excluded. Active GPU
kernel time and continuous end-to-end elapsed time are unmeasured. Backend
counters cover **29/30**, reporting **186402 input / 34920 output tokens**; raw
per-task times, tokens and causes are retained. Token totals include only
attempts with final counters; missing counters are unavailable, never zero. Backend terminations:
`{"stopped-below-token-limits": 29, "unobserved-or-ambiguous": 1}`.
Upstream response captures exist for **29/30** attempts.
Missing captures: `["dev-004"]`. This request timed out before
response-body capture began; no answer or final backend counter was retained.
The recorded timeout, request and available logs are preserved; missing values
are not reconstructed.

## Fixed inputs and changed variable

Changed variable: trusted API `thinking_mode: disabled`, carried as
`chat_template_kwargs: {"enable_thinking": false}` in every captured request.
The prepared Qwen-label launcher already disables thinking in its server template.
The changed variable is the trusted API option and its upstream encoding,
compared with this profile's preceding repeat; server template arguments stay
`{"enable_thinking": false}`. This proves what was requested, without a claim
that the extension is supported by every model.
Complete framing, JSON-object output, prompts, strict checks, profiles,
300-second idle/total deadlines, temperature 0, seed 42, four threads, Q8 caches,
batch 256 and microbatch 128 stay fixed. Context is 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)`, ranging **24137–29804**.
Input ranges **2932–8599**; **does not fit: 0**, separate from refusals.
These are the largest allowances in this prepared profile and transport.

Source freeze SHA-256:
`85dc3c08ba406fc4fae5cc3c49bcad9684bd1fe27c1743c5aa05b0acfea85abd` (2,283 repository and 20 private bindings).
API SHA-256: `ba623b925414e0b390533b18c59b060ad7058c8996d44741420a29b433d05ac1`.
Helper SHA-256: `852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
Actual copied Qwen-label profile equals its frozen original, staged and bound before
launch. Model SHA-256: `5ae89d6720ba0a27f643e6fbb0d1b557870d17aeff23156e5981e863ec16985c`
(16547400032 bytes). All source and private bindings pass before
each job and in the audit. No weights are downloaded or model role adopted.
The driver/profile remains unqualified.

Erratum: inherited private policy labels `chat_transport_binary_source` and
`comparison` still describe the preceding source and text-to-JSON-object change.
For every pair here, actual API source is `a896f055256e96baec09966c71c5d0a7f1714d96` and the changed variable
is the trusted thinking option, established by the source fields, API hash and
captured requests. The inherited `collector_policy.changed_variable` also says
server template arguments remain default; for this profile the prepared template
already uses `enable_thinking: false`. These labels affect no number. Frozen
policy and prior sealed capsules remain unchanged; label hygiene remains T-309
backlog.

## Verification and review

The existing controller runs 15 separate two-task jobs through both resource
wrappers, with a 900-second execution cap after GPU admission and 30-second
pauses outside reservations.
The initial Qwen controller waited for the build slot without starting any job
and did not survive the session. All 15 job directories and the candidate
directory were absent before the first resume controller launched. It completed
tasks 1–20, then waited for a build slot and did not survive the session. All
present job directories were complete; the next five job directories were
absent. A second resume controller completed tasks 21–28, then waited for the
build slot before teardown. The final pair directory was still absent. A third
resume controller skips all completed pairs and finishes tasks 29–30 without replay. The controllers differ only in
unique reservation-log suffixes; old logs and launch receipts are preserved.
All frozen inputs and copied profiles pass before each launch. The final launch's
immediate argument assertion failed for an unestablished reason; a subsequent
exact argument and start-tick check verified the one live queued controller.
No duplicate was launched. Each controller refuses partial replay. Queues are
waited normally; no other worker or queue is interrupted. Audit and
original-helper reproduction cover all 30 tasks without
executing candidate code. All 15 jobs revoke settings and reap API/model owners;
keys and sockets are absent, and all 30 API tokens are retired. Exact-argument
process scanning covers this PID namespace only, with no global-idle claim.
Sampled total GPU peaks range **18990–19864 MiB**.

The private `thinking-disabled-qwen-20261114` review capsule retains all frozen
bindings, actual copied profiles, upstream captures, task records, selected
candidate source files, audit and reproduction methods. Its read/hash-only
verifier extracts nothing and executes no archived code. It checks hashes and
recomputes requests, responses, strict outcomes, counters and totals. Bindings
and the post-commit receipt identify the exact files and committed blobs.
No executable candidate tree or credential is retained.

An initial Markdown gate caught a duplicate blank line; it was corrected
without changing any metric. Reporting fast gate, direct Markdown lint and
actual table rendering pass. After a shared build-slot wait, only the builder's
own pre-entry reporting queue was cancelled. These remaining Markdown/hash
checks ran within unchanged lane limits. Model jobs and original-helper
reproduction ran through the build reservation; model jobs also used the GPU
reservation. No build, test suite or model ran outside its required reservation.
All 507 ledger table lines and the prior scoreboard prefix are preserved;
added-content privacy/scope checks pass. The source gate remains failing with
1,167 passed, 53 inherited failures and 19 ignored, as the retained source review
reproduced. No overall-green source-gate claim is made.

This increment changes no source, prompt, task, threshold, permission or SBOM.
Independent core review remains required. Two profiles still need fresh
post-change measurements before tuning. No held-out run, task-quality claim,
qualification, row closure, release or owner readiness follows.
The local label `qwen35-27b` remains opaque: the retained file hash identifies
only the measured bytes, without a new model identity or provenance claim.

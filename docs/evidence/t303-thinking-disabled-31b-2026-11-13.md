# Thinking-disabled 31B development repeat — 2026-11-13–14 UTC

All 30 frozen development tasks ran at source `a896f055256e96baec09966c71c5d0a7f1714d96`:
**26/30 strict ungraded proposals**. First-attempt and within-budget task passes
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
| `strict-edit-refused:code.proposal.edit` | 3 |
| `strict-edit-refused:code.proposal.format` | 1 |
| `ungraded-proposal` | 26 |

Request/check time totals **1417.912 seconds**; reserved-work timers total
**1980.295 seconds / 33.005 minutes**. These timers start after frozen-input
validation and include model/runtime verification, loading, calls and cleanup.
Queues, wrapper/pre-entry overhead and session gaps are excluded. Active GPU
kernel time and continuous end-to-end elapsed time are unmeasured. Backend
counters cover **30/30**, reporting **215953 input / 31604 output tokens**; raw
per-task times, tokens and causes are retained. Backend terminations:
`{"stopped-below-token-limits": 30}`.

## Fixed inputs and changed variable

Changed variable: trusted API `thinking_mode: disabled`, carried as
`chat_template_kwargs: {"enable_thinking": false}` in every captured request.
The prepared 31B launcher already disables thinking in its server template.
The changed variable is the trusted API option and its upstream encoding,
compared with this profile's preceding repeat; server template arguments stay
`{"enable_thinking": false}`. This proves what was requested, without a claim
that the extension is supported by every model.
Complete framing, JSON-object output, prompts, strict checks, profiles,
300-second idle/total deadlines, temperature 0, seed 42, four threads, Q8 caches,
batch 256 and microbatch 128 stay fixed. Context is 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)`, ranging **23002–29511**.
Input ranges **3225–9734**; **does not fit: 0**, separate from refusals.
These are the largest allowances in this prepared profile and transport.

Source freeze SHA-256:
`85dc3c08ba406fc4fae5cc3c49bcad9684bd1fe27c1743c5aa05b0acfea85abd` (2,283 repository and 20 private bindings).
API SHA-256: `ba623b925414e0b390533b18c59b060ad7058c8996d44741420a29b433d05ac1`.
Helper SHA-256: `852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
Actual copied 31B profile equals its frozen original, staged and bound before
launch. Model SHA-256: `05e0fa1452e73e1b43bdfd378b82b488f57eb63db2bc5e4209f458e08d0542a7`
(17155893600 bytes). All source and private bindings pass before
each job and in the audit. No weights are downloaded or model role adopted.
The driver/profile remains unqualified.

Erratum: inherited private policy labels `chat_transport_binary_source` and
`comparison` still describe the preceding source and text-to-JSON-object change.
For every pair here, actual API source is `a896f055256e96baec09966c71c5d0a7f1714d96` and the changed variable
is the trusted thinking option, established by the source fields, API hash and
captured requests. These labels affect no number. Frozen policy and prior sealed
capsules remain unchanged; label hygiene remains T-309 backlog.

## Verification and review

The existing controller runs 15 separate two-task jobs through both resource
wrappers, with a 900-second execution cap after GPU admission and 30-second
pauses outside reservations.
The first controller finished tasks 1–2, then waited for a build slot; it did
not survive the session. No next job directory existed. Resume changes only the
reservation-log suffix and skips the completed pair. Launch receipts bind exact
arguments and start ticks after all frozen inputs and copied profiles pass.
An immediate post-spawn argument assertion failed; its transient arguments were
not retained. Recovery verified the single existing controller by exact arguments
and start ticks and retained it without relaunch. The controller refuses
partial replay. Queues are waited normally; no other worker or queue is interrupted. Audit and original-helper reproduction cover all 30 tasks without
executing candidate code. All 15 jobs revoke settings and reap API/model owners;
keys and sockets are absent, and all 30 API tokens are retired. Exact-argument
process scanning covers this PID namespace only, with no global-idle claim.
Sampled total GPU peaks range **20716–21757 MiB**.

The private `thinking-disabled-31b-20261113` review capsule retains all frozen
bindings, actual copied profiles, upstream captures, task records, selected
candidate source files, audit and reproduction methods. Its read/hash-only
verifier extracts nothing and executes no archived code. It checks hashes and
recomputes requests, responses, strict outcomes, counters and totals. Bindings
and the post-commit receipt identify the exact files and committed blobs.
No executable candidate tree or credential is retained.

Reporting fast gate, direct Markdown lint and actual table rendering pass.
All 507 ledger table lines and the prior scoreboard prefix are preserved;
added-content privacy/scope checks pass. The source gate remains failing with
1,167 passed, 53 inherited failures and 19 ignored, as the retained source review
reproduced. No overall-green source-gate claim is made.

The retained [E4B reporting review](../reviews/9d6a06392171-thinking-disabled-e4b-measurement.txt)
returns PASS for exact `9d6a063921714933a28f9b3fdb404df954e543c9`, with no high or medium finding.
Its low log-wording and policy-label notes remain T-309 backlog. It reviews
that preceding measurement only. This 31B increment changes no
source, prompt, task, threshold, permission or SBOM and still requires independent
core review. Three profiles still need fresh post-change measurements before
tuning. No held-out run, task-quality claim, qualification, row closure, release
or owner readiness follows.

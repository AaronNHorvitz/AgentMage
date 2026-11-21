# T-303/T-306: unique-anchor one-hundred-fifty-six-attempt checkpoint

Frozen source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Reporting parent: `f33aab477185cc0f3e556ee6a20d6352a7e63f50`.

This increment adds dev-025/dev-026 on all six existing profiles. Cumulative
counts are **156/180 attempted, 24 remaining**. Strict ungraded proposals:
gemma4-31b 22/26; qwen35-27b 20/26; gpt-oss 9/26; muse 17/26; gemma4-e4b 10/26; gemma4-26b-a4b 20/26. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over twenty-six tasks per profile, including
[the unchanged preceding checkpoint](t303-unique-anchor-checkpoint144-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 22/26 | 3072–9639 / 23097–29664 | 178552 / 36267 (26/26) | 1743.559 | 42.121 |
| qwen35-27b | 20/26 | 2811–8502 / 24234–29925 | 157581 / 33191 (26/26) | 1232.307 | 29.038 |
| gpt-oss | 9/26 | 2710–7753 / 24983–30026 | 138930 / 401143 (25/26) | 2512.142 | 47.153 |
| muse | 17/26 | 2625–7787 / 24949–30111 | 107142 / 167996 (20/26) | 5923.726 | 107.743 |
| gemma4-e4b | 10/26 | 3068–9635 / 23101–29668 | 178448 / 86044 (26/26) | 1294.613 | 25.139 |
| gemma4-26b-a4b | 20/26 | 3072–9639 / 23097–29664 | 178552 / 109992 (26/26) | 1811.217 | 41.947 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 22}`; backend terminations `{"stopped-below-token-limits": 26}`; response fields `{"content:nonempty": 26, "reasoning:absent-or-empty": 26, "reasoning_content:absent-or-empty": 26}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 20}`; backend terminations `{"stopped-below-token-limits": 26}`; response fields `{"content:nonempty": 26, "reasoning:absent-or-empty": 26, "reasoning_content:absent-or-empty": 26}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 7, "resource-guard-interrupted:chat.transport.http": 1, "strict-edit-refused:code.proposal.edit": 8, "ungraded-proposal": 9}`; backend terminations `{"output-cap": 8, "resource-guard-interrupted-no-terminal-counters": 1, "stopped-below-token-limits": 17}`; response fields `{"content:absent-or-empty": 7, "content:nonempty": 18, "reasoning:absent-or-empty": 25, "reasoning_content:nonempty": 25}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 17}`; backend terminations `{"stopped-below-token-limits": 20, "transport-timeout-no-terminal-counters": 6}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:nonempty": 20}`. Does not fit: 0.

gemma4-e4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 15, "ungraded-proposal": 10}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 25}`; response fields `{"content:nonempty": 26, "reasoning:absent-or-empty": 26, "reasoning_content:absent-or-empty": 26}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"generation-refused:complete:length": 3, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 20}`; backend terminations `{"output-cap": 3, "stopped-below-token-limits": 23}`; response fields `{"content:nonempty": 26, "reasoning:absent-or-empty": 26, "reasoning_content:absent-or-empty": 26}`. Does not fit: 0.

## Frozen experiment and limitations

The changed variable remains [Decision 0215](../decisions/0215-unique-anchor-generation-guidance.md)'s
fixed generation instruction and its resulting budget-dependent excerpts.
All 30 contexts were frozen before inference; ten excerpts differ from the
preceding line-range campaign. No variable changes in this reporting increment.
Source, parser, strict checker, schema, framing, profiles, model/runtime
identities, temperature, seed, thinking request and deadlines remain frozen.
No submitted answer or line ending is repaired.

The context is 32,768 tokens. Output per task is
`min(32768, 32768 - rendered input tokens - 32)`. The table records actual
rendered fit, separately from preparation byte counts. Thinking remains
requested disabled, which does not establish absence of internal reasoning.
API usage is not requested. Backend terminal counters have explicit coverage;
missing observations remain unavailable. All are first generations with zero
retries. Aggregate causes: `{"generation-refused:complete:length": 5, "generation-refused:error:chat.response.incomplete": 7, "generation-refused:error:chat.transport.timeout": 6, "resource-guard-interrupted:chat.transport.http": 1, "strict-edit-refused:code.proposal.edit": 38, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 98}`.

Only native Rust retrieval, credential-owning chat HTTP transport, local
inference and the strict checker run. The effect-free helper and transport
binary remain those bound by the frozen policy. Original native executable
admission and original grading controls remain unavailable; authority is not
reconstructed. No candidate, project test, hidden or held-out code executes.
There is no test feedback, retry, task-pass, model qualification or role adoption.
Complete candidate projections, including unedited files, are retained as data.
Intermediate counts do not select new work before the full repeat finishes.

The preceding timeout and resource-failure observations remain unchanged and are described in the
preceding checkpoint; no missing counter is inferred. Per-task backend slices
exclude late counters from a previous request.

The twelve new attempts have causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 9}`.
Strict-refusal attempts: qwen35-27b dev-025, gemma4-e4b dev-025, gemma4-e4b dev-026.
Muse request times are 194.985 and
132.746 seconds. New terminal backend coverage is
12/12; cumulative coverage is 149/156. Missing counters remain
unavailable. No submitted answer is repaired or retried.

**Resource gate remains FAIL.** The preceding gpt-oss dev-020 job remains
`failed`, with its exact pinned report and attempt bytes unchanged. Its sampled
22,682 MiB peak exceeded the unchanged 22,528 MiB ceiling. The continuation
validates that exact report using the unchanged `resource_abort_evidence.py`
and skips only that already-counted job; the frozen controller, batch and
collection scripts are preserved. Every other existing report must still be
`completed`. A partial job directory still stops the controller. The only
controller source change is retained as a diff and reproduced by the verifier.
All six new jobs use the frozen batch and collection scripts. No attempt is
relabelled completed, repeated or removed; no guard, limit, model setting or
input changes. The unchanged success-only auditor still fails on the old job;
accounting PASS never means resource PASS. No earlier candidate is modified.

Backend cancellation remains T-309 backlog after the frozen repeat.

## Resources and reproduction

Six additional two-task jobs acquire the build and GPU wrappers independently,
each with a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs
and the existing sampled 22,528 MiB GPU guard. Both reservations release after
each pair, with five 30-second inter-job pauses. This controller wall including
queues and pauses is **1195.033 seconds**; the sum of fourteen successful or
partially attempted controllers is
**19787.510 seconds**, excluding session, recovery and denied-preflight gaps.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The preceding checkpoint's pre-task reservation refusal remains retained
in its original capsule, carried forward byte-for-byte and excluded from summed task-controller wall time.
No new reservation refusal is assumed here; raw wrapper exits are retained.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 156 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding one-hundred-forty-four attempts are byte-checked against their
original checkpoint manifest. The full 1,065-file preparation capsule, consumed
predecessor receipts and all prior 2,390 repository/94 private bindings remain
preserved. The source snapshot contains 9,858 authenticated files. The earlier
million-file measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `deec000a66440cd80b480fb4b3b034690b4c1fecbe4dc62f55afcfb59e66973f`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/f33aab477185-unique-anchor-development-checkpoint144.txt) returns **Verdict: PASS** for exact `f33aab477185cc0f3e556ee6a20d6352a7e63f50`; its report is retained unchanged. No row closes.

Next resume dev-027/dev-028 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows,
including the sampled-GPU-guard observation and reporting-count templating,
remain T-309 backlog. The preceding
review's continuation observation remains addressed by the unchanged exact-failure controller. This checkpoint requires independent core review and
closes no row.
Pre-commit and post-commit verification logs are retained under the shared
review directory.

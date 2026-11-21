# T-303/T-306: unique-anchor one-hundred-eighty-attempt checkpoint

Frozen source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Reporting parent: `93d051951ae1388bbb9b757078ff87d60c52e912`.

This increment adds dev-029/dev-rust-001 on all six existing profiles. Cumulative
counts are **180/180 attempted, 0 remaining**. Strict ungraded proposals:
gemma4-31b 25/30; qwen35-27b 24/30; gpt-oss 10/30; muse 20/30; gemma4-e4b 12/30; gemma4-26b-a4b 23/30. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over thirty tasks per profile, including
[the unchanged preceding checkpoint](t303-unique-anchor-checkpoint168-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 25/30 | 3072–9639 / 23097–29664 | 209405 / 40247 (30/30) | 1949.301 | 47.383 |
| qwen35-27b | 24/30 | 2811–8502 / 24234–29925 | 184740 / 44859 (30/30) | 1554.681 | 35.333 |
| gpt-oss | 10/30 | 2710–7753 / 24983–30026 | 163993 / 499808 (29/30) | 3034.900 | 56.547 |
| muse | 20/30 | 2625–7787 / 24949–30111 | 132147 / 205739 (24/30) | 6834.237 | 124.490 |
| gemma4-e4b | 12/30 | 3068–9635 / 23101–29668 | 209285 / 91244 (30/30) | 1388.073 | 27.065 |
| gemma4-26b-a4b | 23/30 | 3072–9639 / 23097–29664 | 209405 / 112871 (30/30) | 1919.578 | 45.451 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 25}`; backend terminations `{"stopped-below-token-limits": 30}`; response fields `{"content:nonempty": 30, "reasoning:absent-or-empty": 30, "reasoning_content:absent-or-empty": 30}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 24}`; backend terminations `{"stopped-below-token-limits": 30}`; response fields `{"content:nonempty": 30, "reasoning:absent-or-empty": 30, "reasoning_content:absent-or-empty": 30}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 10, "resource-guard-interrupted:chat.transport.http": 1, "strict-edit-refused:code.proposal.edit": 8, "ungraded-proposal": 10}`; backend terminations `{"output-cap": 11, "resource-guard-interrupted-no-terminal-counters": 1, "stopped-below-token-limits": 18}`; response fields `{"content:absent-or-empty": 10, "content:nonempty": 19, "reasoning:absent-or-empty": 29, "reasoning_content:nonempty": 29}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 3, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 20}`; backend terminations `{"stopped-below-token-limits": 24, "transport-timeout-no-terminal-counters": 6}`; response fields `{"content:nonempty": 24, "reasoning:absent-or-empty": 24, "reasoning_content:nonempty": 24}`. Does not fit: 0.

gemma4-e4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 17, "ungraded-proposal": 12}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 29}`; response fields `{"content:nonempty": 30, "reasoning:absent-or-empty": 30, "reasoning_content:absent-or-empty": 30}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"generation-refused:complete:length": 3, "strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 23}`; backend terminations `{"output-cap": 3, "stopped-below-token-limits": 27}`; response fields `{"content:nonempty": 30, "reasoning:absent-or-empty": 30, "reasoning_content:absent-or-empty": 30}`. Does not fit: 0.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 5, "generation-refused:error:chat.response.incomplete": 10, "generation-refused:error:chat.transport.timeout": 6, "resource-guard-interrupted:chat.transport.http": 1, "strict-edit-refused:code.proposal.edit": 43, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 114}`.

Only native Rust retrieval, credential-owning chat HTTP transport, local
inference and the strict checker run. The effect-free helper and transport
binary remain those bound by the frozen policy. Original native executable
admission and original grading controls remain unavailable; authority is not
reconstructed. No candidate, project test, hidden or held-out code executes.
There is no test feedback, retry, task-pass, model qualification or role adoption.
Complete candidate projections, including unedited files, are retained as data.
The full repeat is now recorded. Its measured failure causes determine the next work.

The preceding timeout and resource-failure observations remain unchanged and are described in the
preceding checkpoint; no missing counter is inferred. Per-task backend slices
exclude late counters from a previous request.

The twelve new attempts have causes `{"generation-refused:error:chat.response.incomplete": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 9}`.
Strict-refusal attempts: gemma4-e4b dev-029, gemma4-26b-a4b dev-029.
Muse request times are 155.750 and
244.664 seconds. New terminal backend coverage is
12/12; cumulative coverage is 173/180. Missing counters remain
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
queues and pauses is **1514.593 seconds**; the sum of sixteen successful or
partially attempted controllers is
**22709.349 seconds**, excluding session, recovery and denied-preflight gaps.
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
sockets are absent. The sealed capsule retains all 180 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding one-hundred-sixty-eight attempts are byte-checked against their
original checkpoint manifest. The full 1,065-file preparation capsule, consumed
predecessor receipts and all prior 2,390 repository/94 private bindings remain
preserved. The source snapshot contains 9,858 authenticated files. The earlier
million-file measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `88d45b3a6b76a4069032227bb144557b963a70f53c1ad2a2d7cfef3fa03eb75b`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/93d051951ae1-unique-anchor-development-checkpoint168.txt) returns **Verdict: PASS** for exact `93d051951ae1388bbb9b757078ff87d60c52e912`; its report is retained unchanged. No row closes.

All 30 development tasks have now been attempted on all six profiles. Next,
use the complete measured failure breakdown to choose work under MEASURE FIRST;
no further inference or capability change is part of this reporting batch.

Among measured inference and strict-check failures, the largest cause is `strict-edit-refused:code.proposal.edit` (43 attempts). This selects the next read-only refusal diagnosis under MEASURE FIRST. Native-execution and grading blockers still prevent task outcomes from being measured; they are not resolved by this repeat.

Earlier accepted lows,
including the sampled-GPU-guard observation and reporting-count templating,
remain T-309 backlog. The preceding
review's continuation observation remains addressed by the unchanged exact-failure controller. This checkpoint requires independent core review and
closes no row.
Pre-commit and post-commit verification logs are retained under the shared
review directory.

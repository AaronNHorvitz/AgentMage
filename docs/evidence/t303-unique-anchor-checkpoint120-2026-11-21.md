# T-303/T-306: unique-anchor one-hundred-twenty-attempt checkpoint

Frozen source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Reporting parent: `3b58ea39f2ac9b600a05a1ef0a30ddfd7c99061a`.

This increment adds dev-019/dev-020 on all six existing profiles. Cumulative
counts are **120/180 attempted, 60 remaining**. Strict ungraded proposals:
gemma4-31b 16/20; qwen35-27b 16/20; gpt-oss 7/20; muse 11/20; gemma4-e4b 8/20; gemma4-26b-a4b 15/20. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over twenty tasks per profile, including
[the unchanged preceding checkpoint](t303-unique-anchor-checkpoint108-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 16/20 | 3072–9639 / 23097–29664 | 138991 / 30509 (20/20) | 1433.616 | 34.182 |
| qwen35-27b | 16/20 | 2811–8502 / 24234–29925 | 122805 / 28116 (20/20) | 1000.713 | 23.278 |
| gpt-oss | 7/20 | 2710–7753 / 24983–30026 | 106604 / 323093 (19/20) | 2054.130 | 38.338 |
| muse | 11/20 | 2625–7787 / 24949–30111 | 75047 / 117204 (14/20) | 4687.952 | 84.880 |
| gemma4-e4b | 8/20 | 3068–9635 / 23101–29668 | 138911 / 78866 (20/20) | 1102.769 | 21.100 |
| gemma4-26b-a4b | 15/20 | 3072–9639 / 23097–29664 | 138991 / 75171 (20/20) | 1335.688 | 31.314 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 16}`; backend terminations `{"stopped-below-token-limits": 20}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:absent-or-empty": 20}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 16}`; backend terminations `{"stopped-below-token-limits": 20}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:absent-or-empty": 20}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 6, "resource-guard-interrupted:chat.transport.http": 1, "strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 7}`; backend terminations `{"output-cap": 7, "resource-guard-interrupted-no-terminal-counters": 1, "stopped-below-token-limits": 12}`; response fields `{"content:absent-or-empty": 6, "content:nonempty": 13, "reasoning:absent-or-empty": 19, "reasoning_content:nonempty": 19}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 11}`; backend terminations `{"stopped-below-token-limits": 14, "transport-timeout-no-terminal-counters": 6}`; response fields `{"content:nonempty": 14, "reasoning:absent-or-empty": 14, "reasoning_content:nonempty": 14}`. Does not fit: 0.

gemma4-e4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 11, "ungraded-proposal": 8}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 19}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:absent-or-empty": 20}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"generation-refused:complete:length": 2, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 15}`; backend terminations `{"output-cap": 2, "stopped-below-token-limits": 18}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:absent-or-empty": 20}`. Does not fit: 0.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 4, "generation-refused:error:chat.response.incomplete": 6, "generation-refused:error:chat.transport.timeout": 6, "resource-guard-interrupted:chat.transport.http": 1, "strict-edit-refused:code.proposal.edit": 29, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 73}`.

Only native Rust retrieval, credential-owning chat HTTP transport, local
inference and the strict checker run. The effect-free helper and transport
binary remain those bound by the frozen policy. Original native executable
admission and original grading controls remain unavailable; authority is not
reconstructed. No candidate, project test, hidden or held-out code executes.
There is no test feedback, retry, task-pass, model qualification or role adoption.
Complete candidate projections, including unedited files, are retained as data.
Intermediate counts do not select new work before the full repeat finishes.

The preceding timeout observations remain unchanged and are described in the
preceding checkpoint; no missing counter is inferred. Per-task backend slices
exclude late counters from a previous request.

The twelve new attempts produce nine strict ungraded proposals, two
`code.proposal.edit` refusals (gpt-oss dev-019, gemma4-e4b dev-019) and one
**resource-guard interruption**, counted separately from model failures.
There is no new output cap or transport timeout. Muse completes in
219.747 and 208.555 seconds.
New terminal backend coverage is 11/12; cumulative coverage is
113/120. Missing counters remain unavailable.

**Resource gate: FAIL.** During gpt-oss dev-020, the existing guard records
22,682 MiB sampled total GPU use against its unchanged 22,528 MiB ceiling.
The job remains `failed`, with `GPU desktop headroom exceeded`; the first
controller exits 1. Its second request records `chat.transport.http` after
184.731 seconds, no captured upstream response and no terminal backend counters.
The progress-only `n_gen` samples are not terminal token usage. The retained
candidate is the unchanged base fallback. No answer is repaired or retried,
no profile or reservation limit is raised and no other worker is signalled.
The first request's strict refusal remains counted as recorded.

After settings revocation and owned API/model cleanup were verified, a second
controller runs only the three unattempted profiles with the same frozen
campaign and wrappers. Globally there are 59 completed pairs and one failed
pair, all 120 attempts retained. The original success-only audit is preserved
byte-for-byte and still fails on the failed job. A reporting-only extension
validates this exact failure against pinned report and attempt bytes, counts
it separately, and reproduces all other original checks. An accounting PASS
never means the resource gate passed. Launch inspection, inference scripts,
policy, schema and the existing timeout validator remain unchanged; the
remaining-profile launcher and reporting-audit extension are retained for
independent core review. No earlier attempt or candidate is modified.

Backend cancellation remains T-309 backlog after the frozen repeat.

## Resources and reproduction

Six additional two-task jobs acquire the build and GPU wrappers independently,
each with a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs
and the existing sampled 22,528 MiB GPU guard. Both reservations release after
each pair. The first controller stopped on a resource failure after three jobs;
a second ran only the three unattempted profiles. Four 30-second inter-job
pauses occur across them. Their summed wall time including queues and pauses
is **1600.140 seconds**; the sum of all eleven controllers is
**15674.436 seconds**, excluding session and recovery gaps.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 120 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding one-hundred-eight attempts are byte-checked against their
original checkpoint manifest. The full 1,065-file preparation capsule, consumed
predecessor receipts and all prior 2,390 repository/94 private bindings remain
preserved. The source snapshot contains 9,858 authenticated files. The earlier
million-file measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `dff8df4a3c4b36d1f6bab3286f8f7770a7296a14b7c7092610dcc61c4a6e0ee9`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/3b58ea39f2ac-unique-anchor-development-checkpoint108.txt) returns **Verdict: PASS** for exact `3b58ea39f2ac9b600a05a1ef0a30ddfd7c99061a`; its report is retained unchanged. No row closes.

Next resume dev-021/dev-022 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.
Pre-commit and post-commit verification logs are retained under the shared
review directory.

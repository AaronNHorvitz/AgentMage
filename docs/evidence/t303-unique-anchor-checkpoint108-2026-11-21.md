# T-303/T-306: unique-anchor one-hundred-eight-attempt checkpoint

Frozen source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Reporting parent: `1f167fe8c89b77b1ee8ac98121fedbdcb54f03a2`.

This increment adds dev-017/dev-018 on all six existing profiles. Cumulative
counts are **108/180 attempted, 72 remaining**. Strict ungraded proposals:
gemma4-31b 14/18; qwen35-27b 14/18; gpt-oss 7/18; muse 9/18; gemma4-e4b 7/18; gemma4-26b-a4b 13/18. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over eighteen tasks per profile, including
[the unchanged preceding checkpoint](t303-unique-anchor-checkpoint96-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 14/18 | 3072–9639 / 23097–29664 | 121712 / 29956 (18/18) | 1314.553 | 30.830 |
| qwen35-27b | 14/18 | 2811–8502 / 24234–29925 | 107487 / 27746 (18/18) | 912.163 | 20.473 |
| gpt-oss | 7/18 | 2710–7753 / 24983–30026 | 99241 / 322228 (18/18) | 1841.155 | 33.965 |
| muse | 9/18 | 2625–7787 / 24949–30111 | 61151 / 102981 (12/18) | 4259.650 | 76.658 |
| gemma4-e4b | 7/18 | 3068–9635 / 23101–29668 | 121640 / 76438 (18/18) | 999.748 | 19.007 |
| gemma4-26b-a4b | 13/18 | 3072–9639 / 23097–29664 | 121712 / 74792 (18/18) | 1207.281 | 27.984 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 14}`; backend terminations `{"stopped-below-token-limits": 18}`; response fields `{"content:nonempty": 18, "reasoning:absent-or-empty": 18, "reasoning_content:absent-or-empty": 18}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 14}`; backend terminations `{"stopped-below-token-limits": 18}`; response fields `{"content:nonempty": 18, "reasoning:absent-or-empty": 18, "reasoning_content:absent-or-empty": 18}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 6, "strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 7}`; backend terminations `{"output-cap": 7, "stopped-below-token-limits": 11}`; response fields `{"content:absent-or-empty": 6, "content:nonempty": 12, "reasoning:absent-or-empty": 18, "reasoning_content:nonempty": 18}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 9}`; backend terminations `{"stopped-below-token-limits": 12, "transport-timeout-no-terminal-counters": 6}`; response fields `{"content:nonempty": 12, "reasoning:absent-or-empty": 12, "reasoning_content:nonempty": 12}`. Does not fit: 0.

gemma4-e4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 10, "ungraded-proposal": 7}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 17}`; response fields `{"content:nonempty": 18, "reasoning:absent-or-empty": 18, "reasoning_content:absent-or-empty": 18}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"generation-refused:complete:length": 2, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 13}`; backend terminations `{"output-cap": 2, "stopped-below-token-limits": 16}`; response fields `{"content:nonempty": 18, "reasoning:absent-or-empty": 18, "reasoning_content:absent-or-empty": 18}`. Does not fit: 0.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 4, "generation-refused:error:chat.response.incomplete": 6, "generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 27, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 64}`.

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

The twelve new attempts produce nine strict ungraded proposals and three
`code.proposal.edit` refusals: gpt-oss dev-017, muse dev-017, gemma4-e4b dev-017.
There is no new output-cap termination or transport timeout. Muse dev-017 and
dev-018 complete in 186.128 and 173.297 seconds respectively; the first
is refused by strict edit checking and the second produces an ungraded proposal.
All twelve new attempts have terminal backend counters. No response is repaired
or retried.

Launch, inspection, audit, timeout validation and all campaign scripts remain
byte-identical to the preceding checkpoint. Reporting helpers change only
checkpoint parameters. No bytecode cache is copied or generated; every new
helper runs with `-B`.

The preceding exact-commit independent core review arrived during inference,
was inspected in full and returns PASS with no high or medium finding. Its
unchanged report is retained. Its lows remain T-309 backlog: hand-written
new-attempt prose, per-pair wrapper provenance, incomplete re-verification of
the earlier million-file inventory and backend cancellation on client timeout.
Cumulative table cells and controller times are independently derived by the
portable verifier. No assertion is removed or weakened.

Backend cancellation remains T-309 backlog after the frozen repeat.

## Resources and reproduction

Six additional two-task jobs acquire the build and GPU wrappers independently,
each with a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs
and the existing sampled 22,528 MiB GPU guard. Both reservations release after
each pair, with five 30-second pauses. This controller's wall time including
queues and pauses is **1136.910 seconds**; the sum of the nine controllers is
**14074.296 seconds**, excluding the gaps between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 108 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding ninety-six attempts are byte-checked against their
original checkpoint manifest. The full 1,065-file preparation capsule, consumed
predecessor receipts and all prior 2,390 repository/94 private bindings remain
preserved. The source snapshot contains 9,858 authenticated files. The earlier
million-file measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `9e6053d95fb103de5ecaee8265bec0eb3386bdc572c2650f4978d7c1dad666c6`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/1f167fe8c89b-unique-anchor-development-checkpoint96.txt) returns **Verdict: PASS** for exact `1f167fe8c89b77b1ee8ac98121fedbdcb54f03a2`; its report is retained unchanged. No row closes.

Next resume dev-019/dev-020 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.
Pre-commit and post-commit verification logs are retained under the shared
review directory.

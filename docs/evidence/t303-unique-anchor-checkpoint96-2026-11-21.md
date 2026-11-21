# T-303/T-306: unique-anchor ninety-six-attempt checkpoint

Frozen source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Reporting parent: `01307e6f81033536feb2dc6cdd88407cb10cbb94`.

This increment adds dev-015/dev-016 on all six existing profiles. Cumulative
counts are **96/180 attempted, 84 remaining**. Strict ungraded proposals:
gemma4-31b 12/16; qwen35-27b 12/16; gpt-oss 6/16; muse 8/16; gemma4-e4b 6/16; gemma4-26b-a4b 11/16. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over sixteen tasks per profile, including
[the unchanged preceding checkpoint](t303-unique-anchor-checkpoint84-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 12/16 | 3072–9639 / 23097–29664 | 113994 / 29069 (16/16) | 1244.966 | 28.657 |
| qwen35-27b | 12/16 | 2811–8502 / 24234–29925 | 100589 / 26857 (16/16) | 861.043 | 19.002 |
| gpt-oss | 6/16 | 2710–7753 / 24983–30026 | 92792 / 305960 (16/16) | 1747.166 | 32.071 |
| muse | 8/16 | 2625–7787 / 24949–30111 | 54796 / 87948 (10/16) | 3900.225 | 70.190 |
| gemma4-e4b | 6/16 | 3068–9635 / 23101–29668 | 113930 / 71763 (16/16) | 905.984 | 17.148 |
| gemma4-26b-a4b | 11/16 | 3072–9639 / 23097–29664 | 113994 / 73586 (16/16) | 1131.541 | 25.735 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 12}`; backend terminations `{"stopped-below-token-limits": 16}`; response fields `{"content:nonempty": 16, "reasoning:absent-or-empty": 16, "reasoning_content:absent-or-empty": 16}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 12}`; backend terminations `{"stopped-below-token-limits": 16}`; response fields `{"content:nonempty": 16, "reasoning:absent-or-empty": 16, "reasoning_content:absent-or-empty": 16}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 6, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 6}`; backend terminations `{"output-cap": 7, "stopped-below-token-limits": 9}`; response fields `{"content:absent-or-empty": 6, "content:nonempty": 10, "reasoning:absent-or-empty": 16, "reasoning_content:nonempty": 16}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 1, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 8}`; backend terminations `{"stopped-below-token-limits": 10, "transport-timeout-no-terminal-counters": 6}`; response fields `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:nonempty": 10}`. Does not fit: 0.

gemma4-e4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 9, "ungraded-proposal": 6}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 15}`; response fields `{"content:nonempty": 16, "reasoning:absent-or-empty": 16, "reasoning_content:absent-or-empty": 16}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"generation-refused:complete:length": 2, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 11}`; backend terminations `{"output-cap": 2, "stopped-below-token-limits": 14}`; response fields `{"content:nonempty": 16, "reasoning:absent-or-empty": 16, "reasoning_content:absent-or-empty": 16}`. Does not fit: 0.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 4, "generation-refused:error:chat.response.incomplete": 6, "generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 24, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 55}`.

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

The twelve new attempts produce eight strict ungraded proposals and four
`code.proposal.edit` refusals: gpt-oss dev-015, muse dev-015, gemma4-e4b
dev-016 and gemma4-26b-a4b dev-015. There is no new output-cap termination
or transport timeout. Muse dev-015 and dev-016 complete in 160.078 and
212.851 seconds respectively; the first is refused by strict edit checking,
and the second produces an ungraded proposal. All twelve new attempts have
terminal backend counters. No response is repaired or retried.

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
queues and pauses is **1253.933 seconds**; the sum of the eight controllers is
**12937.386 seconds**, excluding the gaps between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 96 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding eighty-four attempts are byte-checked against their
original checkpoint manifest. The full 1,065-file preparation capsule, consumed
predecessor receipts and all prior 2,390 repository/94 private bindings remain
preserved. The source snapshot contains 9,858 authenticated files. The earlier
million-file measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `f1bd9968bfe707f5c89ba950edc71c3f6719942bcc8c1d652d2ec089d35ac0a0`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/01307e6f8103-unique-anchor-development-checkpoint84.txt) returns **Verdict: PASS** for exact `01307e6f81033536feb2dc6cdd88407cb10cbb94`; its report is retained unchanged. No row closes.

Next resume dev-017/dev-018 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.
Pre-commit and post-commit verification logs are retained under the shared
review directory.

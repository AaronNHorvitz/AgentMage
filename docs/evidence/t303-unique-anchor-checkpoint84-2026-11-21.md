# T-303/T-306: unique-anchor eighty-four-attempt checkpoint

Frozen source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Reporting parent: `220e90a7eb480768a4aa84f339a4224defd2da74`.

This increment adds dev-013/dev-014 on all six existing profiles. Cumulative
counts are **84/180 attempted, 96 remaining**. Strict ungraded proposals:
gemma4-31b 10/14; qwen35-27b 10/14; gpt-oss 5/14; muse 7/14; gemma4-e4b 5/14; gemma4-26b-a4b 10/14. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over fourteen tasks per profile, including
[the unchanged preceding checkpoint](t303-unique-anchor-checkpoint72-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 10/14 | 3072–9639 / 23097–29664 | 95489 / 27291 (14/14) | 1139.202 | 25.407 |
| qwen35-27b | 10/14 | 2811–8444 / 24292–29925 | 83927 / 25163 (14/14) | 786.221 | 17.160 |
| gpt-oss | 5/14 | 2710–7753 / 24983–30026 | 77682 / 278043 (14/14) | 1579.104 | 28.960 |
| muse | 7/14 | 2625–7787 / 24949–30111 | 39859 / 72501 (8/14) | 3527.295 | 63.379 |
| gemma4-e4b | 5/14 | 3068–9635 / 23101–29668 | 95433 / 70930 (14/14) | 863.975 | 16.206 |
| gemma4-26b-a4b | 10/14 | 3072–9639 / 23097–29664 | 95489 / 72479 (14/14) | 1073.223 | 23.847 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 10}`; backend terminations `{"stopped-below-token-limits": 14}`; response fields `{"content:nonempty": 14, "reasoning:absent-or-empty": 14, "reasoning_content:absent-or-empty": 14}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 10}`; backend terminations `{"stopped-below-token-limits": 14}`; response fields `{"content:nonempty": 14, "reasoning:absent-or-empty": 14, "reasoning_content:absent-or-empty": 14}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 6, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 5}`; backend terminations `{"output-cap": 7, "stopped-below-token-limits": 7}`; response fields `{"content:absent-or-empty": 6, "content:nonempty": 8, "reasoning:absent-or-empty": 14, "reasoning_content:nonempty": 14}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 7}`; backend terminations `{"stopped-below-token-limits": 8, "transport-timeout-no-terminal-counters": 6}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:nonempty": 8}`. Does not fit: 0.

gemma4-e4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 8, "ungraded-proposal": 5}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 13}`; response fields `{"content:nonempty": 14, "reasoning:absent-or-empty": 14, "reasoning_content:absent-or-empty": 14}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"generation-refused:complete:length": 2, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 10}`; backend terminations `{"output-cap": 2, "stopped-below-token-limits": 12}`; response fields `{"content:nonempty": 14, "reasoning:absent-or-empty": 14, "reasoning_content:absent-or-empty": 14}`. Does not fit: 0.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 4, "generation-refused:error:chat.response.incomplete": 6, "generation-refused:error:chat.transport.timeout": 6, "strict-edit-refused:code.proposal.edit": 20, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 47}`.

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

The twelve new attempts produce seven strict ungraded proposals, two
`code.proposal.edit` refusals (gemma4-31b dev-014 and gemma4-e4b dev-013), two
output-cap terminations, and one client timeout. GPT-OSS dev-014 and
gemma4-26b-a4b dev-014 end with `complete:length`, using 27,345 and 26,036
output tokens respectively. Each equals its own requested output budget.
Muse dev-013 completes in 206.416 seconds; dev-014 ends with
`chat.transport.timeout` after 300.011 seconds, with no upstream response
captured and no terminal backend counters. Its slice records continued
generation; no missing value is inferred. Eleven new attempts have complete
terminal counters. No response is repaired or retried.

The timeout evidence validator adds only the observed Muse dev-014 case to
its missing-response list. Every validation condition is unchanged. Launch,
inspection, audit and all campaign scripts remain byte-identical to the
preceding checkpoint. No bytecode cache is copied or generated; every new
helper runs with `-B`.

The preceding independent review has no high or medium finding. Its low on
hand-written new-attempt prose remains T-309 backlog, together with per-pair
wrapper provenance, incomplete re-verification of the earlier million-file
inventory and backend cancellation on client timeout. Cumulative table cells
are independently derived by the portable verifier. No assertion is removed
or weakened.

Backend cancellation remains T-309 backlog after the frozen repeat.

## Resources and reproduction

Six additional two-task jobs acquire the build and GPU wrappers independently,
each with a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs
and the existing sampled 22,528 MiB GPU guard. Both reservations release after
each pair, with five 30-second pauses. This controller's wall time including
queues and pauses is **1775.303 seconds**; the sum of the seven controllers is
**11683.453 seconds**, excluding the gaps between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 84 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding seventy-two attempts are byte-checked against their
original checkpoint manifest. The full 1,065-file preparation capsule, consumed
predecessor receipts and all prior 2,390 repository/94 private bindings remain
preserved. The source snapshot contains 9,858 authenticated files. The earlier
million-file measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `3d7804a263746b9e0e6a4d51a9e3657ed7963755505102c7dedc04373c425266`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/220e90a7eb48-unique-anchor-development-checkpoint72.txt) returns **Verdict: PASS** for exact `220e90a7eb480768a4aa84f339a4224defd2da74`; its report is retained unchanged. No row closes.

Next resume dev-015/dev-016 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.
Pre-commit and post-commit verification logs are retained under the shared
review directory.

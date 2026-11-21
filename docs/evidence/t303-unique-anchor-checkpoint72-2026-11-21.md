# T-303/T-306: unique-anchor seventy-two-attempt checkpoint

Frozen source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Reporting parent: `e6d063d2128aa0a2ebb3b3a34c0f6918766b58d8`.

This increment adds dev-011/dev-012 on all six existing profiles. Cumulative
counts are **72/180 attempted, 108 remaining**. Strict ungraded proposals:
gemma4-31b 9/12; qwen35-27b 8/12; gpt-oss 4/12; muse 6/12; gemma4-e4b 4/12; gemma4-26b-a4b 9/12. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over twelve tasks per profile, including
[the unchanged preceding checkpoint](t303-unique-anchor-checkpoint60-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 9/12 | 3072–9639 / 23097–29664 | 79207 / 22112 (12/12) | 957.622 | 21.667 |
| qwen35-27b | 8/12 | 2811–8444 / 24292–29925 | 69671 / 21968 (12/12) | 682.002 | 14.866 |
| gpt-oss | 4/12 | 2710–7753 / 24983–30026 | 64610 / 231219 (12/12) | 1337.292 | 24.651 |
| muse | 6/12 | 2625–7787 / 24949–30111 | 32170 / 63949 (7/12) | 3020.869 | 54.324 |
| gemma4-e4b | 4/12 | 3068–9635 / 23101–29668 | 79159 / 56024 (12/12) | 726.413 | 13.671 |
| gemma4-26b-a4b | 9/12 | 3072–9639 / 23097–29664 | 79207 / 45062 (12/12) | 813.029 | 18.953 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 9}`; backend terminations `{"stopped-below-token-limits": 12}`; response fields `{"content:nonempty": 12, "reasoning:absent-or-empty": 12, "reasoning_content:absent-or-empty": 12}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 8}`; backend terminations `{"stopped-below-token-limits": 12}`; response fields `{"content:nonempty": 12, "reasoning:absent-or-empty": 12, "reasoning_content:absent-or-empty": 12}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:error:chat.response.incomplete": 6, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 4}`; backend terminations `{"output-cap": 6, "stopped-below-token-limits": 6}`; response fields `{"content:absent-or-empty": 6, "content:nonempty": 6, "reasoning:absent-or-empty": 12, "reasoning_content:nonempty": 12}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 5, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 6}`; backend terminations `{"stopped-below-token-limits": 7, "transport-timeout-no-terminal-counters": 5}`; response fields `{"content:nonempty": 7, "reasoning:absent-or-empty": 7, "reasoning_content:nonempty": 7}`. Does not fit: 0.

gemma4-e4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 7, "ungraded-proposal": 4}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 11}`; response fields `{"content:nonempty": 12, "reasoning:absent-or-empty": 12, "reasoning_content:absent-or-empty": 12}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 9}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 11}`; response fields `{"content:nonempty": 12, "reasoning:absent-or-empty": 12, "reasoning_content:absent-or-empty": 12}`. Does not fit: 0.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 2, "generation-refused:error:chat.response.incomplete": 6, "generation-refused:error:chat.transport.timeout": 5, "strict-edit-refused:code.proposal.edit": 18, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 40}`.

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
`code.proposal.edit` refusals (gemma4-31b and qwen35-27b dev-012), and three
output-cap terminations. GPT-OSS dev-011/dev-012 end with
`chat.response.incomplete`, using 27,451/30,009 output tokens respectively;
gemma4-e4b dev-012 ends with `complete:length`, using 29,627 output tokens.
Each equals its own requested output budget. All twelve attempts have complete
backend terminal counters. Both new Muse requests finish before their unchanged
300-second deadline; no new transport timeout occurs. The timeout validator,
launch, inspection and audit scripts remain byte-identical to the preceding
checkpoint. No response is repaired or retried.

The preceding review has no high or medium finding. Its bytecode-cache low is
addressed by copying no cache and running every new helper with `-B`; this work
directory contains no bytecode cache. Its low on hand-written new-attempt prose
remains T-309 backlog, together with the accepted earlier lows. Counts in the
cumulative tables are independently derived by the portable verifier.

Backend cancellation remains T-309 backlog after the frozen repeat.

## Resources and reproduction

Six additional two-task jobs acquire the build and GPU wrappers independently,
each with a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs
and the existing sampled 22,528 MiB GPU guard. Both reservations release after
each pair, with five 30-second pauses. This controller's wall time including
queues and pauses is **2068.944 seconds**; the sum of the six controllers is
**9908.150 seconds**, excluding the gaps between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 72 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding sixty attempts are byte-checked against their
original checkpoint manifest. The full 1,065-file preparation capsule, consumed
predecessor receipts and all prior 2,390 repository/94 private bindings remain
preserved. The source snapshot contains 9,858 authenticated files. The earlier
million-file measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `0dae594f217ce20a72514f66103da0928a71b603280fbf6ee9fea630111d37cd`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/e6d063d2128a-unique-anchor-development-checkpoint60.txt) returns **Verdict: PASS** for exact `e6d063d2128aa0a2ebb3b3a34c0f6918766b58d8`; its report is retained unchanged. No row closes.

Next resume dev-013/dev-014 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.
Pre-commit and post-commit verification logs are retained under the shared
review directory.

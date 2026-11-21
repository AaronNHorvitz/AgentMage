# T-303/T-306: unique-anchor sixty-attempt checkpoint

Frozen source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Reporting parent: `f74d83a84abb6e41a9dab7d1a8265e9f4bb17bc9`.

This increment adds dev-009/dev-010 on all six existing profiles. Cumulative
counts are **60/180 attempted, 120 remaining**. Strict ungraded proposals:
gemma4-31b 8/10; qwen35-27b 7/10; gpt-oss 4/10; muse 4/10; gemma4-e4b 3/10; gemma4-26b-a4b 7/10. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over ten tasks per profile, including
[the unchanged preceding checkpoint](t303-unique-anchor-checkpoint48-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 8/10 | 3072–9639 / 23097–29664 | 69825 / 17457 (10/10) | 759.000 | 17.480 |
| qwen35-27b | 7/10 | 2811–8444 / 24292–29925 | 61165 / 13044 (10/10) | 460.818 | 10.584 |
| gpt-oss | 4/10 | 2710–7753 / 24983–30026 | 56598 / 173759 (10/10) | 1029.787 | 19.194 |
| muse | 4/10 | 2625–7787 / 24949–30111 | 24272 / 40556 (5/10) | 2483.034 | 44.818 |
| gemma4-e4b | 3/10 | 3068–9635 / 23101–29668 | 69785 / 26015 (10/10) | 429.978 | 8.495 |
| gemma4-26b-a4b | 7/10 | 3072–9639 / 23097–29664 | 69825 / 36456 (10/10) | 688.572 | 15.918 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 8}`; backend terminations `{"stopped-below-token-limits": 10}`; response fields `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:absent-or-empty": 10}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 7}`; backend terminations `{"stopped-below-token-limits": 10}`; response fields `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:absent-or-empty": 10}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:error:chat.response.incomplete": 4, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 4}`; backend terminations `{"output-cap": 4, "stopped-below-token-limits": 6}`; response fields `{"content:absent-or-empty": 4, "content:nonempty": 6, "reasoning:absent-or-empty": 10, "reasoning_content:nonempty": 10}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 5, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 4}`; backend terminations `{"stopped-below-token-limits": 5, "transport-timeout-no-terminal-counters": 5}`; response fields `{"content:nonempty": 5, "reasoning:absent-or-empty": 5, "reasoning_content:nonempty": 5}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 7, "ungraded-proposal": 3}`; backend terminations `{"stopped-below-token-limits": 10}`; response fields `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:absent-or-empty": 10}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 7}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 9}`; response fields `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:absent-or-empty": 10}`. Does not fit: 0.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 4, "generation-refused:error:chat.transport.timeout": 5, "strict-edit-refused:code.proposal.edit": 16, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 33}`.

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
`code.proposal.edit` refusals (gemma4-e4b dev-009/dev-010), and one
`chat.transport.timeout` (muse dev-010). Muse dev-009 completes before dev-010
begins. Dev-010 reaches the unchanged 300-second transport deadline with no
upstream reply or terminal backend counter set. Those missing tokens remain
unavailable; no response is parsed or repaired. The effect-free timeout
validator adds only this observed named case to its existing list, with every
validation condition unchanged. All eleven other new attempts have complete
backend terminal counters. No new attempt hits its observed output cap.

Backend cancellation remains T-309 backlog after the frozen repeat.

## Resources and reproduction

Six additional two-task jobs acquire the build and GPU wrappers independently,
each with a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs
and the existing sampled 22,528 MiB GPU guard. Both reservations release after
each pair, with five 30-second pauses. This controller's wall time including
queues and pauses is **1083.384 seconds**; the sum of the five controllers is
**7839.206 seconds**, excluding the gaps between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 60 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding forty-eight attempts are byte-checked against their
original checkpoint manifest. The full 1,065-file preparation capsule, consumed
predecessor receipts and all prior 2,390 repository/94 private bindings remain
preserved. The source snapshot contains 9,858 authenticated files. The earlier
million-file measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `21e43030b7ac44e7dbf4ff82d3c01bf185da4129ca7ffa041d632e8947af7b50`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/f74d83a84abb-unique-anchor-development-checkpoint48.txt) returns **Verdict: PASS** for exact `f74d83a84abb6e41a9dab7d1a8265e9f4bb17bc9`; its report is retained unchanged. No row closes.

Next resume dev-011/dev-012 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.
The preceding review's low log-location finding is addressed by retaining
this checkpoint's pre-commit verification log under the shared review directory.

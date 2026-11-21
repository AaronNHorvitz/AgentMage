# T-303/T-306: original-address sixty-attempt checkpoint

Frozen source: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`. Reporting parent: `6fe83726f54d5b8560489743f32721de4497c4d7`.

This increment adds dev-009/dev-010 on all six existing profiles. Cumulative
counts are **60/180 attempted, 120 remaining**. Strict ungraded proposals:
gemma4-31b 7/10; qwen35-27b 7/10; gpt-oss 3/10; muse 6/10; gemma4-e4b 3/10; gemma4-26b-a4b 7/10. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over ten tasks per profile, including
[the unchanged preceding checkpoint](t303-original-address-checkpoint48-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 7/10 | 3087–9654 / 23082–29649 | 69978 / 13418 (10/10) | 782.895 | 19.993 |
| qwen35-27b | 7/10 | 2825–8458 / 24278–29911 | 61308 / 11917 (10/10) | 483.505 | 11.809 |
| gpt-oss | 3/10 | 2723–7766 / 24970–30013 | 56732 / 165554 (10/10) | 1038.486 | 19.500 |
| muse | 6/10 | 2639–7801 / 24935–30097 | 41277 / 64923 (8/10) | 2246.180 | 43.007 |
| gemma4-e4b | 3/10 | 3083–9650 / 23086–29653 | 69938 / 17345 (10/10) | 578.716 | 11.394 |
| gemma4-26b-a4b | 7/10 | 3087–9654 / 23082–29649 | 69978 / 13002 (10/10) | 594.725 | 15.421 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 7}`; backend terminations `{"stopped-below-token-limits": 10}`; response fields `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:absent-or-empty": 10}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 7}`; backend terminations `{"stopped-below-token-limits": 10}`; response fields `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:absent-or-empty": 10}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 2, "strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 3}`; backend terminations `{"output-cap": 3, "stopped-below-token-limits": 7}`; response fields `{"content:absent-or-empty": 2, "content:nonempty": 8, "reasoning:absent-or-empty": 10, "reasoning_content:nonempty": 10}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 2, "strict-edit-refused:code.proposal.edit": 1, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 6}`; backend terminations `{"stopped-below-token-limits": 8, "transport-timeout-no-terminal-counters": 2}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:nonempty": 8}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 7, "ungraded-proposal": 3}`; backend terminations `{"stopped-below-token-limits": 10}`; response fields `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:absent-or-empty": 10}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 7}`; backend terminations `{"stopped-below-token-limits": 10}`; response fields `{"content:nonempty": 10, "reasoning:absent-or-empty": 10, "reasoning_content:absent-or-empty": 10}`. Does not fit: 0.

Muse dev-007 and dev-008 reached the unchanged 300-second transport deadline.
No upstream response or terminal token counters were captured for either;
both remain failed base-fallback attempts, with zero retries. Missing counters
are unavailable and are not estimated.
The preceding independent review observed about 11 seconds of uncancelled
dev-007 backend generation inside dev-008's transport window, leaving about
289 seconds for its backend processing. Both remain counted failures. Backend
cancellation on client timeout remains T-309 backlog; no timing or inference
variable changes during this repeat.

## Frozen experiment and limitations

The changed variable remains [Decision 0216](../decisions/0216-original-address-insertion-guidance.md)'s
fixed generation instruction and its resulting budget-dependent excerpts.
All 30 contexts were frozen before inference; six excerpts differ from the
preceding unique-anchor campaign. No variable changes in this reporting increment.
Source, parser, strict checker, schema, framing, profiles, model/runtime
identities, temperature, seed, thinking request and deadlines remain frozen.
No submitted answer or line ending is repaired.

The context is 32,768 tokens. Output per task is
`min(32768, 32768 - rendered input tokens - 32)`. The table records actual
rendered fit, separately from preparation byte counts. Thinking remains
requested disabled, which does not establish absence of internal reasoning.
API usage is not requested. Backend terminal counters have explicit coverage;
missing observations remain unavailable. All are first generations with zero
retries. Aggregate causes: `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 2, "generation-refused:error:chat.transport.timeout": 2, "strict-edit-refused:code.proposal.edit": 21, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 33}`.

Only native Rust retrieval, credential-owning chat HTTP transport, local
inference and the strict checker run. The effect-free helper and transport
binary remain those bound by the frozen policy. Original native executable
admission and original grading controls remain unavailable; authority is not
reconstructed. No candidate, project test, hidden or held-out code executes.
There is no test feedback, retry, task-pass, model qualification or role adoption.
Complete candidate projections, including unedited files, are retained as data.
Intermediate counts do not select new work before the full repeat finishes.

## Resources and reproduction

Six additional two-task jobs acquire the build and GPU wrappers independently,
each with a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs
and the existing sampled 22,528 MiB GPU guard. Both reservations release after
each pair, with five 30-second pauses. This controller's wall time including
queues and pauses is **1238.913 seconds**; the sum of the five controllers is
**8243.224 seconds**, excluding the gap between sessions.
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
original checkpoint manifest. The full 1,055-file preparation capsule, consumed
predecessor receipts and all prior 2,425 repository/111 private bindings remain
preserved. The source snapshot contains 9,893 authenticated files. The preceding
measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `5e75d140f8e115bee6e0823bdd7d63cb431761a9b688df2d322770a69c0878b1`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,045 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/6fe83726f54d-original-address-development-checkpoint48.txt) returns **Verdict: PASS** for exact `6fe83726f54d5b8560489743f32721de4497c4d7`; its report is retained unchanged. No row closes.

Existing T-309 backlog retains campaign-specific policy descriptions,
building helpers directly from the frozen commit, review-request wording and
backend cancellation on client timeout.
No production or inference changes are made for these reporting notes.

Next resume dev-011/dev-012 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.

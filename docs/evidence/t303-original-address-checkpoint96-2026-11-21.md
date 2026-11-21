# T-303/T-306: original-address ninety-six-attempt checkpoint

Frozen source: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`. Reporting parent: `9bbb28abfe082a46494a9da10858992e65d5cf2a`.

This increment adds dev-015/dev-016 on all six existing profiles. Cumulative
counts are **96/180 attempted, 84 remaining**. Strict ungraded proposals:
gemma4-31b 12/16; qwen35-27b 11/16; gpt-oss 3/16; muse 9/16; gemma4-e4b 4/16; gemma4-26b-a4b 9/16. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over sixteen tasks per profile, including
[the unchanged preceding checkpoint](t303-original-address-checkpoint84-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 12/16 | 3087–9654 / 23082–29649 | 114237 / 20094 (16/16) | 1142.503 | 28.978 |
| qwen35-27b | 11/16 | 2825–8516 / 24220–29911 | 100816 / 24143 (16/16) | 854.768 | 19.651 |
| gpt-oss | 3/16 | 2723–7766 / 24970–30013 | 93004 / 261039 (16/16) | 1625.935 | 30.379 |
| muse | 9/16 | 2639–7801 / 24935–30097 | 69288 / 104822 (12/16) | 3858.225 | 72.040 |
| gemma4-e4b | 4/16 | 3083–9650 / 23086–29653 | 114173 / 27919 (16/16) | 867.647 | 17.231 |
| gemma4-26b-a4b | 9/16 | 3087–9654 / 23082–29649 | 114237 / 23940 (16/16) | 896.879 | 23.281 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 12}`; backend terminations `{"stopped-below-token-limits": 16}`; response fields `{"content:nonempty": 16, "reasoning:absent-or-empty": 16, "reasoning_content:absent-or-empty": 16}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 11}`; backend terminations `{"stopped-below-token-limits": 16}`; response fields `{"content:nonempty": 16, "reasoning:absent-or-empty": 16, "reasoning_content:absent-or-empty": 16}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 2, "generation-refused:error:chat.response.incomplete": 3, "strict-edit-refused:code.proposal.edit": 8, "ungraded-proposal": 3}`; backend terminations `{"output-cap": 5, "stopped-below-token-limits": 11}`; response fields `{"content:absent-or-empty": 3, "content:nonempty": 13, "reasoning:absent-or-empty": 16, "reasoning_content:nonempty": 16}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 9}`; backend terminations `{"stopped-below-token-limits": 12, "transport-timeout-no-terminal-counters": 4}`; response fields `{"content:nonempty": 12, "reasoning:absent-or-empty": 12, "reasoning_content:nonempty": 12}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 12, "ungraded-proposal": 4}`; backend terminations `{"stopped-below-token-limits": 16}`; response fields `{"content:nonempty": 16, "reasoning:absent-or-empty": 16, "reasoning_content:absent-or-empty": 16}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 7, "ungraded-proposal": 9}`; backend terminations `{"stopped-below-token-limits": 16}`; response fields `{"content:nonempty": 16, "reasoning:absent-or-empty": 16, "reasoning_content:absent-or-empty": 16}`. Does not fit: 0.

This increment: `{"strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 6}`. No new transport timeout occurred; all four earlier timeouts remain counted.

Muse dev-007 and dev-008 reached the unchanged 300-second transport deadline.
No upstream response or terminal token counters were captured for either;
both remain failed base-fallback attempts, with zero retries. Missing counters
are unavailable and are not estimated.
An earlier independent review observed about 11 seconds of uncancelled
dev-007 backend generation inside dev-008's transport window, leaving about
289 seconds for its backend processing. Both remain counted failures. Backend
cancellation on client timeout remains T-309 backlog; no timing or inference
variable changes during this repeat.

Muse dev-011 and dev-012 also reached the same 300-second transport deadline
in an earlier increment, without a captured upstream response or terminal token
counters. Both remain failed base-fallback attempts with zero retries. About
11 seconds of uncancelled dev-011 backend generation overlapped dev-012's
request window, leaving about 289 seconds for its backend processing.
No missing token counters are estimated; cancellation remains T-309 backlog.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 2, "generation-refused:error:chat.response.incomplete": 3, "generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 38, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 48}`.

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
queues and pauses is **1208.952 seconds**; the sum of the eight controllers is
**12993.106 seconds**, excluding the gap between sessions.
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
original checkpoint manifest. The full 1,055-file preparation capsule, consumed
predecessor receipts and all prior 2,425 repository/111 private bindings remain
preserved. The source snapshot contains 9,893 authenticated files. The preceding
measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `e3f7b908dca23bfa59cc0a16316eac40330b3303648a457a72393241c9ab435c`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,045 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/9bbb28abfe08-original-address-development-checkpoint84.txt) returns **Verdict: PASS** for exact `9bbb28abfe082a46494a9da10858992e65d5cf2a`; its report is retained unchanged. No row closes.

Existing T-309 backlog retains campaign-specific policy descriptions,
building helpers directly from the frozen commit, review-request wording and
backend cancellation on client timeout.
No production or inference changes are made for these reporting notes.

Next resume dev-017/dev-018 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.

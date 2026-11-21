# T-303/T-306: original-address one-hundred-twenty-attempt checkpoint

Frozen source: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`. Reporting parent: `53c6655d498f30d3d9bb1afef692c07bb6aec92c`.

This increment adds dev-019/dev-020 on all six existing profiles. Cumulative
counts are **120/180 attempted, 60 remaining**. Strict ungraded proposals:
gemma4-31b 16/20; qwen35-27b 14/20; gpt-oss 6/20; muse 12/20; gemma4-e4b 5/20; gemma4-26b-a4b 11/20. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over twenty tasks per profile, including
[the unchanged preceding checkpoint](t303-original-address-checkpoint108-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 16/20 | 3087–9654 / 23082–29649 | 139176 / 21668 (20/20) | 1311.619 | 34.225 |
| qwen35-27b | 14/20 | 2825–8516 / 24220–29911 | 122981 / 25402 (20/20) | 951.838 | 22.649 |
| gpt-oss | 6/20 | 2723–7766 / 24970–30013 | 113400 / 308392 (20/20) | 1924.019 | 36.219 |
| muse | 12/20 | 2639–7801 / 24935–30097 | 89511 / 134219 (16/20) | 4593.202 | 85.505 |
| gemma4-e4b | 5/20 | 3083–9650 / 23086–29653 | 139096 / 35495 (20/20) | 1036.959 | 20.589 |
| gemma4-26b-a4b | 11/20 | 3087–9654 / 23082–29649 | 139176 / 27955 (20/20) | 1058.637 | 27.943 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 16}`; backend terminations `{"stopped-below-token-limits": 20}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:absent-or-empty": 20}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 14}`; backend terminations `{"stopped-below-token-limits": 20}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:absent-or-empty": 20}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 2, "generation-refused:error:chat.response.incomplete": 3, "strict-edit-refused:code.proposal.edit": 9, "ungraded-proposal": 6}`; backend terminations `{"output-cap": 5, "stopped-below-token-limits": 15}`; response fields `{"content:absent-or-empty": 3, "content:nonempty": 17, "reasoning:absent-or-empty": 20, "reasoning_content:nonempty": 20}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 3, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 12}`; backend terminations `{"stopped-below-token-limits": 16, "transport-timeout-no-terminal-counters": 4}`; response fields `{"content:nonempty": 16, "reasoning:absent-or-empty": 16, "reasoning_content:nonempty": 16}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 15, "ungraded-proposal": 5}`; backend terminations `{"stopped-below-token-limits": 20}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:absent-or-empty": 20}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 9, "ungraded-proposal": 11}`; backend terminations `{"stopped-below-token-limits": 20}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:absent-or-empty": 20}`. Does not fit: 0.

This increment: `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 10}`. No new transport timeout occurred; all four earlier timeouts remain counted.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 2, "generation-refused:error:chat.response.incomplete": 3, "generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 46, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 64}`.

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
queues and pauses is **1113.756 seconds**; the sum of the ten controllers is
**15473.217 seconds**, excluding the gap between sessions.
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
cleanup receipts. The preceding one hundred eight attempts are byte-checked against their
original checkpoint manifest. The full 1,055-file preparation capsule, consumed
predecessor receipts and all prior 2,425 repository/111 private bindings remain
preserved. The source snapshot contains 9,893 authenticated files. The preceding
measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `4095bfd1ac447ec4d5610b38457489933ccdc9634f46d7a067186b662697f7af`.

The reporting verifier preserves the entire preceding scoreboard prefix and
checks this increment's cause summary within its new append. Repeated cause
counts in older entries do not change this check. Every table cell is derived
from the reproduced audit; no observation, threshold or strict check changes.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,045 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/53c6655d498f-original-address-development-checkpoint108.txt) returns **Verdict: PASS** for exact `53c6655d498f30d3d9bb1afef692c07bb6aec92c`; its report is retained unchanged. No row closes.

Existing T-309 backlog retains campaign-specific policy descriptions,
building helpers directly from the frozen commit, review-request wording and
backend cancellation on client timeout.
No production or inference changes are made for these reporting notes.

Next resume dev-021/dev-022 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.

# T-303/T-306: original-address one-hundred-forty-four-attempt checkpoint

Frozen source: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`. Reporting parent: `05d02e4ebb4ae73c9e6e56abe91fa1aaed5eb201`.

This increment adds dev-023/dev-024 on all six existing profiles. Cumulative
counts are **144/180 attempted, 36 remaining**. Strict ungraded proposals:
gemma4-31b 20/24; qwen35-27b 18/24; gpt-oss 7/24; muse 14/24; gemma4-e4b 8/24; gemma4-26b-a4b 14/24. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over twenty-four tasks per profile, including
[the unchanged preceding checkpoint](t303-original-address-checkpoint132-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 20/24 | 3087–9654 / 23082–29649 | 169081 / 24455 (24/24) | 1502.432 | 39.393 |
| qwen35-27b | 18/24 | 2825–8516 / 24220–29911 | 149199 / 28805 (24/24) | 1117.120 | 26.852 |
| gpt-oss | 7/24 | 2723–7766 / 24970–30013 | 137799 / 363236 (24/24) | 2223.487 | 42.010 |
| muse | 14/24 | 2639–7801 / 24935–30097 | 113792 / 165849 (20/24) | 5375.989 | 100.634 |
| gemma4-e4b | 8/24 | 3083–9650 / 23086–29653 | 168985 / 41543 (24/24) | 1188.449 | 23.602 |
| gemma4-26b-a4b | 14/24 | 3087–9654 / 23082–29649 | 169081 / 35129 (24/24) | 1245.337 | 32.898 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 20}`; backend terminations `{"stopped-below-token-limits": 24}`; response fields `{"content:nonempty": 24, "reasoning:absent-or-empty": 24, "reasoning_content:absent-or-empty": 24}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 18}`; backend terminations `{"stopped-below-token-limits": 24}`; response fields `{"content:nonempty": 24, "reasoning:absent-or-empty": 24, "reasoning_content:absent-or-empty": 24}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 2, "generation-refused:error:chat.response.incomplete": 4, "strict-edit-refused:code.proposal.edit": 11, "ungraded-proposal": 7}`; backend terminations `{"output-cap": 6, "stopped-below-token-limits": 18}`; response fields `{"content:absent-or-empty": 4, "content:nonempty": 20, "reasoning:absent-or-empty": 24, "reasoning_content:nonempty": 24}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 5, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 14}`; backend terminations `{"stopped-below-token-limits": 20, "transport-timeout-no-terminal-counters": 4}`; response fields `{"content:nonempty": 20, "reasoning:absent-or-empty": 20, "reasoning_content:nonempty": 20}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 16, "ungraded-proposal": 8}`; backend terminations `{"stopped-below-token-limits": 24}`; response fields `{"content:nonempty": 24, "reasoning:absent-or-empty": 24, "reasoning_content:absent-or-empty": 24}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 10, "ungraded-proposal": 14}`; backend terminations `{"stopped-below-token-limits": 24}`; response fields `{"content:nonempty": 24, "reasoning:absent-or-empty": 24, "reasoning_content:absent-or-empty": 24}`. Does not fit: 0.

This increment: `{"strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 7}`. No new transport timeout occurred; all four earlier timeouts remain counted.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 2, "generation-refused:error:chat.response.incomplete": 4, "generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 52, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 81}`.

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
queues and pauses is **1540.071 seconds**; the sum of the twelve controllers is
**18590.641 seconds**, excluding the gap between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 144 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding one hundred thirty-two attempts are byte-checked against their
original checkpoint manifest. The full 1,055-file preparation capsule, consumed
predecessor receipts and all prior 2,425 repository/111 private bindings remain
preserved. The source snapshot contains 9,893 authenticated files. The preceding
measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `abd8984ef4e19098e02d0426b4a0c9dcb3adbf459db49a6607298cd99adc573c`.

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

The [preceding independent core review](../reviews/05d02e4ebb4a-original-address-development-checkpoint132.txt) returns **Verdict: PASS** for exact `05d02e4ebb4ae73c9e6e56abe91fa1aaed5eb201`; its report is retained unchanged. No row closes.

Existing T-309 backlog retains campaign-specific policy descriptions,
building helpers directly from the frozen commit, review-request wording and
backend cancellation on client timeout.
No production or inference changes are made for these reporting notes.

Next resume dev-025/dev-026 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.

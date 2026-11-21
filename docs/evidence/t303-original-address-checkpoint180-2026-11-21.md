# T-303/T-306: original-address one-hundred-eighty-attempt checkpoint

Frozen source: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`. Reporting parent: `fe0cfc3156903c17a609b9ebed45ff82463af3dc`.

This increment adds dev-029/dev-rust-001 on all six existing profiles. Cumulative
counts are **180/180 attempted, 0 remaining**. Strict ungraded proposals:
gemma4-31b 26/30; qwen35-27b 22/30; gpt-oss 8/30; muse 17/30; gemma4-e4b 11/30; gemma4-26b-a4b 18/30. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over thirty tasks per profile, including
[the unchanged preceding checkpoint](t303-original-address-checkpoint168-2026-11-21.md).

| Profile | Strict ungraded proposals | With joined-line boundary | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- | --- |
| gemma4-31b | 26/30 | 23 | 3087–9654 / 23082–29649 | 209780 / 30360 (30/30) | 1858.156 | 48.745 |
| qwen35-27b | 22/30 | 17 | 2825–8516 / 24220–29911 | 185090 / 33669 (30/30) | 1320.180 | 32.319 |
| gpt-oss | 8/30 | 4 | 2723–7766 / 24970–30013 | 170942 / 481896 (30/30) | 2905.461 | 54.495 |
| muse | 17/30 | 10 | 2639–7801 / 24935–30097 | 146775 / 213473 (26/30) | 6543.012 | 121.912 |
| gemma4-e4b | 11/30 | 4 | 3083–9650 / 23086–29653 | 209660 / 53729 (30/30) | 1480.299 | 29.280 |
| gemma4-26b-a4b | 18/30 | 11 | 3087–9654 / 23082–29649 | 209780 / 43175 (30/30) | 1527.518 | 40.816 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 26}`; backend terminations `{"stopped-below-token-limits": 30}`; response fields `{"content:nonempty": 30, "reasoning:absent-or-empty": 30, "reasoning_content:absent-or-empty": 30}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 8, "ungraded-proposal": 22}`; backend terminations `{"stopped-below-token-limits": 30}`; response fields `{"content:nonempty": 30, "reasoning:absent-or-empty": 30, "reasoning_content:absent-or-empty": 30}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 2, "generation-refused:error:chat.response.incomplete": 7, "strict-edit-refused:code.proposal.edit": 13, "ungraded-proposal": 8}`; backend terminations `{"output-cap": 9, "stopped-below-token-limits": 21}`; response fields `{"content:absent-or-empty": 7, "content:nonempty": 23, "reasoning:absent-or-empty": 30, "reasoning_content:nonempty": 30}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 6, "strict-edit-refused:code.proposal.format": 3, "ungraded-proposal": 17}`; backend terminations `{"stopped-below-token-limits": 26, "transport-timeout-no-terminal-counters": 4}`; response fields `{"content:nonempty": 26, "reasoning:absent-or-empty": 26, "reasoning_content:nonempty": 26}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 19, "ungraded-proposal": 11}`; backend terminations `{"stopped-below-token-limits": 30}`; response fields `{"content:nonempty": 30, "reasoning:absent-or-empty": 30, "reasoning_content:absent-or-empty": 30}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 12, "ungraded-proposal": 18}`; backend terminations `{"stopped-below-token-limits": 30}`; response fields `{"content:nonempty": 30, "reasoning:absent-or-empty": 30, "reasoning_content:absent-or-empty": 30}`. Does not fit: 0.

This increment: `{"generation-refused:error:chat.response.incomplete": 1, "strict-edit-refused:code.proposal.edit": 1, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 9}`. No new transport timeout occurred; all four earlier timeouts remain counted.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 2, "generation-refused:error:chat.response.incomplete": 7, "generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 62, "strict-edit-refused:code.proposal.format": 3, "ungraded-proposal": 102}`.

Among observed inference and strict-check failures, the largest recorded cause is `strict-edit-refused:code.proposal.edit` (62 attempts). This selects the next read-only refusal diagnosis under MEASURE FIRST. Native-execution and grading blockers still prevent task outcomes and are not resolved by this repeat.

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
queues and pauses is **1328.703 seconds**; the sum of the fifteen controllers is
**23074.031 seconds**, excluding the gap between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 180 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding one hundred sixty-eight attempts are byte-checked against their
original checkpoint manifest. The full 1,055-file preparation capsule, consumed
predecessor receipts and all prior 2,425 repository/111 private bindings remain
preserved. The source snapshot contains 9,893 authenticated files. The preceding
measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `d7b7e4e92e6505026869c42ae73ac9395ef55110befbba2bfcd8a419894a4568`.

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

The [preceding independent core review](../reviews/fe0cfc315690-original-address-development-checkpoint168.txt) returns **Verdict: PASS** for exact `fe0cfc3156903c17a609b9ebed45ff82463af3dc`; its report is retained unchanged. No row closes.

Existing T-309 backlog retains campaign-specific policy descriptions,
building helpers directly from the frozen commit, review-request wording and
backend cancellation on client timeout.
The joined-line column answers the checkpoint-156 review's low finding.
It counts accepted proposals with at least one edit that removes a final LF,
replaces it with nonempty text lacking a final LF, and has a nonblank following
original source line. The portable diagnostic reproduces complete accepted
candidate bytes and includes the preserved exact-text client form. It does not
execute candidates or grade syntax, behavior or task correctness. Adjacent
edits and exact-text proposals are counted separately in the raw diagnostic;
this describes original boundaries and does not assume following lines survive
adjacent edits. Twelve synthetic cases cover LF, CRLF, blank following lines,
EOF, deletion, Unicode, exact text and adjacent edits. The checker and all
submitted bytes remain unchanged. Diagnostic SHA-256: `59c3c009fde633825a79855e82e9fb0d7282bea6ef347b1907bead160ef41f64`.
The earlier 60/88 observation remains attributed to checkpoint 156, not this
completed cohort.

No production or inference changes are made for these reporting notes.

All 30 development tasks on all six profiles have been attempted. Next,
reproduce and classify the complete measured failure breakdown before tuning. Other accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.

# T-303/T-306: original-address twenty-four-attempt checkpoint

Frozen source: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`. Reporting parent: `b4318437d35be35a7440417cba5c0ca3492ec644`.

This increment adds dev-003/dev-004 on all six existing profiles. Cumulative
counts are **24/180 attempted, 156 remaining**. Strict ungraded proposals:
gemma4-31b 3/4; qwen35-27b 3/4; gpt-oss 1/4; muse 3/4; gemma4-e4b 1/4; gemma4-26b-a4b 2/4. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over four tasks per profile, including
[the unchanged preceding checkpoint](t303-original-address-checkpoint12-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 3/4 | 3087–9654 / 23082–29649 | 26457 / 6513 (4/4) | 324.777 | 7.181 |
| qwen35-27b | 3/4 | 2825–8458 / 24278–29911 | 23378 / 5634 (4/4) | 194.023 | 4.592 |
| gpt-oss | 1/4 | 2730–7766 / 24970–30006 | 21775 / 60526 (4/4) | 361.992 | 6.892 |
| muse | 3/4 | 2674–7801 / 24935–30062 | 21703 / 33186 (4/4) | 834.742 | 16.731 |
| gemma4-e4b | 1/4 | 3083–9650 / 23086–29653 | 26441 / 8349 (4/4) | 220.349 | 4.275 |
| gemma4-26b-a4b | 2/4 | 3087–9654 / 23082–29649 | 26457 / 6026 (4/4) | 179.795 | 4.874 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 3}`; backend terminations `{"stopped-below-token-limits": 4}`; response fields `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:absent-or-empty": 4}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 3}`; backend terminations `{"stopped-below-token-limits": 4}`; response fields `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:absent-or-empty": 4}`. Does not fit: 0.

gpt-oss: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 4}`; response fields `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:nonempty": 4}`. Does not fit: 0.

muse: causes `{"strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 3}`; backend terminations `{"stopped-below-token-limits": 4}`; response fields `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:nonempty": 4}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 4}`; response fields `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:absent-or-empty": 4}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 4}`; response fields `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:absent-or-empty": 4}`. Does not fit: 0.

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
retries. Aggregate causes: `{"strict-edit-refused:code.proposal.edit": 10, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 13}`.

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
queues and pauses is **1494.731 seconds**; the sum of the two controllers is
**3107.129 seconds**, excluding the gap between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 24 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding twelve attempts are byte-checked against their
original checkpoint manifest. The full 1,055-file preparation capsule, consumed
predecessor receipts and all prior 2,425 repository/111 private bindings remain
preserved. The source snapshot contains 9,893 authenticated files. The preceding
measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `a9712156d4e0950e36d143f3238a25123f82fe1c5dc750579bc37459374e7d5d`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,045 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/b4318437d35b-original-address-development-checkpoint12.txt) returns **Verdict: PASS** for exact `b4318437d35be35a7440417cba5c0ca3492ec644`; its report is retained unchanged. No row closes.

T-309 backlog from the preceding review: generate campaign-specific policy
descriptions instead of retaining stale labels; build helpers directly from
the frozen commit. The latter is an existing backlog item. Neither changes
this frozen repeat.

Next resume dev-005/dev-006 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.

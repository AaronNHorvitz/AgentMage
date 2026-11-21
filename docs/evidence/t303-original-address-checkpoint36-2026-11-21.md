# T-303/T-306: original-address thirty-six-attempt checkpoint

Frozen source: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`. Reporting parent: `adc0aabb2860923adae4bed82e9ff7a566d657ba`.

This increment adds dev-005/dev-006 on all six existing profiles. Cumulative
counts are **36/180 attempted, 144 remaining**. Strict ungraded proposals:
gemma4-31b 4/6; qwen35-27b 4/6; gpt-oss 1/6; muse 4/6; gemma4-e4b 2/6; gemma4-26b-a4b 4/6. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over six tasks per profile, including
[the unchanged preceding checkpoint](t303-original-address-checkpoint24-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 4/6 | 3087–9654 / 23082–29649 | 43670 / 7681 (6/6) | 437.093 | 10.171 |
| qwen35-27b | 4/6 | 2825–8458 / 24278–29911 | 38206 / 6454 (6/6) | 265.913 | 6.631 |
| gpt-oss | 1/6 | 2730–7766 / 24970–30006 | 35232 / 94435 (6/6) | 580.723 | 11.010 |
| muse | 4/6 | 2674–7801 / 24935–30062 | 35085 / 52963 (6/6) | 1327.330 | 25.546 |
| gemma4-e4b | 2/6 | 3083–9650 / 23086–29653 | 43646 / 12111 (6/6) | 325.038 | 6.381 |
| gemma4-26b-a4b | 4/6 | 3087–9654 / 23082–29649 | 43670 / 6895 (6/6) | 262.076 | 7.251 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 4}`; backend terminations `{"stopped-below-token-limits": 6}`; response fields `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:absent-or-empty": 6}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 4}`; backend terminations `{"stopped-below-token-limits": 6}`; response fields `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:absent-or-empty": 6}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:error:chat.response.incomplete": 1, "strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 1}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 5}`; response fields `{"content:absent-or-empty": 1, "content:nonempty": 5, "reasoning:absent-or-empty": 6, "reasoning_content:nonempty": 6}`. Does not fit: 0.

muse: causes `{"strict-edit-refused:code.proposal.edit": 1, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 4}`; backend terminations `{"stopped-below-token-limits": 6}`; response fields `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:nonempty": 6}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 6}`; response fields `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:absent-or-empty": 6}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 4}`; backend terminations `{"stopped-below-token-limits": 6}`; response fields `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:absent-or-empty": 6}`. Does not fit: 0.

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
retries. Aggregate causes: `{"generation-refused:error:chat.response.incomplete": 1, "strict-edit-refused:code.proposal.edit": 15, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 19}`.

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
queues and pauses is **1521.936 seconds**; the sum of the three controllers is
**4629.065 seconds**, excluding the gap between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 36 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding twenty-four attempts are byte-checked against their
original checkpoint manifest. The full 1,055-file preparation capsule, consumed
predecessor receipts and all prior 2,425 repository/111 private bindings remain
preserved. The source snapshot contains 9,893 authenticated files. The preceding
measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `6694e66b97b08cb69b728db15f09f3456cd64e6a1cf26f5efd9d9a5797157338`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,045 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/adc0aabb2860-original-address-development-checkpoint24.txt) returns **Verdict: PASS** for exact `adc0aabb2860923adae4bed82e9ff7a566d657ba`; its report is retained unchanged. No row closes.

T-309 backlog retains campaign-specific policy descriptions and building
helpers directly from the frozen commit. The preceding review adds a low
for review-request wording that names the wrong retained start file. The next
request names the actual retained file. No production or inference changes.

Next resume dev-007/dev-008 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.

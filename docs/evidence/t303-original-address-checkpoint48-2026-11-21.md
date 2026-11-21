# T-303/T-306: original-address forty-eight-attempt checkpoint

Frozen source: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`. Reporting parent: `30afcea0ca3f74dbcd4b17cd3c9c9c6100c1a520`.

This increment adds dev-007/dev-008 on all six existing profiles. Cumulative
counts are **48/180 attempted, 132 remaining**. Strict ungraded proposals:
gemma4-31b 5/8; qwen35-27b 5/8; gpt-oss 1/8; muse 4/8; gemma4-e4b 2/8; gemma4-26b-a4b 5/8. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over eight tasks per profile, including
[the unchanged preceding checkpoint](t303-original-address-checkpoint36-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 5/8 | 3087–9654 / 23082–29649 | 62573 / 13079 (8/8) | 696.568 | 17.302 |
| qwen35-27b | 5/8 | 2825–8458 / 24278–29911 | 54669 / 11191 (8/8) | 423.328 | 10.024 |
| gpt-oss | 1/8 | 2730–7766 / 24970–30006 | 50392 / 144747 (8/8) | 909.192 | 16.902 |
| muse | 4/8 | 2674–7801 / 24935–30062 | 35085 / 52963 (6/8) | 1927.369 | 36.719 |
| gemma4-e4b | 2/8 | 3083–9650 / 23086–29653 | 62541 / 16790 (8/8) | 486.149 | 9.538 |
| gemma4-26b-a4b | 5/8 | 3087–9654 / 23082–29649 | 62573 / 12542 (8/8) | 505.919 | 12.891 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 5}`; backend terminations `{"stopped-below-token-limits": 8}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:absent-or-empty": 8}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 5}`; backend terminations `{"stopped-below-token-limits": 8}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:absent-or-empty": 8}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 2, "strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 1}`; backend terminations `{"output-cap": 3, "stopped-below-token-limits": 5}`; response fields `{"content:absent-or-empty": 2, "content:nonempty": 6, "reasoning:absent-or-empty": 8, "reasoning_content:nonempty": 8}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 2, "strict-edit-refused:code.proposal.edit": 1, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 4}`; backend terminations `{"stopped-below-token-limits": 6, "transport-timeout-no-terminal-counters": 2}`; response fields `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:nonempty": 6}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 8}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:absent-or-empty": 8}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 5}`; backend terminations `{"stopped-below-token-limits": 8}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:absent-or-empty": 8}`. Does not fit: 0.

Muse dev-007 and dev-008 reached the unchanged 300-second transport deadline.
No upstream response or terminal token counters were captured for either;
both remain failed base-fallback attempts, with zero retries. Missing counters
are unavailable and are not estimated.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 2, "generation-refused:error:chat.transport.timeout": 2, "strict-edit-refused:code.proposal.edit": 20, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 22}`.

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
queues and pauses is **2375.246 seconds**; the sum of the four controllers is
**7004.311 seconds**, excluding the gap between sessions.
Table request/check and reserved-job body times are cumulative. Body time
excludes queues and is neither kernel time nor full reservation duration.
Preparation, reporting and complete end-to-end latency remain unmeasured.
The lane's memory.high threshold constrains timings. No limit is raised,
other worker signalled or model downloaded. Launch wrapper bytes and environment
are retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve new
token retirement receipts are persisted before unlinking. Keys, tokens and
sockets are absent. The sealed capsule retains all 48 raw attempts, complete
candidate bytes, wire requests/replies, events, backend counters, contexts and
cleanup receipts. The preceding thirty-six attempts are byte-checked against their
original checkpoint manifest. The full 1,055-file preparation capsule, consumed
predecessor receipts and all prior 2,425 repository/111 private bindings remain
preserved. The source snapshot contains 9,893 authenticated files. The preceding
measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `582f212f2ea7cca792409d9a97ea34e67e8797d7b8a89be2be1098fa846b5f82`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,045 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/30afcea0ca3f-original-address-development-checkpoint36.txt) returns **Verdict: PASS** for exact `30afcea0ca3f74dbcd4b17cd3c9c9c6100c1a520`; its report is retained unchanged. No row closes.

Existing T-309 backlog retains campaign-specific policy descriptions,
building helpers directly from the frozen commit and review-request wording.
No production or inference changes are made for these reporting notes.

Next resume dev-009/dev-010 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.

# T-303/T-306: unique-anchor forty-eight-attempt checkpoint

Frozen source: `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Reporting parent: `371487527d5a19a33cd8904c05441251493ed034`.

This increment adds dev-007/dev-008 on all six existing profiles. Cumulative
counts are **48/180 attempted, 132 remaining**. Strict ungraded proposals:
gemma4-31b 6/8; qwen35-27b 5/8; gpt-oss 2/8; muse 3/8; gemma4-e4b 3/8; gemma4-26b-a4b 5/8. These count accepted edits, not solved tasks. First-attempt and
within-budget task passes remain unavailable. T-303/T-306 stay open.

All table values below are cumulative over eight tasks per profile, including
[the unchanged preceding checkpoint](t303-unique-anchor-checkpoint36-2026-11-21.md).

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 6/8 | 3072–9639 / 23097–29664 | 62450 / 16975 (8/8) | 717.077 | 16.039 |
| qwen35-27b | 5/8 | 2811–8444 / 24292–29925 | 54554 / 12570 (8/8) | 434.941 | 9.786 |
| gpt-oss | 2/8 | 2717–7753 / 24983–30019 | 50284 / 153958 (8/8) | 915.677 | 16.924 |
| muse | 3/8 | 2660–7787 / 24949–30076 | 21647 / 34979 (4/8) | 2040.262 | 36.680 |
| gemma4-e4b | 3/8 | 3068–9635 / 23101–29668 | 62418 / 24274 (8/8) | 378.970 | 7.422 |
| gemma4-26b-a4b | 5/8 | 3072–9639 / 23097–29664 | 62450 / 35737 (8/8) | 644.245 | 14.366 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 6}`; backend terminations `{"stopped-below-token-limits": 8}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:absent-or-empty": 8}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 5}`; backend terminations `{"stopped-below-token-limits": 8}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:absent-or-empty": 8}`. Does not fit: 0.

gpt-oss: causes `{"generation-refused:error:chat.response.incomplete": 4, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 2}`; backend terminations `{"output-cap": 4, "stopped-below-token-limits": 4}`; response fields `{"content:absent-or-empty": 4, "content:nonempty": 4, "reasoning:absent-or-empty": 8, "reasoning_content:nonempty": 8}`. Does not fit: 0.

muse: causes `{"generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 3}`; backend terminations `{"stopped-below-token-limits": 4, "transport-timeout-no-terminal-counters": 4}`; response fields `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:nonempty": 4}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 3}`; backend terminations `{"stopped-below-token-limits": 8}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:absent-or-empty": 8}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 5}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 7}`; response fields `{"content:nonempty": 8, "reasoning:absent-or-empty": 8, "reasoning_content:absent-or-empty": 8}`. Does not fit: 0.

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
retries. Aggregate causes: `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 4, "generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 14, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 24}`.

Only native Rust retrieval, credential-owning chat HTTP transport, local
inference and the strict checker run. The effect-free helper and transport
binary remain those bound by the frozen policy. Original native executable
admission and original grading controls remain unavailable; authority is not
reconstructed. No candidate, project test, hidden or held-out code executes.
There is no test feedback, retry, task-pass, model qualification or role adoption.
Complete candidate projections, including unedited files, are retained as data.
Intermediate counts do not select new work before the full repeat finishes.

The preceding muse dev-005/dev-006 requests reached the unchanged 300-second
transport deadline without an upstream reply or terminal backend counter set.
Those observations remain unchanged; missing tokens remain unavailable.
The preceding dev-005 backend continued about eleven seconds beyond its client
deadline into dev-006's slice. Its late counters are excluded by the per-task
slice protocol. Dev-006's 300-second request includes that wait, leaving about
289 seconds for backend processing before its own timeout. No late dev-005
counter is attributed to dev-006 or used to infer missing tokens.
Both new gpt-oss replies and the new gemma4-26b-a4b dev-008 reply are incomplete
at their observed output caps. Both new muse requests reach the unchanged 300-second transport deadline without an
upstream reply or terminal backend counter set. Dev-007's backend continued
about eleven seconds past its client deadline into dev-008's slice; those late
counters are excluded by the same protocol. Dev-008's request includes that
wait, leaving about 289 seconds for backend processing before its own timeout.
The existing effect-free timeout validator now recognizes only these two
additional named observations; its validation conditions remain unchanged.
No response is parsed, repaired or assigned an inferred token count.
Backend cancellation remains T-309 backlog after the frozen repeat.

## Resources and reproduction

Six additional two-task jobs acquire the build and GPU wrappers independently,
each with a 900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs
and the existing sampled 22,528 MiB GPU guard. Both reservations release after
each pair, with five 30-second pauses. This controller's wall time including
queues and pauses is **2036.988 seconds**; the sum of the four controllers is
**6755.821 seconds**, excluding the gaps between sessions.
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
original checkpoint manifest. The full 1,065-file preparation capsule, consumed
predecessor receipts and all prior 2,390 repository/94 private bindings remain
preserved. The source snapshot contains 9,858 authenticated files. The earlier
million-file measurement inventory is not reverified in full. The portable
verifier reproduces contexts, strict checks, complete candidate scope, wire
requests, counters and cumulative metrics; only the effect-free checker runs.

Audit SHA-256: `05d3796eda995bc50e0e52b496e3fd298cbcc600407ca6adad04c934169cc1ee`.

Reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and capsule reproduction must pass before requesting
review. No repository source changes. The preceding full source gate remains
**FAIL: 1,044 passed, 53 inherited failures, 11 ignored, zero filtered,
31 targets**. It is not rerun for reporting and no failure is waived. Existing
Python and supply-chain limitations remain. All 512 ledger rows, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs remain unchanged. Prior scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/371487527d5a-unique-anchor-development-checkpoint36.txt) returns **Verdict: PASS** for exact `371487527d5a19a33cd8904c05441251493ed034`; its report is retained unchanged. No row closes.

Next resume dev-009/dev-010 on all six profiles. Complete all 30 development
tasks per profile before more tuning or capability work. Earlier accepted lows
remain T-309 backlog. This checkpoint requires independent core review and
closes no row.

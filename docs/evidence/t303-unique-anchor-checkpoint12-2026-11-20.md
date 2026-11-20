# T-303/T-306: unique-anchor twelve-attempt checkpoint

Frozen source and reporting parent: `6ee61e63298e0aca1e1e8896278d24ab72262a61`.

Two development tasks on each of six existing profiles record **12/180
attempted, 168 remaining**. Strict ungraded proposals: gemma4-31b 1/2; qwen35-27b 1/2; gpt-oss 1/2; muse 2/2; gemma4-e4b 0/2; gemma4-26b-a4b 2/2.
These are edit acceptance counts, not solved tasks. First-attempt and
within-budget task passes remain unavailable. No row closes.

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 1/2 | 9218–9639 / 23097–23518 | 18857 / 2164 (2/2) | 146.395 | 3.281 |
| qwen35-27b | 1/2 | 7997–8444 / 24292–24739 | 16441 / 1801 (2/2) | 71.633 | 1.672 |
| gpt-oss | 1/2 | 7375–7753 / 24983–25361 | 15128 / 21081 (2/2) | 113.355 | 2.232 |
| muse | 2/2 | 7370–7787 / 24949–25366 | 15157 / 14086 (2/2) | 342.266 | 6.160 |
| gemma4-e4b | 0/2 | 9214–9635 / 23101–23522 | 18849 / 10993 (2/2) | 174.811 | 3.248 |
| gemma4-26b-a4b | 2/2 | 9218–9639 / 23097–23518 | 18857 / 1738 (2/2) | 111.154 | 3.021 |

gemma4-31b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0.

gpt-oss: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Does not fit: 0.

muse: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0.

## Frozen experiment and its limits

[Decision 0215](../decisions/0215-unique-anchor-generation-guidance.md) changes
the fixed proposal instruction and its budget-dependent excerpts. All 30 new
contexts were frozen before inference; excerpts differ on ten tasks from the
preceding experiment. Parser, checker, framing, schema and permissions stay
unchanged. Answers and line endings are handled literally, with no repair.

Context is 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)` per task. Model/runtime
identities, temperature, seed, thinking request and 300-second transport
deadlines match the preceding repeat. Requested disabled thinking does not
prove absence of internal reasoning. The token fit and budgets above are
actual rendered measurements, separate from the preparation's byte counts.
API usage is not requested. Backend terminal counters are reported with
coverage; absent counters stay unavailable. These are first generations,
with zero retries. Aggregate causes: `{"strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 7}`.

Only native Rust retrieval, the credential-owning chat HTTP transport, local
inference and the strict checker run. The same effect-free helper entry is
rebuilt for the frozen source; the transport binary remains the previously
reviewed build, as recorded in the bound policy. Original native executable
admission and original grading controls remain unavailable. No authority is
reconstructed. No candidate, task-project test, hidden or held-out code executes;
there is no feedback, retry, task-pass, model qualification or role adoption.
Complete candidate projections are retained and checked as data, including
all unedited files.

## Resources, reproduction and next step

Six two-task jobs acquire both host wrappers independently, each with a
900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs and the
existing 22,528 MiB sampled GPU guard. Both reservations release after each
pair, with five 30-second pauses. Controller wall time including queues and
pauses is **1349.351 seconds**. Reserved-job body time excludes queues and
is neither kernel time nor complete reservation duration. Preparation,
reporting and complete end-to-end latency remain unmeasured. The lane's
memory.high threshold constrains timings. No limit is raised, other worker
signalled or model downloaded. Launch wrapper bytes and environment are
retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve token
retirement receipts are persisted before unlinking. Keys, sockets and tokens
are absent. Raw requests/replies, events, backend logs, contexts, candidate
bytes and cleanup receipts are retained in a sealed checkpoint. Every new
inventory entry is authenticated. The entire 1,065-file preparation capsule
is included. All prior 2,355 repository and 78 private binding names are
preserved, now 2,390 and 94 respectively; the read-only source has 9,858 files.
The consumed predecessor receipts are authenticated against its complete
original manifest. This checkpoint does not reverify the earlier million-file
measurement inventory. Its portable verifier reproduces all 30 new contexts,
strict checks, complete candidate projections, requests, counters and published
metrics; only the effect-free checker executes.

Audit SHA-256: `e0440bddbdc2b08d06c30f20bf78b470263a1f64417f87409c69c2906587a344`.

The reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and complete checkpoint reproduction are required before
the review request. No repository source changes occur. The unchanged full
source gate remains **FAIL** from the preceding source change: **1,044 passed,
53 inherited failures, 11 ignored, zero filtered, 31 targets**. It is not rerun
for this reporting increment and no failure is waived. Existing Python and
supply-chain limitations remain. All 512 ledger table lines, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs are preserved. Earlier scoreboard bytes stay an exact
prefix. No SBOM regeneration, long acceptance gate, release, owner-test-ready
or Git push occurs.

The [preceding independent core review](../reviews/6ee61e63298e-unique-anchor-instruction.txt) returns **Verdict: PASS** for exact `6ee61e63298e0aca1e1e8896278d24ab72262a61`. Its report is retained unchanged; no parent row closes. Earlier accepted lows remain T-309 backlog.

Resume the same frozen campaign at dev-003/dev-004 on every profile. Complete
all 30 development tasks on all six profiles before further tuning or
capability work. Intermediate counts select no new work. Independent core
review is required for this checkpoint; T-303 and T-306 remain open.

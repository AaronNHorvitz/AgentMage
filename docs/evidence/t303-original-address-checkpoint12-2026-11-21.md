# T-303/T-306: original-address twelve-attempt checkpoint

Frozen source and reporting parent: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`.

Two development tasks on each of six existing profiles record **12/180
attempted, 168 remaining**. Strict ungraded proposals: gemma4-31b 2/2; qwen35-27b 1/2; gpt-oss 1/2; muse 2/2; gemma4-e4b 1/2; gemma4-26b-a4b 1/2.
These are edit acceptance counts, not solved tasks. First-attempt and
within-budget task passes remain unavailable. No row closes.

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 2/2 | 9233–9654 / 23082–23503 | 18887 / 2016 (2/2) | 126.258 | 3.169 |
| qwen35-27b | 1/2 | 8011–8458 / 24278–24725 | 16469 / 1841 (2/2) | 80.620 | 2.135 |
| gpt-oss | 1/2 | 7388–7766 / 24970–25348 | 15154 / 32561 (2/2) | 198.739 | 3.752 |
| muse | 2/2 | 7384–7801 / 24935–25352 | 15185 / 17704 (2/2) | 449.233 | 9.183 |
| gemma4-e4b | 1/2 | 9229–9650 / 23086–23507 | 18879 / 2700 (2/2) | 103.484 | 2.042 |
| gemma4-26b-a4b | 1/2 | 9233–9654 / 23082–23503 | 18887 / 2565 (2/2) | 80.268 | 2.256 |

gemma4-31b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0.

gpt-oss: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Does not fit: 0.

muse: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Does not fit: 0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0.

## Frozen experiment and its limits

[Decision 0216](../decisions/0216-original-address-insertion-guidance.md) changes
the fixed proposal instruction and its budget-dependent excerpts. All 30 new
contexts were frozen before inference; excerpts differ on six tasks from the
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
with zero retries. Aggregate causes: `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 8}`.

Only native Rust retrieval, the credential-owning chat HTTP transport, local
inference and the strict checker run. The unchanged effect-free helper entry is
rebuilt for the frozen source and reproduces all 30 frozen contexts. The
transport binary remains the previously reviewed build, as recorded in the bound policy. Original native executable
admission and original grading controls remain unavailable. No authority is
reconstructed. No candidate, task-project test, hidden or held-out code executes;
there is no feedback, retry, task-pass, model qualification or role adoption.
Complete candidate projections are retained and checked as data, including
all unedited files. An initial preparation lookup failed before inference;
the unchanged original helper entry was rebuilt, with its source, build log
and successful 30-context preflight retained.

## Resources, reproduction and next step

Six two-task jobs acquire both host wrappers independently, each with a
900-second deadline, low priority, nice 19, 16 GiB RAM, four CPUs and the
existing 22,528 MiB sampled GPU guard. Both reservations release after each
pair, with five 30-second pauses. Controller wall time including queues and
pauses is **1612.399 seconds**. Reserved-job body time excludes queues and
is neither kernel time nor complete reservation duration. Preparation,
reporting and complete end-to-end latency remain unmeasured. The lane's
memory.high threshold constrains timings. No limit is raised, other worker
signalled or model downloaded. Launch wrapper bytes and environment are
retained; per-pair wrapper provenance remains the disclosed T-309 backlog.

All settings are revoked; owned API/model processes are reaped; twelve token
retirement receipts are persisted before unlinking. Keys, sockets and tokens
are absent. Raw requests/replies, events, backend logs, contexts, candidate
bytes and cleanup receipts are retained in a sealed checkpoint. Every new
inventory entry is authenticated. The entire 1,055-file preparation capsule
is included. All prior 2,390 repository and 94 private binding names are
preserved, now 2,425 and 111 respectively; the read-only source has 9,893 files.
The six consumed predecessor profile receipts and preceding frozen-input
record are authenticated against the complete 119,294-entry original manifest.
This checkpoint does not reverify the complete predecessor measurement
inventory. Its portable verifier reproduces all 30 new contexts,
strict checks, complete candidate projections, requests, counters and published
metrics; only the effect-free checker executes.

Audit SHA-256: `3f79975ae4a982786456a4aa55f319fdfa9f335234ca0ddec050dda5538577f8`.

The reporting fast gate, direct Markdown lint/render, review-format, privacy,
licence, preservation and complete checkpoint reproduction are required before
the review request. No repository source changes occur. The unchanged full
source gate remains **FAIL** from the preceding source change: **1,045 passed,
53 inherited failures, 11 ignored, zero filtered, 31 targets**. It is not rerun
for this reporting increment and no failure is waived. Existing Python and
supply-chain limitations remain. All 512 ledger table lines, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs are preserved. Earlier scoreboard bytes stay an exact
prefix. No SBOM regeneration, long acceptance gate, release, owner-test-ready
or Git push occurs.

The [preceding independent core review](../reviews/7821274ab5ca-original-address-instruction.txt) returns **Verdict: PASS** for exact `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`. Its report is retained unchanged; no parent row closes. Existing lows remain T-309 backlog. The review adds two low backlog notes: clarify mixed-form/nonempty-range wording and bind helper builds directly to frozen commits. Neither is changed during this repeat.

Resume the same frozen campaign at dev-003/dev-004 on every profile. Complete
all 30 development tasks on all six profiles before further tuning or
capability work. Intermediate counts select no new work. Independent core
review is required for this checkpoint; T-303 and T-306 remain open.

# T-303/T-306: unique-line twelve-attempt checkpoint

Frozen source and reporting parent: `5ae9ab641fe8d77618309510e58c8a46a8f98ea6`.

Two development tasks on each of six existing profiles record **12/180
attempted, 168 remaining**. Strict ungraded proposals: gemma4-31b 2/2; qwen35-27b 1/2; gpt-oss 1/2; muse 0/2; gemma4-e4b 1/2; gemma4-26b-a4b 1/2.
These are edit acceptance counts, not solved tasks. First-attempt and
within-budget task passes remain unavailable. No row closes.

| Profile | Strict ungraded proposals | Rendered input / requested output tokens | Observed backend input / output tokens (coverage) | Request and strict-check seconds | Reserved-job GPU minutes |
| --- | --- | --- | --- | --- | --- |
| gemma4-31b | 2/2 | 9481–10336 / 22400–23255 | 19817 / 2065 (2/2) | 151.727 | 3.720 |
| qwen35-27b | 1/2 | 8364–9175 / 23561–24372 | 17539 / 1650 (2/2) | 101.326 | 2.941 |
| gpt-oss | 1/2 | 7397–8072 / 24664–25339 | 15469 / 28470 (2/2) | 223.242 | 4.404 |
| muse | 0/2 | 7384–8104 / 24632–25352 | 8104 / 7697 (1/2) | 518.735 | 10.604 |
| gemma4-e4b | 1/2 | 9477–10332 / 22404–23259 | 19809 / 17506 (2/2) | 216.122 | 4.041 |
| gemma4-26b-a4b | 1/2 | 9481–10336 / 22400–23255 | 19817 / 2208 (2/2) | 93.404 | 2.602 |

gemma4-31b: causes `{"ungraded-proposal": 2}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0. Accepted proposals with joined-line boundaries: 1/2.

qwen35-27b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0. Accepted proposals with joined-line boundaries: 0/1.

gpt-oss: causes `{"generation-refused:error:chat.response.incomplete": 1, "ungraded-proposal": 1}`; backend terminations `{"output-cap": 1, "stopped-below-token-limits": 1}`; response fields `{"content:absent-or-empty": 1, "content:nonempty": 1, "reasoning:absent-or-empty": 2, "reasoning_content:nonempty": 2}`. Does not fit: 0. Accepted proposals with joined-line boundaries: 1/1.

muse: causes `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 1}`; backend terminations `{"stopped-below-token-limits": 1, "transport-timeout-no-terminal-counters": 1}`; response fields `{"content:nonempty": 1, "reasoning:absent-or-empty": 1, "reasoning_content:nonempty": 1}`. Does not fit: 0. Accepted proposals with joined-line boundaries: 0/0.

gemma4-e4b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0. Accepted proposals with joined-line boundaries: 0/1.

gemma4-26b-a4b: causes `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`; backend terminations `{"stopped-below-token-limits": 2}`; response fields `{"content:nonempty": 2, "reasoning:absent-or-empty": 2, "reasoning_content:absent-or-empty": 2}`. Does not fit: 0. Accepted proposals with joined-line boundaries: 0/1.

## Frozen experiment and limits

[Decision 0217](../decisions/0217-visible-unique-line-hints.md) changes source
headers to identify unique visible original lines and illustrates replacement
LF explicitly. All 30 contexts were frozen before inference. The preparation
changes excerpts on 11 tasks and makes three unchanged old dev-008 answers
lose visibility; that disclosed coverage cost is not a new model outcome.
Strict checker, schema, retrieval allowance, permissions and limits stay
unchanged. No submitted answer, range or newline is repaired.

Context is 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)` per task. Profile and runtime
identities, temperature, seed, thinking request and 300-second transport
deadlines match the preceding repeat. The table reports actual rendered token
fit, separate from preparation byte counts. Requested disabled thinking does
not prove absence of internal reasoning. API usage is not requested. Backend
terminal counters have explicit coverage; missing observations stay unavailable.
Muse dev-001 reaches the 300-second transport deadline with no captured
upstream response or terminal token counters. Its backend continues into the
next attempt: the retained dev-002 slice shows 6.526 seconds between captured
progress for the preceding generation and dispatch of its own generation.
This is an observed interval, not the complete request delay. Both attempts
remain failures; missing counters are not estimated. Timeout cancellation
remains T-309 backlog, with no deadline or runtime change during this repeat.

These are first generations with zero retries. Aggregate causes:
`{"generation-refused:error:chat.response.incomplete": 1, "generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 6}`.

Only Rust retrieval, credential-owning chat HTTP transport, inference and
strict checking run. The original effect-free helper entry is rebuilt against
the frozen source and reproduces all 30 contexts before inference. Its entry
source is byte-identical to preparation; the transport remains the previously
reviewed binary bound in policy. Original native executable admission and
grading controls remain unavailable; no authority is reconstructed. No
candidate, task-project test, hidden or held-out code executes. No feedback,
retry, task pass, model qualification or role adoption is claimed. Complete
candidate projections, including unedited files, are retained and checked as data.

Joined-line counts beside refusal causes are overlapping textual observations,
not task grades. They count accepted proposals that remove LF before a nonblank
following original line with a nonempty replacement lacking final LF. Complete
accepted candidate bytes reproduce, including exact-text and adjacent edits;
no newline or answer is repaired.

## Resources and reproduction

Six two-task jobs independently acquire both host wrappers with 900-second
deadlines, low priority, nice 19, 16 GiB RAM, four CPUs and the existing
22,528 MiB sampled GPU guard. Both reservations release after every pair,
with five 30-second pauses. Controller wall including queues and pauses is
**2480.892 seconds**. Reserved-job body time excludes queues and is neither
kernel time nor complete reservation duration. Preparation, reporting and
complete end-to-end latency remain unmeasured. The lane's memory.high threshold
constrains timings. No limit is raised, other worker signalled or model downloaded.
Launch wrapper bytes and environment are retained; per-pair wrapper provenance
remains the disclosed T-309 backlog. The frozen policy retains a historical
first-pair A4B note in its descriptive model_selection field; actual launch
order follows its models list, beginning with gemma4-31b. No model is omitted
or selected from intermediate results, and the frozen policy is not rewritten.

Settings are revoked, owned API/model processes reaped, and twelve retirement
receipts persisted before token unlinking. Keys, sockets and tokens are absent.
Raw requests/replies, events, backend logs, contexts, candidate bytes and cleanup
receipts are retained in the sealed checkpoint. Every inventory entry is
verified. The entire 1170-file preparation capsule is included.
All preceding 2425 repository and 111 private binding names are
preserved, now 2460 and 128; the read-only source has 9928 Git blobs.
The six consumed predecessor profile receipts and prior frozen-input record
are authenticated against its original 119284-entry manifest. The complete
predecessor measurement inventory is not reverified. Portable verification
reproduces all 30 contexts, strict checks, complete candidate projections,
wire requests, counters and published metrics. Only effect-free checks execute.

Audit SHA-256: `d669c4cfc26581a17877f4a29c523571cdef1532d8b8816d1596654eb888b492`.

Reporting fast gate, direct Markdown lint/render, review format, privacy,
licence, preservation and complete checkpoint reproduction must pass before
review is requested. No production source changes occur. The preceding full
source gate remains **FAIL**: **1,048 passed, 53 inherited failures, 11 ignored,
zero filtered, 31 targets**. It is retained in preparation/source-gate-final.log;
it is not rerun or waived for this reporting increment. Existing Python and
supply-chain limitations remain. All 512 ledger table lines, 49 historical
path blobs, README, historical inventory, Decision 0051, status model and
supply-chain outputs are preserved. Earlier scoreboard bytes remain an exact
prefix. No SBOM regeneration, long gate, release, owner-test-ready or Git push.

The [preceding independent core review](../reviews/5ae9ab641fe8-visible-unique-line-hints.txt) returns **Verdict: PASS** for exact `5ae9ab641fe8d77618309510e58c8a46a8f98ea6`. The report is retained unchanged. No row closes. Its two low findings join T-309: an intermittent MCP stdin BrokenPipe test failure and bounded repeated whole-file scans when computing unique-line hints. The independent full gate recorded 1,047 passes, 54 failures and 11 ignored tests: the same inherited 53 plus that MCP flake. Five isolated 14/14 reruns passed. This separate observed result is preserved, not relabelled as the recorded builder result of 1,048/53. No high or medium finding is open.

Resume dev-003/dev-004 on every profile with this same frozen source and settings.
Finish all 30 development tasks on all six profiles before further tuning or
capability work. Intermediate counts select no new work. Independent core
review is required for this checkpoint; T-303/T-306 remain open.

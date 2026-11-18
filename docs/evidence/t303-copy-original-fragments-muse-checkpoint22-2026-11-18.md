# T-303/T-306: frozen original-fragment Muse twenty-two-attempt checkpoint

Date: 2026-11-18. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `97e4d0032aa5b96dc822cc7f250b36c8603281d0`. Independent core review required; no row closes.

| Measurement | New four attempts | Cumulative Muse |
| --- | --- | --- |
| Strict ungraded proposals | 3/4 | 15/22 |
| Task passes, first attempt / retries | Unavailable / unavailable | Unavailable / unavailable |
| Request/check seconds | 552.676 | 3931.181 |
| Reserved-job work seconds | 671.456 | 4444.756 |
| Reserved-job GPU minutes | 11.191 | 74.079 |
| Backend input / output tokens | 29714 / 22812 | 131473 / 141820 |
| Counter coverage | 4/4 | 20/22 |

Positions 19–22 (dev-019 through dev-022) add
3/4 strict ungraded proposals; Muse cumulative:
15/22. The repeat is 142/180 complete, with
38 attempts remaining (8 Muse and 30 E4B) before tuning or capability work.
All 138 earlier attempts remain byte-identical and are counted once.

New failure causes and ungraded results: `{"strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 3}`.
Cumulative Muse causes: `{"generation-refused:error:chat.transport.timeout": 2, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 3, "ungraded-proposal": 15}`.
Does not fit in this increment: 0.
New rendered input: 6718–8124 tokens;
output budget: 24612–26018, using
`min(32768, 32768 - rendered input tokens - 32)`.
New backend terminations: `{"stopped-below-token-limits": 4}`.
New response fields: `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:nonempty": 4}`.
The unchanged request sends `chat_template_kwargs.enable_thinking: false`;
response fields do not establish an absence of internal model reasoning.

The changed variable remains the fixed original-fragment instruction from
source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`. This increment changes no source,
helper, context, prompt, schema, model setting, framing, thinking request,
budget or deadline. It does not select a model role.

Partial path only: Rust context selection, the credential-owning chat HTTP API,
local inference and strict edit checking. Native executable admission and
original execution/grading-control receipts remain unavailable; effects,
project tests, feedback and retries cannot run. No candidate, hidden or
held-out test executes. Task passes and within-budget passes stay unavailable.
Proposals are ungraded. T-303/T-306 remain open.

Two two-task jobs acquire both resource wrappers afresh, with 900-second
limits, a 30-second released pause, low priority, nice 19, 16 GiB RAM and
four CPUs. The controller wall interval is 1648.291 seconds including
queues and the released pause, excluding snapshot preparation and reporting.
New request/check time is 552.676 seconds;
reserved-job body time is 671.456 seconds /
11.191 GPU minutes. These exclude queues and are not
GPU kernel time or full reservation duration. End-to-end latency, continuous
cumulative wall time and kernel time are unmeasured. Snapshot preparation
releases its own build slot before the unchanged controller starts.

Before inference, the preceding original sealed verifier authenticates all
sixteen earlier dependency capsules, the preceding increment, all 2,283
repository and 44 private bindings, raw records, live state and complete
committed-diff receipts. This capsule retains all seventeen dependency
references, original reporting methods and recorded adaptations. The complete
9,706-file Git-backed source snapshot remains read-only during inference.
No measured input changes and no previous attempt is replayed.

All 30 contexts and every complete new answer reproduce through the unchanged,
SHA-256-authenticated effect-free Rust helper. Proposed code is retained as
data and never executed. Cleanup checks settings revocation, reaped API/model
processes, absent keys/sockets, four retired API tokens and an empty
owned-process scan. No foreign process is signalled, model downloaded or
external service used. Earlier corrections, incomplete responses and missing
counters remain preserved in their original capsules.

Reporting fast gate, Markdown lint/render, privacy and licence checks,
complete raw audit, bindings, preservation and helper reproduction pass.
Source is unchanged; source tests are not repeated. The last source gate
remains FAIL: 1,030 passed, 53 inherited failures and 11 ignored; two inherited
Python failures and supply-chain limits remain disclosed. All 512 ledger table
lines, 49 historical private-path blobs, inventory, Decision 0051, status fields,
frozen inputs and previous scoreboard bytes are preserved. No SBOM regeneration,
full acceptance, promotion or release runs. Existing T-309 backlog stays open.
The preceding exact-commit review was pending at session start. Subsequent
independent verdicts are retained separately; no verdict is inferred for this
increment. Only the independent reviewer accepts.

A pre-inference reporting adaptation stopped on an exact-match assertion.
Its denominator substitution was corrected before launch; the original
method, partial outputs and correction record are retained. No measured
input, model attempt or resource setting changed.

The preceding exact-commit Muse checkpoint18 review returned PASS with no
high, medium or new low finding. Its report is retained unchanged; it accepts
only the preceding measurement and closes no row. Existing T-309 backlog
remains open. No verdict is inferred for this increment.

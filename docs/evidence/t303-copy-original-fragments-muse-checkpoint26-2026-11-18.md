# T-303/T-306: frozen original-fragment Muse twenty-six-attempt checkpoint

Date: 2026-11-18. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `0ffdabc25b77b7552795bb135f9af9fdbb2cb167`. Independent core review required; no row closes.

| Measurement | New four attempts | Cumulative Muse |
| --- | --- | --- |
| Strict ungraded proposals | 4/4 | 19/26 |
| Task passes, first attempt / retries | Unavailable / unavailable | Unavailable / unavailable |
| Request/check seconds | 439.760 | 4370.941 |
| Reserved-job work seconds | 491.148 | 4935.904 |
| Reserved-job GPU minutes | 8.186 | 82.265 |
| Backend input / output tokens | 21547 / 18826 | 153020 / 160646 |
| Counter coverage | 4/4 | 24/26 |

Positions 23–26 (dev-023 through dev-026) add
4/4 strict ungraded proposals; Muse cumulative:
19/26. The repeat is 146/180 complete, with
34 attempts remaining (4 Muse and 30 E4B) before tuning or capability work.
All 142 earlier attempts remain byte-identical and are counted once.

New failure causes and ungraded results: `{"ungraded-proposal": 4}`.
Cumulative Muse causes: `{"generation-refused:error:chat.transport.timeout": 2, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 3, "ungraded-proposal": 19}`.
Does not fit in this increment: 0.
New rendered input: 4112–7622 tokens;
output budget: 25114–28624, using
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
four CPUs. The controller wall interval is 524.289 seconds including
queues and the released pause, excluding snapshot preparation and reporting.
New request/check time is 439.760 seconds;
reserved-job body time is 491.148 seconds /
8.186 GPU minutes. These exclude queues and are not
GPU kernel time or full reservation duration. End-to-end latency, continuous
cumulative wall time and kernel time are unmeasured. Snapshot preparation
releases its own build slot before the unchanged controller starts.

Before inference, the preceding original sealed verifier authenticates all
seventeen earlier dependency capsules, the preceding increment, all 2,283
repository and 44 private bindings, raw records, live state and complete
committed-diff receipts. This capsule retains all eighteen dependency
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

The preceding exact-commit Muse checkpoint22 review returned PASS with no
high, medium or new low finding. Its report is retained unchanged; it accepts
only the preceding measurement and closes no row. Existing T-309 backlog
remains open. No verdict is inferred for this increment.

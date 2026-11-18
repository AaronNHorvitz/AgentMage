# T-303/T-306: frozen original-fragment Muse ten-attempt checkpoint

Date: 2026-11-18. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `c6c7ca5802b800b1cec21d6fac2164e29a7e4cb2`. Independent core review required; no row closes.

| Measurement | Ten Muse attempts |
| --- | --- |
| Strict ungraded proposals | 5/10 |
| Task passes, first attempt / retries | Unavailable / unavailable |
| Request/check seconds | 1800.270 |
| Reserved-job work seconds | 2079.291 |
| Reserved-job GPU minutes | 34.655 |
| Backend input / output tokens | 58324 / 63045 |
| Counter coverage | 9/10 |

Positions 1–10 (dev-001 through dev-010) produce
5/10 strict ungraded proposals.
Failure causes and ungraded results: `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 2, "ungraded-proposal": 5}`.
Does not fit: 0.
Rendered input: 2913–8327 tokens.
Output budget: 24409–29823, using
`min(32768, 32768 - rendered input tokens - 32)`.
Backend terminations: `{"stopped-below-token-limits": 9, "unobserved-or-ambiguous": 1}`.
Response fields: `{"content:nonempty": 9, "reasoning:absent-or-empty": 9, "reasoning_content:nonempty": 9}`.
The request sends `chat_template_kwargs.enable_thinking: false`; observed
response fields are recorded separately and do not establish an absence of
internal model reasoning.

The changed variable remains the fixed original-fragment instruction. This
checkpoint changes no source, helper, context, prompt, schema, model setting,
framing, thinking request, budget or deadline. All 120 previous attempts remain
byte-identical and are counted once. The six-profile repeat is 130/180 complete;
50 attempts remain (20 Muse and 30 E4B) before tuning or capability work.

Partial path only: Rust context selection, the credential-owning chat HTTP API,
local inference and strict edit checking. Native executable admission and
original execution/grading-control receipts remain unavailable; effects,
project tests, feedback and retries cannot run. No candidate, hidden or
held-out test executes. These proposals are ungraded, not solved tasks.
T-303/T-306 remain open and no model role qualifies.

Five two-task jobs use both resource wrappers, 900-second limits, 30-second
released pauses, low priority, nice 19, 16 GiB RAM and four CPUs. The controller
wall interval is 5972.158 seconds including queues and released pauses,
excluding snapshot preparation and reporting. Request/check time is
1800.270 seconds. Reserved-job body time is
2079.291 seconds / 34.655 GPU minutes;
it excludes queues and is not GPU kernel time or full reservation duration.
End-to-end latency, continuous cumulative wall time and kernel time are
unmeasured. Snapshot preparation alone takes a build slot; it releases that
slot before the controller starts. Each pair acquires both wrappers afresh.

The preceding sealed verifier authenticates all thirteen earlier dependency
capsules, the preceding increment, all 2,283 repository and 44 private bindings,
raw records, live state and complete committed-diff receipts before inference.
The new capsule retains all fourteen dependency references, every original
reporting method and adaptation. The complete 9,706-file Git-backed source
snapshot remains read-only during inference. No measured input is changed.

All 30 contexts and every complete new answer reproduce through the unchanged,
SHA-256-authenticated effect-free Rust helper. Proposed code is retained as data
and never executed. Cleanup checks settings revocation, reaped API/model
processes, absent keys/sockets, ten retired API tokens and an empty owned-process
scan. No foreign process is signalled, model downloaded or external service used.
Earlier corrections, incomplete responses and missing counters remain preserved
in their original capsules; none is reconstructed or dropped.

Reporting fast gate, Markdown lint/render, privacy and licence checks,
complete raw audit, all bindings, preservation and helper reproduction pass.
Source is unchanged; source tests are not repeated. The last source gate
remains FAIL: 1,030 passed, 53 inherited failures and 11 ignored; two inherited
Python failures and supply-chain limits remain disclosed. All 512 ledger table
lines, 49 historical private-path blobs, inventory, Decision 0051, status fields,
frozen inputs and previous scoreboard bytes are preserved. No SBOM regeneration,
full acceptance, promotion or release runs. Existing T-309 backlog stays open.
The preceding commit has an independent PASS with no new findings; its report
is retained unchanged. It accepts that measurement only and closes no row.
No verdict is inferred for this increment.

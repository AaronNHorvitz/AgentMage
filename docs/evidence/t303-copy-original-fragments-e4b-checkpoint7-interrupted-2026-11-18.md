# T-303/T-306: frozen original-fragment E4B seven-attempt interrupted checkpoint

Date: 2026-11-18. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `b1ea1bb5e8317e0b2c0a59149492b16e119ee214`. Independent core review required; no row closes.

| Measurement | E4B positions 1–7 |
| --- | --- |
| Strict ungraded proposals | 3/7 |
| Task passes, first attempt / retries | Unavailable / unavailable |
| Request/check seconds | 562.374 |
| Reserved-job work seconds | 604.467 |
| Reserved-job GPU minutes | 10.074 |
| Backend input / output tokens | 44823 / 48981 |
| Counter coverage | 6/7 |

E4B positions 1–7 (dev-001 through dev-007) record
3/7 strict ungraded proposals. This repeat is
157/180 attempted (including one resource interruption), with 23 E4B attempts remaining before further tuning or
capability work. Every earlier attempt and measured setting is preserved.

Three jobs completed six attempts. During task dev-007 in the fourth job,
the unchanged GPU guard observed 23,868 MiB of total VRAM use against its
22,528 MiB limit and terminated the owned model. The API recorded
`chat.transport.http`; the next task stopped at context construction with
connection refused. The controller exited 1. This is a resource interruption,
reported separately from model failures. The original failed job and all
partial bytes are preserved; no batch summary is fabricated and no attempt
is replayed. dev-008 has only a context file and never reached inference.
No attribution is made for the source of the total GPU-memory increase.
The sampled peak is an over-limit observation, not successful resource admission.

Failure causes and ungraded results: `{"generation-refused:complete:length": 1, "resource-interrupted:gpu-headroom": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 3}`.
Does not fit: 0.
Rendered input: 3225–9734 tokens;
output budget: 23002–29511, using
`min(32768, 32768 - rendered input tokens - 32)`.
The interrupted task has no complete backend token counters; the token totals
below cover only the observed completed responses, with coverage stated.
Backend terminations: `{"output-cap": 1, "resource-interrupted-counters-unavailable": 1, "stopped-below-token-limits": 5}`.
Response fields: `{"content:nonempty": 6, "reasoning:absent-or-empty": 6, "reasoning_content:absent-or-empty": 6}`.
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

Four two-task jobs acquire both resource wrappers afresh, with 900-second
limits, 30-second released pauses, low priority, nice 19, 16 GiB RAM and
four CPUs. Controller wall time is 730.513 seconds including queues
and released pauses, excluding snapshot preparation and reporting.
Request/check time is 562.374 seconds;
reserved-job body time is 604.467 seconds /
10.074 GPU minutes. These exclude queues and are not
GPU kernel time or full reservation duration. End-to-end latency, continuous
cumulative wall time and kernel time are unmeasured. Snapshot preparation
releases its own build slot before the unchanged controller starts.

Before inference, the preceding original sealed verifier authenticates all
nineteen earlier dependency capsules, the preceding increment, all 2,283
repository and 44 private bindings, raw records, live state and complete
committed-diff receipts. This capsule retains all twenty dependency references,
original reporting methods and recorded adaptations. The complete 9,706-file
Git-backed source snapshot remains read-only during inference. No measured
input changes and no previous attempt is replayed.

All 30 contexts and every complete new answer reproduce through the unchanged,
SHA-256-authenticated effect-free Rust helper. Proposed code is retained as
data and never executed. Cleanup checks settings revocation, reaped API/model
processes, absent keys/sockets, eight retired API tokens and an empty
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
The preceding exact-commit review was pending at session start; any subsequently
observed verdict is retained separately. No verdict is inferred for this
increment. Only the independent reviewer accepts.

The preceding exact-commit Muse completion review subsequently returned PASS
with no high or medium finding. Its report is retained unchanged and accepts
only that prior measurement. No row closes; existing T-309 backlog remains.

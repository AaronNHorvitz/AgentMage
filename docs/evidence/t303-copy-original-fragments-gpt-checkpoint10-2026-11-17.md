# T-303/T-306: ten frozen original-fragment gpt-oss development attempts

Date: 2026-11-17. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `b37e498d9aa416948f18b0f3da92c0fb50a700f8`. Independent core review required; no row closes.

| Measurement | Ten new gpt-oss attempts |
| --- | --- |
| Strict ungraded proposals | 3/10 |
| Task passes, first attempt / retries | Unavailable / unavailable |
| Request/check seconds | 610.865 |
| Reserved-job work seconds | 674.878 |
| Reserved-job GPU minutes | 11.248 |
| Backend input / output tokens | 61825 / 122784 |
| Counter coverage | 10/10 |

Ten new attempts (positions 1–10: dev-001 through dev-010):
3/10 strict ungraded proposals;
610.865 request/check seconds,
674.878 reserved-job work seconds /
11.248 reserved-job GPU minutes;
61825 input / 122784 output tokens,
with backend counters for 10/10 attempts.
Causes: `{"generation-refused:error:chat.response.incomplete": 2, "strict-edit-refused:code.proposal.edit": 5, "ungraded-proposal": 3}`.
Does not fit: 0.
Rendered input: 2992–8361 tokens.
Output budget: 24375–29744, using
`min(32768, 32768 - rendered input tokens - 32)`.
Backend terminations: `{"output-cap": 2, "stopped-below-token-limits": 8}`.

The changed variable remains the fixed original-fragment instruction. This
increment changes no source, helper, context, prompt, schema, model setting,
framing, thinking request, budget or deadline. All ninety earlier attempts
remain byte-identical and are counted once. The six-profile repeat is 100/180
complete; 80 attempts remain before tuning or capability work.

Partial path only: Rust context selection, the credential-owning chat HTTP API,
local inference and strict edit checking. Native executable admission and
original execution/grading-control receipts remain unavailable; effects,
project tests, feedback and retries cannot run. No candidate, hidden or
held-out test executes. These proposals are ungraded, not solved tasks.
T-303/T-306 remain open and no model role qualifies.

Five two-task jobs use both resource wrappers, 900-second limits, 30-second
released pauses, low priority, nice 19, 16 GiB RAM and four CPUs. The new
controller wall window is 1052.501 seconds, including queues and released
pauses, excluding preparation and reporting. GPU kernel time, full reservation
duration, continuous cumulative wall time and end-to-end latency are unmeasured.

All 2,283 repository and 44 private bindings, eleven dependency capsules and
their complete committed-diff receipts authenticate before inference. The
read-only source snapshot retains 9,706 files. No input or predicate is removed.
All 30 contexts and every retained complete new answer check reproduce with
the unchanged SHA-256-authenticated, effect-free Rust helper. No model inference
or candidate execution occurs during reproduction.

Every job revokes settings, reaps API/model processes and removes keys and
sockets. Ten new API tokens are retired; the exact owned-process scan is empty.
No foreign process is signalled, model downloaded or external service used.
Raw jobs, retained candidates without execution, all reporting methods,
original methods and adaptation hashes are sealed in the review capsule.
The earlier 27B timeout remains recorded with its missing response/counters;
none is invented. Previous capsules preserve their complete correction chains.
The independent 27B completion review arrived during this session and returns
PASS for exact `b37e498d9aa416948f18b0f3da92c0fb50a700f8`, with no new findings.
Its report is retained unchanged and accepts only that measurement, closing
no row. No verdict is inferred for this increment. Existing T-309 backlog
stays open.

Reporting fast gate, Markdown lint/render, privacy and licence checks,
complete raw audit, all bindings, preservation and helper reproduction pass.
Source is unchanged; source tests are not repeated. The last source gate
remains FAIL: 1,030 passed, 53 inherited failures and 11 ignored; two inherited
Python failures and supply-chain limits remain disclosed. All 512 ledger table
lines, 49 historical private-path blobs, inventory, Decision 0051, status fields,
frozen inputs and previous scoreboard bytes are preserved. No SBOM regeneration,
full acceptance, promotion or release runs. T-303/T-306 remain open.

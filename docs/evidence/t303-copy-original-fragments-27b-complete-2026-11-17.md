# T-303/T-306: completed frozen original-fragment 27B measurement

Date: 2026-11-17. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `6367f54bf866dbc91681b713218bdaa0aca11939`. Independent core review required; no row closes.

| Measurement | Ten new attempts | Thirty cumulative 27B attempts |
| --- | --- | --- |
| Strict ungraded proposals | 10/10 | 27/30 |
| Task passes, first attempt / retries | Unavailable / unavailable | Unavailable / unavailable |
| Request/check seconds | 381.778 | 1389.361 |
| Reserved-job work seconds | 533.819 | 1764.111 |
| Reserved-job GPU minutes | 8.897 | 29.402 |
| Backend input / output tokens | 64036 / 11162 | 186628 / 32707 |
| Counter coverage | 10/10 | 29/30 |

New ten attempts (positions 21–30: dev-021 through dev-029 and dev-rust-001): 10/10 strict
ungraded proposals; 381.778 request/check seconds,
533.819 reserved-job work seconds / 8.897
reserved-job GPU minutes; 64036 input / 11162 output tokens,
with backend counters for 10/10 attempts.
New causes: `{"ungraded-proposal": 10}`.
Cumulative causes: `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 27}`.
Does not fit: 0 cumulative.
Rendered input: 2940–8607 tokens.
Output budget: 24129–29796, using
`min(32768, 32768 - rendered input tokens - 32)`.
Cumulative backend terminations: `{"stopped-below-token-limits": 29, "unobserved-or-ambiguous": 1}`.

The changed variable remains the fixed original-fragment instruction. This
resume changes no source, helper, context, prompt, schema, model setting,
framing, thinking request, budget or deadline. The earlier eighty attempts
remain byte-identical and are counted once. The six-profile repeat is 90/180
complete; 90 attempts remain before tuning or capability work.

Partial path only: Rust context selection, the credential-owning chat HTTP API,
local inference and strict edit checking. Native executable admission and
original execution/grading-control receipts remain unavailable; effects,
project tests, feedback and retries cannot run. No candidate, hidden or
held-out test executes. These proposals are ungraded, not solved tasks.
T-303/T-306 remain open and no model role qualifies.

Five two-task jobs use both resource wrappers, 900-second limits, 30-second
released pauses, low priority, nice 19, 16 GiB RAM and four CPUs. The new
controller wall window is 1652.404 seconds, including queues and released
pauses, excluding preparation and reporting. GPU kernel time, full reservation
duration, continuous cumulative wall time and end-to-end latency are unmeasured.

All 2,283 repository and 44 private bindings, ten dependency capsules and their
complete committed-diff receipts authenticate before inference. The read-only
source snapshot retains 9,706 files. No input or predicate is removed. All 30
contexts and every retained complete new answer check reproduce with the
unchanged SHA-256-authenticated, effect-free Rust helper. No model inference
or candidate execution occurs during reproduction.

Every job revokes settings, reaps API/model processes and removes keys and
sockets. Ten new API tokens are retired; the exact owned-process scan is empty.
No foreign process is signalled, model downloaded or external service used.
Raw jobs, retained candidates without execution, all reporting methods,
original methods and adaptation hashes are sealed in the review capsule.
The prior timeout remains recorded with its missing response/counters; none
is invented. Previous capsules preserve their complete correction chains.
The previous checkpoint received independent PASS for exact
`6367f54bf866dbc91681b713218bdaa0aca11939`, with no new findings. Its report is
retained unchanged and accepts that checkpoint only; it closes no row and gives
no verdict for this increment. Earlier T-309 backlog stays open.

Reporting fast gate, Markdown lint/render, privacy and licence checks,
complete raw audit, all bindings, preservation and helper reproduction pass.
Source is unchanged; source tests are not repeated. The last source gate
remains FAIL: 1,030 passed, 53 inherited failures and 11 ignored; two inherited
Python failures and supply-chain limits remain disclosed. All 512 ledger table
lines, 49 historical private-path blobs, inventory, Decision 0051, status fields,
frozen inputs and previous scoreboard bytes are preserved. No SBOM regeneration,
full acceptance, promotion or release runs. T-303/T-306 remain open.

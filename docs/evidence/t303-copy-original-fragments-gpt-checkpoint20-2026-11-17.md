# T-303/T-306: twenty frozen original-fragment gpt-oss attempts

Date: 2026-11-17. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `a8bc858c860f38658324d600283ffa2efb50d492`. Independent core review required; no row closes.

| Measurement | Ten new attempts | Twenty cumulative gpt-oss attempts |
| --- | --- | --- |
| Strict ungraded proposals | 3/10 | 6/20 |
| Task passes, first attempt / retries | Unavailable / unavailable | Unavailable / unavailable |
| Request/check seconds | 733.798 | 1344.663 |
| Reserved-job work seconds | 808.594 | 1483.472 |
| Reserved-job GPU minutes | 13.477 | 24.725 |
| Backend input / output tokens | 62820 / 136682 | 124645 / 259466 |
| Counter coverage | 10/10 | 20/20 |

New ten attempts (positions 11–20: dev-011 through dev-020):
3/10 strict ungraded proposals;
733.798 request/check seconds,
808.594 reserved-job work seconds /
13.477 reserved-job GPU minutes;
62820 input / 136682 output tokens,
with backend counters for 10/10 attempts.
New causes: `{"generation-refused:error:chat.response.incomplete": 1, "strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 3}`.
Cumulative causes: `{"generation-refused:error:chat.response.incomplete": 3, "strict-edit-refused:code.proposal.edit": 11, "ungraded-proposal": 6}`.
Does not fit: 0 cumulative.
Cumulative rendered input: 2992–8521 tokens.
Cumulative output budget: 24215–29744, using
`min(32768, 32768 - rendered input tokens - 32)`.
Cumulative backend terminations: `{"output-cap": 3, "stopped-below-token-limits": 17}`.
Cumulative response fields: `{"content:absent-or-empty": 3, "content:nonempty": 17, "reasoning:absent-or-empty": 20, "reasoning_content:nonempty": 20}`.
The request sends `chat_template_kwargs.enable_thinking: false`; the unchanged
server template sets `reasoning_effort: medium`. Observed reasoning output
is reported by those response fields; the request does not prove its absence.

The changed variable remains the fixed original-fragment instruction. This
resume changes no source, helper, context, prompt, schema, model setting,
framing, thinking request, budget or deadline. All one hundred earlier attempts
remain byte-identical and are counted once. The six-profile repeat is 110/180
complete; 70 attempts remain before tuning or capability work.

Partial path only: Rust context selection, the credential-owning chat HTTP API,
local inference and strict edit checking. Native executable admission and
original execution/grading-control receipts remain unavailable; effects,
project tests, feedback and retries cannot run. No candidate, hidden or
held-out test executes. These proposals are ungraded, not solved tasks.
T-303/T-306 remain open and no model role qualifies.

Five two-task jobs use both resource wrappers, 900-second limits, 30-second
released pauses, low priority, nice 19, 16 GiB RAM and four CPUs. The new
controller wall window is 1949.934 seconds, including queues and released
pauses, excluding preparation and reporting. GPU kernel time, full reservation
duration, continuous cumulative wall time and end-to-end latency are unmeasured.

All 2,283 repository and 44 private bindings, twelve dependency capsules and
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
The preceding reporting increment is under independent review at session start;
no verdict is inferred for it or this increment. Existing T-309 backlog stays
open. A failed preflight launch before its script existed and failed preparation
of a reporting method are retained; neither started a model or changed any
measured input. The working reporting method is retained and authenticated. The preceding
checkpoint review subsequently returned PASS with no high or medium finding.
Its low reporting note joins T-309; this report supplies the requested thinking
settings and response-field disclosure. The review is retained unchanged. The initial packaging guard also caught an
expected-file list that omitted this retained review. Its failed copy and every
original method are retained; the corrected guard requires all four public files
and seals a fresh capsule. No measured input or published number changed.

Reporting fast gate, Markdown lint/render, privacy and licence checks,
complete raw audit, all bindings, preservation and helper reproduction pass.
Source is unchanged; source tests are not repeated. The last source gate
remains FAIL: 1,030 passed, 53 inherited failures and 11 ignored; two inherited
Python failures and supply-chain limits remain disclosed. All 512 ledger table
lines, 49 historical private-path blobs, inventory, Decision 0051, status fields,
frozen inputs and previous scoreboard bytes are preserved. No SBOM regeneration,
full acceptance, promotion or release runs. T-303/T-306 remain open.

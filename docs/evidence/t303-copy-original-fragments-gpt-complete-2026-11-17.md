# T-303/T-306: completed frozen original-fragment gpt-oss measurement

Date: 2026-11-18 (batch begun 2026-11-17). Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `b8e4b7ff9a4ec624b1d8801d9772196d7e162f11`. Independent core review required; no row closes.

| Measurement | Ten new attempts | Thirty cumulative gpt-oss attempts |
| --- | --- | --- |
| Strict ungraded proposals | 2/10 | 8/30 |
| Task passes, first attempt / retries | Unavailable / unavailable | Unavailable / unavailable |
| Request/check seconds | 921.164 | 2265.828 |
| Reserved-job work seconds | 999.201 | 2482.673 |
| Reserved-job GPU minutes | 16.653 | 41.378 |
| Backend input / output tokens | 63884 / 174129 | 188529 / 433595 |
| Counter coverage | 10/10 | 30/30 |

New ten attempts (positions 21–30: dev-021 through dev-029 and dev-rust-001):
2/10 strict ungraded proposals;
921.164 request/check seconds,
999.201 reserved-job work seconds /
16.653 reserved-job GPU minutes;
63884 input / 174129 output tokens,
with backend counters for 10/10 attempts.
New causes: `{"generation-refused:error:chat.response.incomplete": 5, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 2}`.
Cumulative causes: `{"generation-refused:error:chat.response.incomplete": 8, "strict-edit-refused:code.proposal.edit": 14, "ungraded-proposal": 8}`.
Does not fit: 0 cumulative.
Cumulative rendered input: 2992–8521 tokens.
Cumulative output budget: 24215–29744, using
`min(32768, 32768 - rendered input tokens - 32)`.
Cumulative backend terminations: `{"output-cap": 8, "stopped-below-token-limits": 22}`.
Cumulative response fields: `{"content:absent-or-empty": 8, "content:nonempty": 22, "reasoning:absent-or-empty": 30, "reasoning_content:nonempty": 30}`.
The request sends `chat_template_kwargs.enable_thinking: false`; the unchanged
server template sets `reasoning_effort: medium`. Observed reasoning output
is reported by those response fields; the request does not prove its absence.

The changed variable remains the fixed original-fragment instruction. This
resume changes no source, helper, context, prompt, schema, model setting,
framing, thinking request, budget or deadline. All one hundred and ten earlier attempts
remain byte-identical and are counted once. The six-profile repeat is 120/180
complete; 60 attempts remain before tuning or capability work.

Partial path only: Rust context selection, the credential-owning chat HTTP API,
local inference and strict edit checking. Native executable admission and
original execution/grading-control receipts remain unavailable; effects,
project tests, feedback and retries cannot run. No candidate, hidden or
held-out test executes. These proposals are ungraded, not solved tasks.
T-303/T-306 remain open and no model role qualifies.

Five two-task jobs use both resource wrappers, 900-second limits, 30-second
released pauses, low priority, nice 19, 16 GiB RAM and four CPUs. The final resumed
controller wall window is 199.168 seconds for the last pair, including its
queues and excluding preflight and reporting. Four pairs completed on November
17; their controller was interrupted while the last pair waited for a build
slot. Its exit code, exit time and interruption cause are unobserved. No last-pair
job directory existed and no owned process was live when resuming on November
18. The queue log and launch records are preserved; completed attempts are never
replayed. Only the unstarted last pair resumes, through the unchanged controller
and both resource wrappers. GPU kernel time, full reservation duration, the
interrupted controller duration, continuous cumulative wall time and end-to-end
latency are unmeasured.

All 2,283 repository and 44 private bindings, thirteen dependency capsules and
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
The preceding reporting increment has an independent PASS; its report is
retained unchanged. No verdict is inferred for this increment. Existing T-309 backlog stays
open. Earlier preflight and reporting corrections remain retained unchanged in
the preceding capsules. They changed no measured inputs. This increment only
adapts reporting paths, counts and ranges; original methods and every adaptation
are retained. The frozen gpt-oss profile is now complete; Muse and E4B remain.

Reporting fast gate, Markdown lint/render, privacy and licence checks,
complete raw audit, all bindings, preservation and helper reproduction pass.
Source is unchanged; source tests are not repeated. The last source gate
remains FAIL: 1,030 passed, 53 inherited failures and 11 ignored; two inherited
Python failures and supply-chain limits remain disclosed. All 512 ledger table
lines, 49 historical private-path blobs, inventory, Decision 0051, status fields,
frozen inputs and previous scoreboard bytes are preserved. No SBOM regeneration,
full acceptance, promotion or release runs. T-303/T-306 remain open.

Reservation accounting: an initial idle launcher held an outer build reservation
while waiting for a nested build slot. Before any new job or inference started,
only its PID/start-time-pinned process group was terminated; all queue and exit
receipts are retained. The resumed launcher holds a build slot for snapshot
preparation only, then releases it before starting the controller. Every pair
still acquires both required build/GPU wrappers and releases them between pairs,
with unchanged priority, limits, inputs, budgets and deadlines. The reported
reserved-job work/GPU minutes cover the measured job body, not either queue
window, preparation or full reservation duration. The cancelled queue window
is separate from the reported final resumed controller wall interval; no attempt,
answer or work time from it is invented or dropped.

The preceding checkpoint review subsequently returned PASS with no high or medium finding; its report is retained unchanged. It closes no row and gives no verdict for this increment.

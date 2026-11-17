# T-303/T-306: frozen original-fragment 27B twenty-attempt checkpoint

Date: 2026-11-17. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `15d409083b921e71ef98d1033325908560cd12f6`. Independent core review required; no row closes.

Ten new attempts, dev-011 through dev-020, extend the same frozen profile.
The preceding seventy attempts remain byte-identical and are counted once.
The repeat is 80/180 complete; 100 attempts remain before tuning or capability
work. Source, helper, prompt, contexts, schema, model configuration, framing,
thinking request, budgets and deadlines are unchanged.

| Measurement | Ten new attempts | Twenty cumulative 27B attempts |
| --- | --- | --- |
| Strict ungraded proposals | 8/10 | 17/20 |
| Task passes, first attempt / retries | Unavailable / unavailable | Unavailable / unavailable |
| Request/check seconds | 641.948 | 1007.583 |
| Reserved-job work seconds | 728.470 | 1230.292 |
| Reserved-job GPU minutes | 12.141 | 20.505 |
| Backend input / output tokens | 60249 / 10313 | 122592 / 21545 |
| Counter coverage | 9/10 | 19/20 |

New causes: `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 8}`.
Cumulative causes: `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 17}`.
Does not fit: 0 cumulative. Served context:
32,768 tokens; output uses `min(32768, 32768 - rendered input tokens - 32)`.
All 30 contexts and every retained complete new answer check reproduce with the unchanged,
SHA-256-authenticated effect-free Rust helper.

The measured path is Rust context selection, the credential-owning chat HTTP
API, local inference and Rust strict edit checking. Native executable admission
and original execution/grading-control receipts remain unavailable and are
never reconstructed or bypassed. Effects, project tests, feedback and retries
cannot run. No candidate, hidden or held-out test executes. A mechanically
valid proposal is ungraded and does not establish task success.

Five new two-task jobs use both resource wrappers, 900-second limits,
30-second released pauses, low build priority, nice 19, 16 GiB RAM and four
CPUs. The read-only source snapshot retains 9,706 files. All 2,283 repository
and 44 private bindings, nine prior capsules and their complete committed-diff
receipts are authenticated before inference. No input or predicate is removed.
The new controller wall window is 853.908 seconds, including queues and
released pauses, excluding preparation and reporting. Earlier interrupted
controller exit/duration stays unobserved; continuous cumulative wall time,
GPU kernel time, full reservation duration and end-to-end latency are unmeasured.

Every job revokes settings, reaps API/model processes and removes keys and
sockets. Ten new API tokens are retired; the exact owned-process scan is empty.
No foreign process is signalled, model downloaded or external service used.
New raw jobs, candidates kept without execution, reporting methods, original
methods and adaptation hashes are sealed in the review capsule. The initial raw
audit failed because it assumed an upstream response capture for every request.
Dev-012 has a recorded 300-second transport timeout, no complete answer and no
upstream response capture. Its base fallback is preserved; no response bytes or
missing counters are invented. The corrected audit accepts only this exact
timeout event with both capture files absent, no answer/check output, and the
recorded generation refusal. The failed audit and all intermediate methods are
retained. The runtime and measured inputs are unchanged. Previous
capsules and raw bytes are preserved. The preceding checkpoint received independent PASS for exact
`15d409083b921e71ef98d1033325908560cd12f6`, with no new findings. Its report
is retained unchanged; it accepts that checkpoint only and closes no row. Existing T-309 low findings stay open. No builder review
closes a row.

Reporting fast gate, Markdown lint/render, privacy and licence checks,
complete raw audit, all bindings, preservation and helper reproduction pass.
Source is unchanged; source tests are not repeated. The last source gate
remains FAIL: 1,030 passed, 53 inherited failures and 11 ignored; two inherited
Python failures and supply-chain limits remain disclosed. All 512 ledger table
lines, 49 historical private-path blobs, inventory, Decision 0051, status fields,
frozen inputs and previous scoreboard bytes are preserved. No SBOM regeneration,
full acceptance, promotion or release runs. T-303/T-306 remain open.

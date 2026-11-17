# T-303/T-306: frozen original-fragment 31B development completion

Date: 2026-11-17. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `b140a69ffb402fd93f2f97e759b0d5497f6079a9`. Independent core review required; no row closes.

Ten new attempts, dev-021 through dev-029 and dev-rust-001, complete this profile. All 20 earlier
31B and all 30 A4B attempts remain byte-identical. No attempt is replayed.
The repeat is 60/180 complete; four profiles and 120 attempts remain before
tuning or capability work. Source, helper, prompt, contexts, schema, model
configuration, framing, thinking request, budgets and deadlines are unchanged.

| Measurement | New ten | 31B cumulative thirty |
| --- | --- | --- |
| Strict ungraded proposals | 6/10 | 22/30 |
| Task passes, first attempt / retries | Unavailable / unavailable | Unavailable / unavailable |
| Request/check seconds | 563.615 | 1840.248 |
| Reserved-job work seconds | 986.244 | 2742.708 |
| Reserved-job GPU minutes | 16.437 | 45.712 |
| Backend input / output tokens | 73179 / 11600 | 216073 / 41676 |
| Counter coverage | 10/10 | 30/30 |

New causes: `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 6}`.
Cumulative causes: `{"strict-edit-refused:code.proposal.edit": 8, "ungraded-proposal": 22}`.
Does not fit: zero. The served context is 32,768 tokens; output uses
`min(32768, 32768 - rendered input tokens - 32)`.
All 30 contexts and ten new answer checks reproduce with the unchanged
SHA-256-authenticated effect-free Rust helper.

This is the partial path: Rust context selection, credential-owning chat HTTP
API, local inference, and Rust strict edit checking. Native executable admission
and the original execution/grading-control receipts are unavailable. They are
never reconstructed or bypassed. Effects, project tests, feedback and retries
cannot run. No candidate, hidden or held-out test executes. A mechanically
valid proposal is ungraded and does not establish task success.

Five two-task jobs use both resource wrappers, 900-second job limits, 30-second
released pauses, low build priority, nice 19, 16 GiB RAM and four CPUs.
The source view is the same read-only 9,706-file snapshot. All 2,283 repository
and 44 private bindings and all six prior capsules plus their portable supplement
are checked before inference; no prior input is removed. The new controller
window is 2334.972 seconds including queues and released pauses, excluding
preparation and reporting. Earlier controller duration remains as previously
reported, including its unobserved interruption. Kernel time, full reservation
time and end-to-end latency are not measured.

Completed jobs revoke settings, reap API/model processes and remove keys and
sockets. Ten new API tokens are retired; the exact owned-process scan is empty.
No foreign process is signalled, model downloaded or external service used.
Raw records, responses, backend counters, all reporting methods, original
methods and every correction record are sealed in the private review capsule.
Dependency manifests and full committed-diff receipts are verified without
rewriting or relabelling earlier evidence. The checkpoint20 independent review returns PASS for exact
`b140a69ffb402fd93f2f97e759b0d5497f6079a9` with no new findings. Its report
is retained unchanged; it accepts that checkpoint only and closes no row.
Earlier backlog remains open; this increment requires its own review.

The reporting fast gate, Markdown lint/render, private-path check, licence
validator, preservation check, full bindings and helper reproduction pass.
Source is unchanged, so source tests are not repeated. Last source-gate result
remains FAIL: 1,030 passed, 53 inherited failures and 11 ignored; two inherited
Python failures and supply-chain limits remain disclosed. No full acceptance
or SBOM regeneration runs. All 512 ledger table lines, 49 historical path blobs,
historical inventory, Decision 0051, status fields, frozen inputs and previous
scoreboard bytes are preserved. Existing T-309 backlog and owner-only blockers
remain open. Independent review alone accepts this increment.

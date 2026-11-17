# T-303/T-306: frozen original-fragment 27B development checkpoint

Date: 2026-11-17. Source `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Reporting base `49a41f1c7e8919de52462fb60d04010662df16c3`. Independent core review required; no row closes.

Ten new attempts, dev-001 through dev-010, start the next frozen profile.
All thirty A4B and thirty 31B attempts remain byte-identical and are counted
once. The repeat is 70/180 complete; 110 attempts remain before tuning or
capability work. Source, helper, prompt, contexts, schema, model configuration,
framing, thinking request, budgets and deadlines are unchanged.

| Measurement | Ten new 27B attempts |
| --- | --- |
| Strict ungraded proposals | 9/10 |
| Task passes, first attempt / retries | Unavailable / unavailable |
| Request/check seconds | 365.635 |
| Reserved-job work seconds | 501.822 |
| Reserved-job GPU minutes | 8.364 |
| Backend input / output tokens | 62343 / 11232 |
| Counter coverage | 10/10 |

Causes: `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 9}`.
Does not fit: 0. Served context: 32,768 tokens;
output uses `min(32768, 32768 - rendered input tokens - 32)`.
All 30 contexts and the ten new answer checks reproduce with the unchanged,
SHA-256-authenticated effect-free Rust helper.

The measured path is Rust context selection, the credential-owning chat HTTP
API, local inference and Rust strict edit checking. Native executable admission
and original execution/grading-control receipts remain unavailable and are
never reconstructed or bypassed. Effects, project tests, feedback and retries
cannot run. No candidate, hidden or held-out test executes. A mechanically
valid proposal is ungraded and does not establish task success.

Five two-task jobs use both resource wrappers, 900-second limits, 30-second
released pauses, low build priority, nice 19, 16 GiB RAM and four CPUs.
The read-only source snapshot has the same 9,706 files. All 2,283 repository
and 44 private bindings, eight prior capsules and their committed-diff receipts
are authenticated before inference. No input or predicate is removed. The
controller wall window is 2556.705 seconds, including queues and released
pauses but excluding preparation and reporting. Earlier controller interruptions
keep their previously disclosed unobserved exit/duration; no cumulative
continuous wall time is claimed. GPU kernel time, complete reservation duration
and end-to-end latency are unmeasured.

Every job revokes settings, reaps API/model processes and removes keys and
sockets. Ten new API tokens are retired and the exact owned-process scan is
empty. No foreign process is signalled, model downloaded or external service
used. New raw jobs, candidates kept without execution, reporting methods,
original methods and correction provenance are sealed in the review capsule.
The pre-inference reporting preparation initially used a wrong lookup for the
previous profile receipt; it failed before verification. The corrected verifier
uses the existing authenticated provenance file. Both versions and the failure
are retained; no measured setting changes. The independent review of exact
`49a41f1c7e8919de52462fb60d04010662df16c3` returns PASS with no new findings.
Its unchanged report is retained; it accepts that commit only, closes no row
and gives no verdict for this checkpoint. Earlier low findings
stay T-309 backlog. No builder review closes a row.

Reporting fast gate, Markdown lint/render, privacy and licence checks, complete
raw audit, all bindings, preservation and helper reproduction pass. Source is
unchanged and source tests are not repeated. The last source gate remains FAIL:
1,030 passed, 53 inherited failures and 11 ignored; two inherited Python failures
and supply-chain limits remain disclosed. All 512 ledger table lines, the 49
historical private-path blobs, inventory, Decision 0051, status fields, frozen
inputs and previous scoreboard bytes are preserved. No SBOM regeneration,
full acceptance, promotion or release runs. T-303/T-306 remain open.

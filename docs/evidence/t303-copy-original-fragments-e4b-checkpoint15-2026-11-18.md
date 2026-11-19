# T-303/T-306: frozen E4B development checkpoint, fifteen attempts

Date: 2026-11-18 (local); new jobs: 2026-11-19 UTC.
Reporting base: `fc774f8ea9344efaa634bbcf5cb954fd4752a302`.
Frozen prompt/checker source: `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.

| Observation | New increment | Cumulative E4B |
| --- | --- | --- |
| Completed inference attempts | 4 (dev-012 through dev-015) | 14 |
| Preserved resource-interrupted attempts | 0 | 1 (dev-007) |
| Strict ungraded proposals / completed attempts | 0/4 | 4/14 |
| Total attempts including interruption | 4 | 15 |
| First-attempt / within-budget task passes | Unavailable / unavailable | Unavailable / unavailable |
| Backend input / output tokens | 29504 / 41741 | 98117 / 156927 |
| Backend token-counter coverage | 4/4 | 14/15 |
| Request/check seconds | 341.435 | 1608.647 |
| Reserved-job body seconds / GPU minutes | 359.856 / 5.998 | 1704.523 / 28.409 |

The six-profile repeat reaches **165/180 attempted, 15 remaining**.
Proposal rates use completed attempts; the preserved dev-007 interruption stays
in the attempt total. Its backend counters remain unavailable. The failed job,
partial dev-008 context (constructed; prompt rendering failed before inference)
and both previous pre-start admission-refusal logs remain unchanged. No task
is replayed. There is no new admission refusal in this increment.

New causes: `{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 3}`.
Cumulative causes: `{"generation-refused:complete:length": 4, "resource-interrupted:gpu-headroom": 1, "strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 4}`.
New backend terminations: `{"output-cap": 1, "stopped-below-token-limits": 3}`.
New response fields: `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:absent-or-empty": 4}`.
Does not fit: 0. Rendered input 3247–9734
tokens; output budgets 23002–29489.
The unchanged wire request disables thinking; response fields do not establish
absence of internal reasoning.

The changed measured variable remains Decision 0212's frozen original-fragment
instruction. Source, prompt, helper, context selection, model profile, schema,
thinking request, deadlines and budgets are unchanged. Served context is 32768;
output remains `min(32768, 32768 - rendered input tokens - 32)` per task.
Both unchanged wrappers are reacquired for each 900-second job, with a
30-second pause with reservations released, low priority, nice 19, 16 GiB RAM,
four CPUs and the unchanged 22528 MiB GPU guard. No foreign process is stopped
and no limit is raised. Snapshot preparation releases its build slot before
the controller starts. The complete 9706-file Git-backed snapshot is read-only.

Controller wall time is 400.124 seconds including queues and the released
pause, excluding snapshot preparation and reporting. New request/check time
is 341.435 seconds; reserved-job body time is
359.856 seconds / 5.998
GPU minutes. Body time excludes queues and is neither GPU kernel time nor full
reservation duration. Continuous campaign wall time and end-to-end latency
remain unmeasured. Counter coverage is 4/4 new and 14/15 cumulative attempts.

Partial path only: Rust context selection, credential-owning chat HTTP API,
local inference and strict checking. Native executable admission and original
execution/grading controls are unavailable. Effects, project tests, feedback
and retries cannot run. No candidate, hidden or held-out test executes. Task
passes remain unavailable; proposals are ungraded. No model role qualifies.
T-303/T-306 remain open. Finish the remaining 15 attempts before tuning or
capability work. Independent core review is required; no row closes.

Before continuation, the preceding clean commit's original verifier passed,
checking all dependencies, raw previous attempts, live state and postcommit
bindings. This increment retains 23 authenticated dependency capsules. All
2283 repository and 44 private frozen input bindings remain identical.
The scheduling-only changes advance to dev-012/index11 with fresh reservation
logs; the frozen worker and collector are unchanged.

Cleanup checks all 83 job reports, including the preserved failed job: revoked
settings, reaped API/model processes, absent keys, sockets and API tokens.
Four new tokens are retired privately. Raw contexts, wire requests/responses,
backend counters, checks and candidates are sealed for independent review.
The complete committed diff is bound to the sealed public bytes.

Reporting fast gate, Markdown lint/render, privacy, licence, preservation and
sealed verification are required. Source tests are not repeated because source
is unchanged. The last source gate remains FAIL: 1030 passed, 53 inherited
failures and 11 ignored; two Python failures and supply-chain limits remain
disclosed. All 512 ledger table lines, 49 historical path blobs, inventory,
Decision 0051, status fields and preceding scoreboard bytes are preserved.
No SBOM regeneration, acceptance, role adoption or release runs.

The preceding exact-commit independent review returned PASS with no new findings; its report is retained unchanged. It accepts only that earlier measurement. Existing T-309 backlog remains open.

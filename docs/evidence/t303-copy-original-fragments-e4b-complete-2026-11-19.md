# T-303/T-306: frozen E4B completion and six-profile repeat

Reporting date: 2026-11-18 (local); jobs: 2026-11-19 UTC.
Reporting base: `2b909c4656866425c42dceb8d4393b1ec89ab82c`.
Frozen prompt/checker source: `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.

| Observation | New increment | Cumulative E4B |
| --- | --- | --- |
| Completed inference attempts | 15 (dev-016–dev-029 plus dev-rust-001) | 29 |
| Preserved resource-interrupted attempts | 0 | 1 (dev-007) |
| Strict ungraded proposals / completed attempts | 7/15 | 11/29 |
| Total attempts including interruption | 15 | 30 |
| First-attempt / within-budget task passes | Unavailable / unavailable | Unavailable / unavailable |
| Backend input / output tokens | 108102 / 103605 | 206219 / 260532 |
| Backend token-counter coverage | 15/15 | 29/30 |
| Request/check seconds | 915.625 | 2524.272 |
| Reserved-job body seconds / GPU minutes | 983.720 / 16.395 | 2688.243 / 44.804 |

The frozen six-profile repeat reaches **180/180 attempted, 0 remaining**:
179 completed inference attempts and one preserved resource interruption.
E4B proposal rates use completed attempts. The failed dev-007 job, its partial
dev-008 context (constructed; prompt rendering failed before inference), and
both historical pre-start admission-refusal logs remain unchanged. No task
is replayed. There is no new admission refusal in this increment.

New E4B causes: `{"generation-refused:complete:length": 2, "strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 7}`.
Cumulative E4B causes: `{"generation-refused:complete:length": 6, "resource-interrupted:gpu-headroom": 1, "strict-edit-refused:code.proposal.edit": 12, "ungraded-proposal": 11}`.
New backend terminations: `{"output-cap": 2, "stopped-below-token-limits": 13}`.
New response fields: `{"content:nonempty": 15, "reasoning:absent-or-empty": 15, "reasoning_content:absent-or-empty": 15}`.
Does not fit: 0. Rendered input 3521–9467
tokens; output budgets 23269–29215.
The unchanged wire request disables thinking; response fields do not establish
absence of internal reasoning.

The changed measured variable remains Decision 0212's frozen original-fragment
instruction. Source, prompt, helper, context selection, profiles, schema,
thinking request, deadlines and budgets are unchanged. Served context is 32768;
output is `min(32768, 32768 - rendered input tokens - 32)` per task.
Eight jobs reacquire both unchanged wrappers, each with a 900-second deadline,
30-second released pauses, low priority, nice 19, 16 GiB RAM, four CPUs and the
unchanged 22528 MiB GPU guard. No foreign process is stopped or limit raised.
The last odd-start job contains only dev-rust-001; its legacy count=2 is scheduling
metadata, not evidence of two attempts. The earlier handoff and my progress
labels incorrectly called the final task dev-030. The frozen task list and
worker selected dev-rust-001. The scheduling preflight checked an unused
dev-030 candidate path; the worker stayed unchanged and selected the correct
task. Reporting and packaging now follow recorded IDs; no task is replayed.
Snapshot preparation releases its build
slot before the controller starts; the 9706-file Git-backed snapshot is read-only.

Controller wall time is 1782.935 seconds including queues and released pauses,
excluding preparation and reporting. New request/check time is
915.625 seconds; reserved-job body time is
983.720 seconds / 16.395
GPU minutes. Body time excludes queues and is neither GPU kernel time nor full
reservation duration. Continuous campaign wall time and end-to-end latency
remain unmeasured. Counter coverage is 15/15 new and 29/30 cumulative E4B.

Partial path only: Rust context selection, credential-owning chat HTTP API,
local inference and strict checking. Native executable admission and original
execution/grading controls are unavailable. Effects, project tests, feedback
and retries cannot run. No candidate, hidden or held-out test executes. Task
passes remain unavailable; proposals are ungraded. No model role qualifies.
T-303/T-306 stay open; completing this partial repeat does not close either row.

Across all six profiles, strict ungraded proposals are gemma4-31b: 22/30 completed; qwen35-27b: 27/30 completed; gpt-oss: 8/30 completed; muse: 22/30 completed; gemma4-e4b: 11/29 completed; gemma4-26b-a4b: 17/30 completed.
All-profile causes: `{"generation-refused:complete:length": 6, "generation-refused:error:chat.response.incomplete": 8, "generation-refused:error:chat.transport.timeout": 4, "resource-interrupted:gpu-headroom": 1, "strict-edit-refused:code.proposal.edit": 50, "strict-edit-refused:code.proposal.format": 4, "ungraded-proposal": 107}`.
The largest measured failure cause is `strict-edit-refused:code.proposal.edit` (50 attempts).
Under MEASURE FIRST, that cause chooses the next work. Inspect the failing
strict edits against their supplied source excerpts before selecting a change;
do not weaken exact matching or treat a proposal as a solved task. Git capability
work remains behind measured reliability; other service tools, flows and team
mode remain later work. No new runtime capability or tuning ships here.

Before continuation the preceding clean commit's original verifier passed,
checking all dependencies, raw previous attempts, live state and postcommit
bindings. All 24 dependency capsules and 2283 repository plus 44 private frozen
input bindings remain authenticated. Scheduling advances to dev-016/index15;
the frozen worker and collector stay unchanged.

Cleanup checks all 91 job reports, including the preserved failed job: revoked
settings, reaped API/model processes, absent keys, sockets and API tokens.
All token files are now absent. The original cleanup removed them, then failed
my incorrect 15-token assertion (eight jobs create two tokens each). Its
per-token hashes were lost with the failed process and are unavailable; the
removed count is not retrospectively observed. The original script and failure
logs are preserved. A separate read-only follow-up verifies token/key/socket
absence, revoked settings and reaped processes across all 91 reports. No
retirement receipt is reconstructed. This reporting correction changes no
inference, frozen worker, source or task outcome. Raw requests/responses,
backend counters, checks and candidates are sealed for independent core review.
The complete committed diff is bound to the sealed public bytes.

Reporting fast gate, Markdown lint/render, privacy, licence, preservation and
sealed verification are required. Source tests are not repeated because source
is unchanged. The last source gate remains FAIL: 1030 passed, 53 inherited
failures and 11 ignored; two Python failures and supply-chain limits remain
disclosed. All 512 ledger table lines, 49 historical path blobs, inventory,
Decision 0051, status fields and preceding scoreboard bytes are preserved.
No SBOM regeneration, acceptance, role adoption or release runs.

The preceding exact-commit independent review returned PASS with no new findings; its report is retained unchanged. It accepts only that earlier checkpoint. Existing T-309 backlog stays open. No verdict is inferred for
this increment. Only the separate reviewer can accept it; no row closes.

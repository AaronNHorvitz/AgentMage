# T-303/T-306: frozen E4B development checkpoint, eleven attempts

Date: 2026-11-18 (local); resumed jobs: 2026-11-19 UTC.
Reporting base: `e22ee8e84b84d73a72704196ded2ad51587f3ada`.
Frozen prompt/checker source: `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.
Independent core review required; no row closes.

| Observation | New increment | Cumulative E4B |
| --- | --- | --- |
| Completed inference attempts | 4 (dev-008 through dev-011) | 10 |
| Preserved resource-interrupted attempts | 0 | 1 (dev-007) |
| Strict ungraded proposals / completed attempts | 1/4 | 4/10 |
| Total attempts including interruption | 4 | 11 |
| First-attempt / within-budget task passes | Unavailable / unavailable | Unavailable / unavailable |
| Backend input / output tokens | 23790 / 66205 | 68613 / 115186 |
| Backend token-counter coverage | 4/4 | 10/11 |
| Request/check seconds | 704.837 | 1267.211 |
| Reserved-job body seconds / GPU minutes | 740.201 / 12.337 | 1344.668 / 22.411 |

The six-profile repeat is now **161/180 attempted**, with **19 E4B attempts
remaining**. These are proposal measurements, not solved-task scores.
The original dev-007 interruption remains counted once, with unavailable
backend counters. Its original job and partial dev-008 context remain unchanged;
that dev-008 context had completed construction but failed during prompt
rendering before inference. The preserved subsequent admission-refusal log is
unchanged and counts as no task attempt.

The clean preceding commit's original verifier passed before this resumption,
authenticating all dependencies, raw previous attempts, live frozen inputs and
the complete committed diff. This increment retains 22 authenticated dependency
capsules. The complete 9,706-file Git-backed source snapshot is read-only during
the run; preparation releases its build slot before controller startup. All
2,283 repository and 44 private input bindings stay identical.

A further admission refusal in this increment also occurred before worker or
model startup, with no new task attempt. Its fresh launcher, reservation log and
reporting-time absence receipt are retained. A later independent low-memory
sample does not establish the exact refused baseline or its source. The next
fresh request followed a released pause and used unchanged wrappers and limits.

A separate explicit resume controller starts at index 7, without replaying the
interrupted seventh attempt or changing the frozen worker. Its only change
from the preceding refused resume is fresh reservation-log paths, preserving
the refusal log. Two fresh two-task jobs use both unchanged resource wrappers,
900-second deadlines, a 30-second pause with reservations released, low
priority, nice 19, 16 GiB RAM, four CPUs and the unchanged 22,528 MiB GPU guard.
No other worker or process is stopped and no limit is raised.

The changed measured variable remains the original-fragment instruction from
Decision 0212. This increment changes no source, prompt, helper, source
selection, schema, context, model profile, thinking request, deadline or budget.
The served context is 32,768 tokens. Each output budget remains
`min(32768, 32768 - rendered input tokens - 32)`.
New rendered input ranges from 3413 to
9374 tokens; output budgets range from
23362 to 29323.
Does-not-fit cases: zero.

New failure causes/ungraded results: `{"generation-refused:complete:length": 2, "strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`.
Cumulative causes: `{"generation-refused:complete:length": 3, "resource-interrupted:gpu-headroom": 1, "strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 4}`.
New backend terminations: `{"output-cap": 2, "stopped-below-token-limits": 2}`.
New response fields: `{"content:nonempty": 4, "reasoning:absent-or-empty": 4, "reasoning_content:absent-or-empty": 4}`.
The unchanged request sends `chat_template_kwargs.enable_thinking: false`;
response-field presence does not establish absence of internal reasoning.

Successful resumed-controller wall time is 773.090 seconds, including queues
and the released pause, excluding the earlier refused controller, snapshot
preparation and reporting. Reserved-job body time
excludes queues and is not GPU kernel time or full reservation duration.
Continuous campaign wall time, kernel time and full end-to-end latency are
unmeasured. Token totals exclude the original interrupted response, whose
backend counters remain unavailable; cumulative coverage is stated above.

This is a partial path: Rust context selection, the credential-owning chat HTTP
API, local inference and strict edit checking. Native executable admission and
original execution/grading-control receipts remain unavailable. Effects,
project tests, feedback and retries cannot run. No candidate, hidden or held-out
test executes. Task passes remain unavailable; proposals are ungraded. No
model role qualifies and T-303/T-306 remain open.

Cleanup verifies revoked settings, reaped API/model processes and absent keys,
sockets and API tokens for all 81 preceding/new job reports, including the
preserved failed job. New API tokens are retired privately. Raw requests,
responses, contexts, check results, backend logs and candidates remain sealed
for independent review; no generated code executes during reporting.
Reporting fast gate, Markdown lint/render, privacy, licence, preservation and
sealed raw verification are required. Source tests are not repeated because no
source changes. The last source gate remains FAIL: 1,030 passed, 53 inherited
failures and 11 ignored; two Python failures and supply-chain limits remain
disclosed. All 512 ledger table lines, 49 historical path blobs, inventory,
Decision 0051, status fields and earlier scoreboard bytes are preserved.
No SBOM regeneration, full acceptance, promotion or release runs.

The preceding exact commit was in independent review at session start and
subsequently returned PASS, with no high, medium or new low findings. Its
unchanged report is retained separately; it accepts only the earlier refusal,
not this increment. Existing T-309 backlog remains open. Finish the remaining
19 attempts before tuning or capability work. Only the independent reviewer
accepts this increment; no row closes here.

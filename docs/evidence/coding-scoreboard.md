# Coding scoreboard

The 8,192-token context / 4,096-token output budget stopped **36 of 40 Muse**
and **38 of 40 GPT-OSS** attempts from producing a complete answer, including
every held-out attempt. These scores mainly measure context/output limits and
are not evidence of relative model quality. Later scoreboards must identify a
changed context or output budget as a separate variable.

Actual-run measurements of the fixed chat-only text-proposal baseline. This is
not execution of the native coding agent. The running chat API has no repository
tools, tests or retry loop; the native CLI retains the deferred host-trust blocker.
The [policy and limits](../guides/coding-baseline.md) were committed before the
first request. [Freeze verification](t311-18-baseline-freeze-2026-11-05.md) binds
all 160 control attempts. The [independent core re-review](../reviews/3a6daad09675-local-coding-t311-18-reporting-repair.txt)
passes expanded T-311.18 and parent T-311 at exact commit
`3a6daad096755c8e76ad53fd6c555524f554b5ca`, with no open high/medium finding.

## Baseline runs

Frozen source commit: `20575afe222bfd661821ec4b0f15296d520d59e2`. Both existing
local weight sets used 8K context (the product chat value) with the lab's
4,096-token output reservation (including reasoning),
temperature zero, seed 42 and medium family-specific reasoning setting.
One request per task is the entire budget: there is no retry, test feedback,
retrieval or cross-task conversation. First and within-budget rates therefore
match. No model, prompt, harness or hidden test was tuned during these runs.

| Date (UTC) | Model | Development first / within budget | Held-out first / within budget | Generation s | Grading s | Reserved GPU work min |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-11-06 | muse | 4/30 / 4/30 | 0/10 / 0/10 | 1550.980 | 66.399 | 29.607 |
| 2026-11-06 | gpt-oss | 0/30 / 0/30 | 0/10 / 0/10 | 364.552 | 66.829 | 7.540 |

| Model artifact | Observed weights SHA-256 | Observed API binary SHA-256 |
| --- | --- | --- |
| muse | `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e` | `21ba7c5f827056fdf77c8ed7dc41ee45fce6cc812e40e83a2462095527a30257` |
| gpt-oss | `27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901` | `21ba7c5f827056fdf77c8ed7dc41ee45fce6cc812e40e83a2462095527a30257` |

Muse uses the existing 30B Q4_K_M weights; gpt-oss uses the existing
20B MXFP4 conversion. No model was downloaded or bundled. Role selection is
deferred to the later measured coding campaign.

Generation and grading durations are measured separately and exclude reservation
queue waits. Reserved GPU work minutes time the private launcher through
verification, loading and owned-resource cleanup; wrapper pre/post checks and
queue time are excluded. They are not active GPU kernel minutes.
API token usage is unavailable (null).
The table below supplements it with actual backend timing counters, including
reasoning tokens. A dash means unobserved; no character estimate or zero is
substituted. Counter log slices and all requests/events remain hash-bound privately.

## Failure causes

Backend termination classes below come from the original hash-bound server log
slices, not the API error labels. `prompt-exceeds-context` means the request was
rejected before generation; `context-exhausted` means the backend recorded
`truncated = 1`; `output-cap` means 4,096 generated tokens without context
truncation; `stopped` means generation ended below both limits. A stopped
generation still has to pass API validation, edit application and grading.

| Model | Split | prompt-exceeds-context | context-exhausted | output-cap | stopped |
| --- | --- | --- | --- | --- | --- |
| muse | dev | 10 | 9 | 7 | 4 |
| muse | held | 5 | 5 | 0 | 0 |
| gpt-oss | dev | 10 | 12 | 6 | 2 |
| gpt-oss | held | 5 | 5 | 0 | 0 |

For each model, ten development requests never reached generation. No held-out
attempt fit: five were rejected at input and five exhausted context. All four
Muse generations that ended within the limits passed; the two GPT-OSS
generations that stopped within limits were refused as `chat.response.invalid`.
This does not establish coding quality outside this integration and budget.

Reproduce these counts and each development classification with the
[retained-log reporting command](../guides/coding-baseline.md#retained-log-termination-report).
It checks the original manifest, resources, launchers, aggregates, request output
budgets, observed context properties, timing records and exact log-slice hashes.
It reads no task definitions or hidden tests, executes nothing from a candidate,
and exports only aggregate held-out counts.

Collection-level outcomes remain 4 passes and 36 `chat-failed` for Muse,
and 40 `chat-failed` for GPT-OSS. The backend classes explain the budget limits
that these broad labels and the API terminal messages obscure.

### Observed API terminal outcomes

| Model | Split | Terminal event | Tasks |
| --- | --- | --- | --- |
| muse | dev | complete:length | 2 |
| muse | dev | complete:stop | 4 |
| muse | dev | error:chat.response.incomplete | 14 |
| muse | dev | error:chat.transport.status | 10 |
| muse | held | error:chat.response.incomplete | 5 |
| muse | held | error:chat.transport.status | 5 |
| gpt-oss | dev | complete:length | 1 |
| gpt-oss | dev | error:chat.response.incomplete | 16 |
| gpt-oss | dev | error:chat.response.invalid | 3 |
| gpt-oss | dev | error:chat.transport.status | 10 |
| gpt-oss | held | error:chat.response.incomplete | 5 |
| gpt-oss | held | error:chat.transport.status | 5 |

These fixed API outcomes describe this integration path. They do not identify
a model-internal cause or establish general coding ability. Incomplete, invalid,
status-refused and length-limited streams cannot be treated as valid proposals.

All 30 development and ten held-out tasks remain in each model denominator.
Refused/partial generations retain an explicitly labelled base fallback and
cannot earn a pass. Independent grading uses the original frozen assertions;
collection success alone earns no pass. Held-out feedback was used only for
this row-closing scoreboard, never for tuning. Its ten tasks cover one narrow
protocol domain, not general application coding.

## Development task timings and backend tokens

| Model | Task | Generation s | Grading s | Backend input tokens | Backend output tokens | Cause | Backend termination | Known race |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| muse | dev-001 | 13.750 | 0.525 | 8047 | 145 | chat-failed | context-exhausted | — |
| muse | dev-002 | 0.038 | 0.27 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-003 | 94.417 | 12.24 | 4075 | 4096 | chat-failed | output-cap | process observation |
| muse | dev-004 | 93.442 | 19.605 | 2706 | 4096 | chat-failed | output-cap | SIGINT timing |
| muse | dev-005 | 45.292 | 0.255 | 6390 | 1802 | chat-failed | context-exhausted | — |
| muse | dev-006 | 0.043 | 0.269 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-007 | 0.035 | 0.277 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-008 | 0.032 | 0.271 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-009 | 63.161 | 0.254 | 2684 | 2421 | none | stopped | — |
| muse | dev-010 | 95.523 | 0.252 | 3669 | 4096 | chat-failed | output-cap | — |
| muse | dev-011 | 65.752 | 0.294 | 5725 | 2467 | chat-failed | context-exhausted | — |
| muse | dev-012 | 94.762 | 0.258 | 2703 | 4096 | chat-failed | output-cap | — |
| muse | dev-013 | 0.039 | 0.255 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-014 | 60.176 | 0.31 | 5680 | 2512 | chat-failed | context-exhausted | — |
| muse | dev-015 | 0.037 | 0.231 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-016 | 0.114 | 0.233 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-017 | 99.189 | 0.261 | 3769 | 4096 | chat-failed | output-cap | — |
| muse | dev-018 | 89.455 | 0.247 | 2780 | 3934 | none | stopped | — |
| muse | dev-019 | 0.037 | 0.267 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-020 | 29.509 | 0.255 | 7086 | 1106 | chat-failed | context-exhausted | — |
| muse | dev-021 | 30.424 | 0.258 | 7330 | 862 | chat-failed | context-exhausted | — |
| muse | dev-022 | 43.059 | 0.252 | 6489 | 1703 | chat-failed | context-exhausted | — |
| muse | dev-023 | 22.746 | 0.424 | 7395 | 797 | chat-failed | context-exhausted | — |
| muse | dev-024 | 55.866 | 0.25 | 4882 | 2343 | none | stopped | — |
| muse | dev-025 | 95.195 | 0.25 | 3885 | 4096 | chat-failed | output-cap | — |
| muse | dev-026 | 50.845 | 0.266 | 4482 | 1869 | none | stopped | — |
| muse | dev-027 | 0.068 | 0.276 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-028 | 0.035 | 0.269 | — | — | chat-failed | prompt-exceeds-context | — |
| muse | dev-029 | 17.623 | 0.264 | 7625 | 567 | chat-failed | context-exhausted | — |
| muse | dev-rust-001 | 93.188 | 6.42 | 3111 | 4096 | chat-failed | output-cap | — |
| gpt-oss | dev-001 | 7.542 | 0.514 | 8101 | 91 | chat-failed | context-exhausted | — |
| gpt-oss | dev-002 | 0.030 | 0.26 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-003 | 19.776 | 12.02 | 4124 | 4068 | chat-failed | context-exhausted | process observation |
| gpt-oss | dev-004 | 19.482 | 19.598 | 2761 | 4096 | chat-failed | output-cap | SIGINT timing |
| gpt-oss | dev-005 | 10.077 | 0.251 | 6532 | 1660 | chat-failed | context-exhausted | — |
| gpt-oss | dev-006 | 0.043 | 0.275 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-007 | 0.034 | 0.271 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-008 | 0.031 | 0.267 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-009 | 11.757 | 0.26 | 2778 | 1424 | chat-failed | stopped | — |
| gpt-oss | dev-010 | 19.963 | 0.248 | 3738 | 4096 | chat-failed | output-cap | — |
| gpt-oss | dev-011 | 17.154 | 0.318 | 5813 | 2379 | chat-failed | context-exhausted | — |
| gpt-oss | dev-012 | 19.393 | 0.252 | 2761 | 4096 | chat-failed | output-cap | — |
| gpt-oss | dev-013 | 0.026 | 0.257 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-014 | 13.101 | 0.307 | 5773 | 2419 | chat-failed | context-exhausted | — |
| gpt-oss | dev-015 | 0.036 | 0.216 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-016 | 0.050 | 0.223 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-017 | 20.251 | 0.262 | 3830 | 3239 | chat-failed | stopped | — |
| gpt-oss | dev-018 | 19.437 | 0.258 | 2834 | 4096 | chat-failed | output-cap | — |
| gpt-oss | dev-019 | 0.033 | 0.261 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-020 | 6.982 | 0.25 | 7200 | 992 | chat-failed | context-exhausted | — |
| gpt-oss | dev-021 | 10.247 | 0.252 | 7442 | 750 | chat-failed | context-exhausted | — |
| gpt-oss | dev-022 | 9.470 | 0.258 | 6599 | 1593 | chat-failed | context-exhausted | — |
| gpt-oss | dev-023 | 6.215 | 0.401 | 7442 | 750 | chat-failed | context-exhausted | — |
| gpt-oss | dev-024 | 16.693 | 0.248 | 4936 | 3256 | chat-failed | context-exhausted | — |
| gpt-oss | dev-025 | 20.063 | 0.393 | 3942 | 4096 | chat-failed | output-cap | — |
| gpt-oss | dev-026 | 22.016 | 0.251 | 4596 | 3596 | chat-failed | context-exhausted | — |
| gpt-oss | dev-027 | 0.059 | 0.274 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-028 | 0.045 | 0.256 | — | — | chat-failed | prompt-exceeds-context | — |
| gpt-oss | dev-029 | 5.009 | 0.263 | 7740 | 452 | chat-failed | context-exhausted | — |
| gpt-oss | dev-rust-001 | 19.494 | 7.408 | 3179 | 4096 | chat-failed | output-cap | — |

## Closing verification

The [same-row reporting repair](t311-18-baseline-reporting-repair-2026-11-06.md)
addresses the independent review's medium failure-attribution finding. The
[independent re-review](../reviews/3a6daad09675-local-coding-t311-18-reporting-repair.txt)
reproduces all 80 classifications and passes expanded T-311.18 and parent T-311.
Low L5 remains backlog: the reporter needs the original retained location and
currently collapses specific refusals into a generic diagnostic.

The completed campaign passed parameter and cleanup checks for all 16 batches.
All 80 grading reports and 260 direct log bindings were verified. Replaying the
exact frozen prompt fields and source replacements reproduced all 80 snapshots:
four applied proposals, with every other snapshot explicitly labelled as a base
fallback. The prior scoped gate passed 62 tests and complete controls passed all
160 attempts. No source changed between the freeze and these model runs.

## Scope and retained evidence

These are text-proposal task observations over the declared finite adapters.
They do not establish a native agent loop, repository tools, approved commits
or pushes, or general model quality. Adapter identity/state limitations,
unasserted clauses, deliberate self-modification and candidate-writable
temporary fixtures remain disclosed. Lane quotas cover the entire lane, not
a separately delegated worker cgroup or candidate-only peaks. The dev-003
process-observation race and dev-004 SIGINT timing race remain failures, never
reasons to omit an attempt. The independent reviewer retained a dev-004 base
control timeout (ERROR instead of its frozen FAIL profile), then a passing
unchanged repeat of all 160 controls. This did not affect model scores: both
dev-004 generations hit the output cap. Hidden tests and thresholds stay frozen.

Private retained-manifest SHA-256: `932463662e05b984a28fd3b329c24df898839d10a749e08b28fc8574725ba9f1` (55618 files).
Raw records stay private because they contain source projections, machine-local
paths and held-out identities. Only checked aggregates and hashes are published.
No release, model qualification or owner-ready claim follows.

| Aggregate | SHA-256 |
| --- | --- |
| muse dev | `84f42a8769ed240ed32164fbc8a583f84090d2aa9af0127b47cf4bdcaae65cc3` |
| muse held | `32397bb2331e406e9b2a94ad4e1215aaf674ebd81fe0884bad2e11c3253f8f1b` |
| gpt-oss dev | `979f50e13de69b82844d0b02f96ea5ac0df83381dab5199f203d45c6b129f2a5` |
| gpt-oss held | `ab5ab42508eebbff47c1d6bd6e53b7380ec5ee63deb9e52ebb2f4194e39a1bf8` |

## Snapshot proposals: development checkpoint, 2026-11-06

**57 of 60 attempts hit the 2,048-token output cap.** The three generations
ending within the limits produced checked proposals and passed independent
grading. These results chiefly expose the restrictive output budget, not
relative model quality or a working native coding workflow.

This is a **non-closing T-303/T-306 implementation checkpoint**, not a native
agent-loop run. Source commit: `21f75c8646ee2a1b94eb1f3b9a305851c46c72ff`.
The [proposal API record](t303-snapshot-proposal-api-2026-11-06.md) and
[fixed collector policy](../guides/coding-proposals.md#development-measurement)
describe caller-supplied snapshots, bounded source selection and strict in-memory
edits. No workspace tools, commands, tests, worktree or retry feedback were used
by the model. Both parent rows remain open. Held-out tasks were not run in this
checkpoint; they remain reserved for the combined row-closing scoreboard.

Observed serving: 8,192 context tokens; requests: **6,000 application message
bytes / 2,048 output tokens**, including reasoning. Temperature zero, seed 42
and medium family-specific reasoning. This changes both source selection and
output budget from the original baseline; it cannot isolate their effects.
One attempt per task is the complete budget, so first and within-budget rates
match. No prompt/source change or test feedback occurred during the campaign.

| Model | Development first / within budget | Held-out | Checked proposals | Generation s | Grading s | Reserved GPU work min |
| --- | --- | --- | --- | --- | --- | --- |
| muse | 1/30 / 1/30 | not run | 1/30 | 1412.594 | 49.647 | 25.891 |
| gpt-oss | 2/30 / 2/30 | not run | 2/30 | 303.390 | 48.116 | 6.395 |

Independent grading uses the unchanged finite-observation adapters and frozen
original assertions. A valid proposal still needs a passing grade; a refused
generation/base fallback earns no model credit. All 30 tasks remain in each
denominator. Scores do not establish general coding quality, an executable
coding agent, model-role selection or a comparison of model quality.

### Termination and observed tokens

Backend classes below are derived directly from the retained exact
server-log slices and timing counters. They are not inferred from API error
labels. Output-cap means 2,048 generated tokens without context truncation.

| Model | Prompt exceeds context | Context exhausted | Output cap | Stopped | Observed input tokens | Observed output tokens | Counter coverage |
| --- | --- | --- | --- | --- | --- | --- | --- |
| muse | 0 | 0 | 29 | 1 | 44495 | 61323 | 30/30 |
| gpt-oss | 0 | 0 | 28 | 2 | 46045 | 58868 | 30/30 |

API token usage remains unavailable; the counts above are observed backend
counters, including reasoning. Unknown values stay unobserved, not estimates.
Generation/grading durations exclude queue waits. Reserved GPU work minutes
include verification, loading and owned-process cleanup; they are not active
GPU kernel minutes. Batches contain at most five tasks and release both shared
reservations between jobs.

| Model | API terminal | Attempts |
| --- | --- | --- |
| muse | `chat.response.incomplete` | 29 |
| muse | `proposal` | 1 |
| gpt-oss | `chat.response.incomplete` | 25 |
| gpt-oss | `code.proposal.incomplete` | 3 |
| gpt-oss | `proposal` | 2 |

### Development task records

`pass` below requires both a valid proposed snapshot and a passing independent
grade. Per-task token counts are backend counters. The retained dev-003 process
observation race and dev-004 SIGINT timing race remain failures, never
exclusions. The API path has no retry loop.

| Model | Task | Generation s | Grading s | Input tokens | Output tokens | Backend termination | API terminal | Grade |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| muse | dev-001 | 52.309 | 0.549 | 1543 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-002 | 47.161 | 0.268 | 1429 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-003 | 46.685 | 11.967 | 1475 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-004 | 46.389 | 19.508 | 1413 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-005 | 46.311 | 0.275 | 1446 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-006 | 52.625 | 0.278 | 1412 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-007 | 47.511 | 0.264 | 1430 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-008 | 46.669 | 0.264 | 1448 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-009 | 46.348 | 0.250 | 1403 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-010 | 46.411 | 0.248 | 1484 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-011 | 51.898 | 0.293 | 1467 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-012 | 45.983 | 0.255 | 1476 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-013 | 46.230 | 0.254 | 1490 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-014 | 45.850 | 0.319 | 1360 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-015 | 45.972 | 0.227 | 1556 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-016 | 51.319 | 0.227 | 1408 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-017 | 45.150 | 0.256 | 1502 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-018 | 45.248 | 0.245 | 1425 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-019 | 42.708 | 0.260 | 1568 | 1931 | stopped | `proposal` | pass |
| muse | dev-020 | 45.261 | 0.250 | 1591 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-021 | 51.736 | 0.250 | 1568 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-022 | 46.171 | 0.250 | 1565 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-023 | 46.542 | 0.425 | 1576 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-024 | 46.346 | 0.250 | 1567 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-025 | 46.779 | 0.253 | 1555 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-026 | 50.803 | 0.255 | 1483 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-027 | 45.287 | 0.271 | 1426 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-028 | 45.053 | 0.257 | 1472 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-029 | 44.979 | 0.258 | 1540 | 2048 | output-cap | `chat.response.incomplete` | fail |
| muse | dev-rust-001 | 44.859 | 7.277 | 1417 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-001 | 13.972 | 0.522 | 1586 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-002 | 9.654 | 0.276 | 1467 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-003 | 9.726 | 12.062 | 1520 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-004 | 9.679 | 19.570 | 1466 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-005 | 9.674 | 0.247 | 1504 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-006 | 14.037 | 0.273 | 1458 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-007 | 9.556 | 0.274 | 1468 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-008 | 9.657 | 0.263 | 1494 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-009 | 10.109 | 0.249 | 1478 | 2048 | output-cap | `code.proposal.incomplete` | fail |
| gpt-oss | dev-010 | 9.704 | 0.248 | 1535 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-011 | 13.918 | 0.288 | 1531 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-012 | 9.513 | 0.250 | 1528 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-013 | 9.463 | 0.250 | 1529 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-014 | 9.458 | 0.300 | 1415 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-015 | 9.597 | 0.215 | 1630 | 2048 | output-cap | `code.proposal.incomplete` | fail |
| gpt-oss | dev-016 | 14.557 | 0.227 | 1480 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-017 | 9.440 | 0.252 | 1555 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-018 | 9.764 | 0.238 | 1475 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-019 | 4.026 | 0.260 | 1625 | 747 | stopped | `proposal` | pass |
| gpt-oss | dev-020 | 9.724 | 0.248 | 1649 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-021 | 13.694 | 0.255 | 1630 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-022 | 9.559 | 0.244 | 1621 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-023 | 9.534 | 0.415 | 1618 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-024 | 9.766 | 0.244 | 1603 | 2048 | output-cap | `code.proposal.incomplete` | fail |
| gpt-oss | dev-025 | 9.499 | 0.244 | 1604 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-026 | 8.217 | 0.241 | 1541 | 777 | stopped | `proposal` | pass |
| gpt-oss | dev-027 | 9.457 | 0.265 | 1460 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-028 | 9.497 | 0.262 | 1511 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-029 | 9.466 | 0.246 | 1594 | 2048 | output-cap | `chat.response.incomplete` | fail |
| gpt-oss | dev-rust-001 | 9.476 | 5.966 | 1470 | 2048 | output-cap | `chat.response.incomplete` | fail |

### Checkpoint bindings

| Model | Observed weights SHA-256 | Observed API binary SHA-256 |
| --- | --- | --- |
| muse | `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e` | `1ae162d2b187726673b24b007b8bc38d2fd6585de5bf2518a7ec08b00467d2f3` |
| gpt-oss | `27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901` | `1ae162d2b187726673b24b007b8bc38d2fd6585de5bf2518a7ec08b00467d2f3` |

Audit: **60 grading reports / 300 direct log bindings**, all 60 source
snapshots, exact serialized context byte counts, backend slices and cleanup
records verified. The unchanged accepted complete-controls receipt is reused;
no held-out assertion is re-executed for this checkpoint.

Audited-results SHA-256: `a2f7eacaa0ce6f0975104258d7b0674efe25fbcfde59381d7cf7b6ba4565fac1`.
Retained-manifest SHA-256: `2d8413a399d65cbbbf09aa1882445e2228cef60ff56240c454bc71adc76b4c35` (39149 files).
Raw requests, events, candidates, graders and server logs remain private.
The independent reviewer receives the exact retained location and reproduction
commands. This source batch awaits independent core review and closes no
parent row, acceptance or release gate.

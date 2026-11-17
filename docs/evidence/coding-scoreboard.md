# Coding scoreboard

The 8,192-token context / 4,096-token output budget stopped **36 of 40 Muse**
and **38 of 40 GPT-OSS** attempts from producing a complete answer, including
every held-out attempt. These scores mainly measure context/output limits and
are not evidence of relative model quality. Later scoreboards must identify a
changed context or output budget as a separate variable.

Actual-run measurements of the fixed chat-only text-proposal baseline. This is
not execution of the native coding agent. At that initial baseline, the chat path supplied no repository
tools, tests or retry loop; the native CLI retained the deferred host-trust blocker.
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

## Larger-budget development measurement, 2026-11-10/11

Source: `33ae305a3bc0bca71f3dbdee3dc6f79a4c5eddf3`. **Partial path**, measured
on every frozen development task: current Rust context selection, credential-owning
chat API and strict Rust edit checking. Native worktree/tool/test/retry/approval/
commit/push execution could not start in this lane. Missing original grading
controls also refuse evaluator admission. **First-attempt and within-budget task
passes are unavailable, not zero.** Strict proposals below are ungraded edits,
not solved tasks. Held-out tasks were not loaded or run.

Changed variables from the November 6 development checkpoint: 32,768 context
tokens (was 8,192); a 32,768-byte message-selection budget (was 6,000);
output reservation `min(32768, 32768 - rendered_input_tokens - 32)` (was 2,048);
a private effect-free library adapter to use that larger chat output allowance;
two tasks per cold model load (was five); four additional models using their
family defaults rather than the primary pair's medium reasoning setting.
These are several changed variables, not a controlled attribution experiment.
All 180 rendered inputs fit the served context; does-not-fit count is zero.
One attempt per task, zero retries, zero executed candidate tests.
The effective time ceilings were 120 seconds total and 30 seconds idle, with
unchanged transport byte caps; the larger token allowance did not extend them.
`qwen35-27b` is an opaque retained artifact label, not a verified model version.
Its GGUF metadata supplies `general.size_label: 27B` and
`general.license: apache-2.0`, but no model name or version. The linked base-model
licence does not establish this local conversion's identity or provenance.

| Date (UTC) | Model label | Context tokens | Output-token budget range | Dev first / within budget | Strict ungraded edits | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-10–11 | muse | 32768 | 24278–29882 | unavailable / unavailable (30 tasks) | 10/30 | 3135.649 | 3485.948 | 58.099 |
| 2026-11-10–11 | gpt-oss | 32768 | 24223–29779 | unavailable / unavailable (30 tasks) | 12/30 | 1403.251 | 1628.866 | 27.148 |
| 2026-11-10–11 | gemma4-e4b | 32768 | 22767–29470 | unavailable / unavailable (30 tasks) | 10/30 | 1264.942 | 1409.855 | 23.498 |
| 2026-11-10–11 | gemma4-26b-a4b | 32768 | 22767–29470 | unavailable / unavailable (30 tasks) | 0/30 | 3329.030 | 3830.751 | 63.846 |
| 2026-11-10–11 | gemma4-31b | 32768 | 22767–29470 | unavailable / unavailable (30 tasks) | 0/30 | 3483.553 | 4044.447 | 67.407 |
| 2026-11-10–11 | qwen35-27b | 32768 | 23973–29725 | unavailable / unavailable (30 tasks) | 4/30 | 3405.421 | 3824.544 | 63.742 |

Request/check seconds sum the chat request and strict-check interval; they exclude
context preparation and token preflight. GPU work minutes mean reserved model-job
wall time divided by 60, including verification, loading and cleanup, excluding
queue waits and reservation-wrapper checks; they are not active GPU kernel time.
API usage counters were not requested and remain null. Unambiguous backend
counters, including reasoning tokens, cover the task counts shown below; missing
counters remain unknown. No token estimate replaces them.

| Model | Outcome / cause | Tasks |
| --- | --- | --- |
| muse | `generation-refused:error:chat.transport.timeout` | 18 |
| muse | `strict-edit-refused:code.proposal.format` | 2 |
| muse | `ungraded-proposal` | 10 |
| gpt-oss | `generation-refused:error:chat.response.invalid` | 1 |
| gpt-oss | `generation-refused:error:chat.transport.limit` | 7 |
| gpt-oss | `generation-refused:error:chat.transport.timeout` | 1 |
| gpt-oss | `strict-edit-refused:code.proposal.edit` | 8 |
| gpt-oss | `strict-edit-refused:code.proposal.format` | 1 |
| gpt-oss | `ungraded-proposal` | 12 |
| gemma4-e4b | `generation-refused:error:chat.transport.timeout` | 1 |
| gemma4-e4b | `strict-edit-refused:code.proposal.edit` | 3 |
| gemma4-e4b | `strict-edit-refused:code.proposal.format` | 16 |
| gemma4-e4b | `ungraded-proposal` | 10 |
| gemma4-26b-a4b | `generation-refused:error:chat.transport.limit` | 1 |
| gemma4-26b-a4b | `generation-refused:error:chat.transport.timeout` | 29 |
| gemma4-31b | `generation-refused:error:chat.transport.timeout` | 29 |
| gemma4-31b | `strict-edit-refused:code.proposal.format` | 1 |
| qwen35-27b | `generation-refused:error:chat.transport.timeout` | 26 |
| qwen35-27b | `ungraded-proposal` | 4 |

| Model | Tasks with observed backend counters | Observed input / output tokens |
| --- | --- | --- |
| muse | 12/30 | 71445 / 39013 |
| gpt-oss | 22/30 | 130332 / 124228 |
| gemma4-e4b | 28/30 | 208437 / 117956 |
| gemma4-26b-a4b | 0/30 | unknown |
| gemma4-31b | 1/30 | 9465 / 2071 |
| qwen35-27b | 4/30 | 29274 / 10184 |

The largest refusal cause is the current transport timeout. Next work must address
completion through the existing loop and measure again, then the strict edit
refusals; these observations do not justify another unrelated guard increment.
Restoring the native executable-trust environment and original controls is needed
before a complete coding score can be measured. No model role is selected here.

See [measurement evidence](t303-development-measurement-2026-11-10.md) for exact
scope, model identities, per-task time/tokens, artifact bindings and reproduction.
The later [trusted-timeout review](../reviews/d31e2969c3ab-local-coding-trusted-model-timeouts.txt)
reproduces the complete raw evidence and returns PASS. T-303 and T-306 stay open.

## Trusted timeout selection repeat, 2026-11-11

Source: `6bb0008ae6ca5ad491fedd02c53c8f52e1781b1f`. Partial path: current Rust selection,
credential-owning chat API and strict edit checking. All 30 GPT-OSS development
requests/contexts are byte-identical to their prior counterparts. No candidate
tests, retries or held-out tasks ran; native execution and original-control
admission remain blocked. Other five models were not rerun.

Changed variables: total/idle 300/60 s (was 120/30), containing private relay/client
310 s (was 130), two-task job reservation 900 s (was 600). Context/output and all
byte caps are unchanged. Progress never renews total time. Every input fits.
The 12/30 strict ungraded proposals equal the previous count;
no task-solving improvement is established. Missing task grades remain unknown.

| Date (UTC) | Model | Context tokens | Output-token budget | Total / idle s | Dev first / within budget | Strict ungraded edits | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-11 | gpt-oss | 32768 | 24223–29779 | 300 / 60 | unavailable / unavailable (30 tasks) | 12/30 | 1428.275 | 1607.238 | 26.787 |

GPU work minutes are reserved model-job wall/60, including load/verification/
cleanup and excluding queue/wrapper checks, not active GPU kernel time. Backend
counters cover 23/30 tasks: 138757 input / 143734 output tokens.
Other counters are unknown; API usage was not requested. Zero does-not-fit tasks,
zero transport timeouts (prior GPT-OSS run: one, dev-016), one attempt each;
first/within-budget passes unavailable.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 1 |
| `generation-refused:error:chat.response.invalid` | 1 |
| `generation-refused:error:chat.transport.limit` | 7 |
| `strict-edit-refused:code.proposal.edit` | 8 |
| `strict-edit-refused:code.proposal.format` | 1 |
| `ungraded-proposal` | 12 |

Largest individual refusal: `code.proposal.edit` (8); next source work follows
strict edit failures. The remaining models' timeout results still need repeating;
this run does not justify a further guard or size-ceiling increment. No model
role or coding admission is selected. See [evidence](t303-trusted-model-timeouts-2026-11-11.md)
for per-task metrics, bindings, raw-log audit and independent reproduction.
The later [trusted-timeout review](../reviews/d31e2969c3ab-local-coding-trusted-model-timeouts.txt)
returns PASS for this repeat and resolves the preceding raw measurement's
missing-input blocker. No high or medium finding remains. Its low comparison
clarification is addressed above; T-303/T-306 remain open.

## Exact-source coverage development repeat, 2026-11-11

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on GPT-OSS through
Rust retrieval, the credential-owning chat API and the unchanged Rust strict
checker. **Partial path: first-attempt and within-budget task passes remain
unavailable, not zero.** No candidate tests, retries, held-out tasks, native
worktree/tool execution, apply/commit/push approvals or durable coding workflow
ran. Native admission and original grading controls remain blockers. The other
five prepared models have not been measured on this source; their resumable
campaign is pending. No model role or coding admission is selected.

Changed variables from the preceding 300/60-second primary-model repeat: complete
original lines and merged/canonical excerpts, plus the clarified exact-edit
instruction. Rendered input and output allowances change with that context.
Unchanged: 32768 context tokens, 32768 serialized selection bytes, output formula
`min(32768, 32768 - rendered_input_tokens - 32)`, total/idle 300/60 seconds,
private client/relay 310 seconds, two-task cold-load jobs 900 seconds, all existing
byte caps, temperature zero, configured seed 42 and medium reasoning. The product
proposal route remains capped at 4096 output tokens; the same private effect-free
adapter uses the larger existing chat allowance. No output ceiling was raised.
These observations do not isolate selection from instruction changes, and fixed
settings do not establish deterministic model output or task-solving improvement.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-11 | gpt-oss | 32768 | 24223–29812 | unavailable / unavailable (30 tasks) | 11/30 | 1515.472 | 1692.982 | 28.216 |

Does-not-fit tasks: 0. One attempt per task, zero retries.
Strict proposals are ungraded edits, never solved tasks. This run shows no
increase in strict proposal acceptance: 11/30 versus 12/30.
Task-solving quality remains unmeasured. Request/check time excludes
context preparation/preflight. GPU work minutes mean reserved model-job wall/60,
including verification/loading/cleanup and excluding queue/wrapper guards;
active GPU kernel time was not measured. Unambiguous backend token counters
cover 22/30 tasks: 139703 input / 146413 output.
Other counters are unknown; API usage was not requested.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.invalid` | 1 |
| `generation-refused:error:chat.transport.limit` | 8 |
| `strict-edit-refused:code.proposal.edit` | 7 |
| `strict-edit-refused:code.proposal.format` | 3 |
| `ungraded-proposal` | 11 |

Largest individual refusal: `generation-refused:error:chat.transport.limit` (8).
Use these results for the next development investigation, after any arriving
high/medium review findings. They do not authorize an unrelated limit or
bookkeeping increment. No held-out evidence or row closure is claimed.
See [evidence](t303-exact-source-coverage-2026-11-11.md) for task-level observations,
exact bindings and independent reproduction. T-303/T-306 stay open.

## Small-model exact-source development repeat, 2026-11-11

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on the prepared
`gemma4-e4b` artifact through Rust context selection, credential-owning chat and
strict edit checking. **Partial path: task first-attempt and within-budget passes
remain unavailable, not zero.** Native executable admission and missing original
grading controls still prevent the complete coding loop and task grading. No
candidate tests, retries, held-out tasks, native tools, worktree edits, approvals,
apply, commit or push ran. Strict proposals are ungraded edits, never solved tasks.
No role selection, coding admission or row closure follows.
Selection receives only each task statement and its editable base file; full
repository analysis and retrieval are not measured. The native Git artifact
refuses its existing root-ownership admission in this lane; original grading
control receipts remain absent. Neither boundary is bypassed or reconstructed.

Changed variables versus this artifact's November 10/11 measurement: complete
source lines and merged/canonical excerpts with the exact-edit instruction;
total/idle 300/60 s (was 120/30); private client/relay 310 s (was 130); two-task
cold-load jobs 900 s (was 600). Context remains 32768 tokens and selection 32768
bytes. Output allowance is `min(32768, 32768 - rendered_input_tokens - 32)`,
including reasoning. All existing byte caps and strict refusals remain. The same
private effect-free adapter uses the chat allowance; the product proposal route
remains capped at 4096 output tokens. Context and time changes are combined;
these results do not isolate which variable changed an outcome. 32K is the largest
prepared context tested, not an exhaustive hardware-limit search. Runtime, weights,
temperature zero, seed 42 and default family settings match the preceding campaign.
All 30 selected contexts match the preceding current-source GPT-OSS run byte for
byte; backend tokenization and output allowances differ by model.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-11 | gemma4-e4b | 32768 | 22998–29579 | unavailable / unavailable (30 tasks) | 10/30 | 1592.324 | 1733.627 | 28.894 |

Does-not-fit tasks: 0. One attempt per task, zero retries and no grade.
Transport timeouts: 0 (the November 10/11 run at 120/30 s had 1).
The November 10/11 run at 120/30 s had 10/30 strict ungraded proposals; neither count
is a task pass rate. GPU work minutes mean reserved model-job wall/60, including
verification, loading and cleanup; queue and wrapper checks are excluded. Active
GPU kernel time was not measured. Request/check time excludes context preparation
and preflight. Every pair used both shared reservations and released them on
completion. No weights were downloaded or modified.

| Outcome / cause | Tasks |
| --- | --- |
| `strict-edit-refused:code.proposal.edit` | 3 |
| `strict-edit-refused:code.proposal.format` | 17 |
| `ungraded-proposal` | 10 |

Unambiguous backend counters cover 30/30 tasks:
214722 input / 142033 output tokens.
API usage was not requested; other counters remain unknown.
Largest individual refusal: `strict-edit-refused:code.proposal.format` (17).
Four other models still need complete repeats. Muse completed only its first two
tasks before a controller-only stop between jobs; those retained attempts are
unfinished campaign progress, not a 30-task measurement, and must not be silently
repeated. Scheduling the smaller model first changed no model input or budget.
The complete six-model repeat remains unfinished; no unrelated source work is
justified by this subset. T-303/T-306 remain open. See
[evidence](t303-small-model-measurement-2026-11-11.md) for exact bindings,
per-task observations and reproduction.

## Muse exact-source development repeat, 2026-11-11

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on the
prepared `muse` artifact through Rust context selection, credential-owning chat
and strict edit checking. **Partial path: task first-attempt and within-budget
passes remain unavailable, not zero.** Native Git executable admission and missing
original grading controls prevent native execution and task grading. No native
tools, worktree edits, candidate tests, test feedback, retries, approvals, apply,
commit, push or held-out tasks ran. Strict proposals are ungraded edits, never
solved tasks. Selection receives only the statement and editable base file;
full repository analysis and retrieval are not measured. Neither native trust
nor original control admission is bypassed or reconstructed.

Changed variables versus this artifact's November 10/11 measurement: complete
source lines and merged/canonical excerpts with the exact-edit instruction;
total/idle 300/60 s (was 120/30), private client/relay 310 s (was 130) and two-task
cold-load jobs 900 s (was 600). Context remains 32768 tokens and selection 32768
bytes. Output allowance is `min(32768, 32768 - rendered_input_tokens - 32)`,
including reasoning. The private effect-free adapter uses the existing chat
allowance; the product proposal route stays capped at 4096 output tokens.
All existing byte caps and strict refusals remain. Combined changes do not
isolate which variable changed an outcome. 32K is the largest prepared context
tested, not an exhaustive hardware-limit search. Runtime, read-only weights,
temperature zero, seed 42 and family template settings match the preceding
campaign. All 30 contexts match the current-source primary-model run byte for
byte; tokenization and output allowances differ by model.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-11 | muse | 32768 | 24394–29889 | unavailable / unavailable (30 tasks) | 18/30 | 4945.146 | 5674.078 | 94.568 |

Does-not-fit tasks: 0. One attempt per task, zero retries and no grade.
Transport timeouts: 2; the November 10/11 run at 120/30 s had 18.
That November 10/11 run had 10/30 strict ungraded proposals; neither count is a task
pass rate. GPU work minutes mean reserved model-job wall/60, including
verification, loading and cleanup, excluding queue and wrapper checks.
Active GPU kernel time was not measured. Request/check time excludes context
preparation and preflight. All 15 two-task jobs used both shared reservations
and released them after completion. The first two attempts predate the smaller
model's repeat and were retained unchanged; the controller resumed from task 3,
without rerunning or dropping earlier attempts. No model was downloaded.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 1 |
| `generation-refused:error:chat.transport.timeout` | 2 |
| `strict-edit-refused:code.proposal.edit` | 6 |
| `strict-edit-refused:code.proposal.format` | 3 |
| `ungraded-proposal` | 18 |

Unambiguous backend counters cover 28/30 tasks:
171088 input / 184996 output tokens.
API usage was not requested; other counters remain unknown.
Largest individual refusal: `strict-edit-refused:code.proposal.edit` (6).
Three other prepared models still need complete repeats before model comparison
or unrelated source work. T-303/T-306 remain open without role selection,
new rows/sub-rows, completion ticks or native coding/release claims. See
[evidence](t303-muse-measurement-2026-11-11.md) for exact bindings and per-task
observations. These measurements do not establish task-solving improvement.

## 26B A4B exact-source development repeat, 2026-11-11

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on the
prepared `gemma4-26b-a4b` artifact through Rust context selection, credential-owning chat
and strict edit checking. **Partial path: task first-attempt and within-budget
passes remain unavailable, not zero.** Native Git executable admission and missing
original grading controls prevent native execution and task grading. No native
tools, worktree edits, candidate tests, test feedback, retries, approvals, apply,
commit, push or held-out tasks ran. Strict proposals are ungraded edits, never
solved tasks. Selection receives only the statement and editable base file;
full repository analysis and retrieval are not measured. Neither native trust
nor original control admission is bypassed or reconstructed.

Changed variables versus this artifact's November 10/11 measurement: complete
source lines and merged/canonical excerpts with the exact-edit instruction;
total/idle 300/60 s (was 120/30), private client/relay 310 s (was 130) and two-task
cold-load jobs 900 s (was 600). Context remains 32768 tokens and selection 32768
bytes. Output allowance is `min(32768, 32768 - rendered_input_tokens - 32)`,
including reasoning. The private effect-free adapter uses the existing chat
allowance; the product proposal route stays capped at 4096 output tokens.
All existing byte caps and strict refusals remain. Combined changes do not
isolate which variable changed an outcome. 32K is the largest prepared context
tested, not an exhaustive hardware-limit search. Runtime, read-only weights,
temperature zero, seed 42 and family template settings match the preceding
campaign. All 30 contexts match the current-source primary-model run byte for
byte; tokenization and output allowances differ by model.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-11 | gemma4-26b-a4b | 32768 | 22998–29579 | unavailable / unavailable (30 tasks) | 0/30 | 4275.841 | 4800.543 | 80.009 |

Does-not-fit tasks: 0. One attempt per task, zero retries and no grade.
Complete answers reaching strict checking: 1/30.
Transport timeouts: 0; the November 10/11 run at 120/30 s had 29.
That November 10/11 run had 0/30 strict ungraded proposals; neither count is a task
pass rate. GPU work minutes mean reserved model-job wall/60, including
verification, loading and cleanup, excluding queue and wrapper checks.
Active GPU kernel time was not measured. Request/check time excludes context
preparation and preflight. All 15 two-task jobs used both shared reservations
and released them after completion. The first two attempts were retained
unchanged after a controller-only stop between jobs. An active pair on another model finished normally and remains
partial progress; the resumed controller completed this model from task 3
without rerunning or dropping earlier attempts. No model was downloaded.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.limit` | 29 |
| `strict-edit-refused:code.proposal.format` | 1 |

Unambiguous backend counters cover 1/30 tasks:
5329 input / 10262 output tokens.
API usage was not requested; other counters remain unknown.
Largest individual refusal: `generation-refused:error:chat.transport.limit` (29).
Two other prepared models still need complete repeats before model comparison
or unrelated source work. T-303/T-306 remain open without role selection,
new rows/sub-rows, completion ticks or native coding/release claims. See
[evidence](t303-a4b-measurement-2026-11-11.md) for exact bindings and per-task
observations. These measurements do not establish task-solving improvement.

## 31B current-source development measurement — 2026-11-11

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on the
prepared `gemma4-31b` artifact through Rust context selection, credential-owning chat
and strict edit checking. **Partial path: task first-attempt and within-budget
passes remain unavailable, not zero.** Native Git executable admission and missing
original grading controls prevent native execution and task grading. No native
tools, worktree edits, candidate tests, test feedback, retries, approvals, apply,
commit, push or held-out tasks ran. Strict proposals are ungraded edits, never
solved tasks. Selection receives only the statement and editable base file;
full repository analysis and retrieval are not measured. Neither native trust
nor original control admission is bypassed or reconstructed.

Changed variables versus this artifact's November 10/11 measurement: complete
source lines and merged/canonical excerpts with the exact-edit instruction;
total/idle 300/60 s (was 120/30), private client/relay 310 s (was 130) and two-task
cold-load jobs 900 s (was 600). Context remains 32768 tokens and selection 32768
bytes. Output allowance is `min(32768, 32768 - rendered_input_tokens - 32)`,
including reasoning. The private effect-free adapter uses the existing chat
allowance; the product proposal route stays capped at 4096 output tokens.
The unchanged ceilings are 4 MiB stream body, 5 MiB wire and 1 MiB text;
they can stop generation before its token allowance. All strict refusals remain.
Combined changes do not
isolate which variable changed an outcome. 32K is the largest prepared context
tested, not an exhaustive hardware-limit search. Runtime, read-only weights,
temperature zero, seed 42 and family template settings match the preceding
campaign. All 30 contexts match the current-source primary-model run byte for
byte; tokenization and output allowances differ by model.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-11 | gemma4-31b | 32768 | 22998–29579 | unavailable / unavailable (30 tasks) | 0/30 | 8303.782 | 8853.704 | 147.562 |

Does-not-fit tasks: 0. One attempt per task, zero retries and no grade.
Complete answers reaching strict checking: 8/30.
Transport timeouts: 22.
The earlier November 10/11 run at 120/30 s had 29 transport timeouts and
0/30 strict ungraded proposals. These combined changes do not isolate cause;
proposal counts are not task pass rates.
GPU work minutes mean reserved model-job wall/60, including
verification, loading and cleanup, excluding queue and wrapper checks.
Active GPU kernel time was not measured. Request/check time excludes context
preparation and preflight. All 15 two-task jobs used both shared reservations
and released them after completion. The first two attempts were retained
unchanged from the earlier campaign controller. This session resumed at task 3,
without rerunning or dropping either earlier attempt. No model was downloaded.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 22 |
| `strict-edit-refused:code.proposal.format` | 8 |

All eight complete answers wrap JSON in Markdown code fences, contrary to the
existing “No Markdown” instruction; strict checking correctly refuses them.

Unambiguous backend counters cover 8/30 tasks:
54642 input / 55081 output tokens.
API usage was not requested; other counters remain unknown.
Largest individual refusal: `generation-refused:error:chat.transport.timeout` (22).
One other prepared model still needs its complete repeat before model comparison
or unrelated source work. T-303/T-306 remain open without role selection,
new rows/sub-rows, completion ticks or native coding/release claims.
See [evidence](t303-31b-measurement-2026-11-11.md) for exact bindings and per-task
observations. These measurements do not establish task-solving improvement.

## 27B current-source development measurement — 2026-11-12

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on
`qwen35-27b`, the retained label for the existing read-only artifact. The campaign
spans November 11–12 UTC; the date below records completion. **Partial path:
Rust source selection, credential-owning chat and strict edit checking. Task
first-attempt and within-budget passes remain unavailable, not zero.** Native
Git executable ownership and missing original grading controls still prevent
native execution and task grading. No candidate tests, test feedback, retries,
approvals, apply, commit, push or held-out tasks ran. Selection receives only the
statement and editable base file; full repository analysis and retrieval are not
measured. No admission control was bypassed or reconstructed. Strict proposals
are ungraded edits, never solved tasks.

Changed variables versus this artifact's November 10/11 run: complete source
lines and merged excerpts with the exact-edit instruction; total/idle 300/60 s
(was 120/30), client/relay 310 s (was 130), and two-task jobs 900 s (was 600).
Context remains 32768 tokens and selection 32768 bytes. Output allowance is
`min(32768, 32768 - rendered_input_tokens - 32)`, including reasoning. The private
effect-free adapter uses the existing chat allowance; the product proposal route
remains capped at 4096 output tokens. The unchanged 4 MiB stream-body, 5 MiB wire
and 1 MiB text ceilings can stop generation before its token allowance. Strict
refusals remain. Combined changes do not isolate causes. 32K is the largest
prepared context tested, not an exhaustive hardware-limit search. Runtime,
weights, temperature zero, seed 42 and template settings are unchanged. All 30
contexts and request messages match the current-source primary run byte for byte;
tokenization and output allowances differ by artifact.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | qwen35-27b | 32768 | 24114–29831 | unavailable / unavailable (30 tasks) | 8/30 | 7666.469 | 8072.425 | 134.540 |

Does-not-fit tasks: 0. One attempt per task, zero retries and no grade.
Complete answers reaching strict checking: 8/30. GPU work minutes mean
reserved model-job wall/60, including verification, loading and cleanup, excluding
queue and wrapper checks. Active GPU kernel time was not measured. Request/check
time excludes preparation and preflight. All 15 two-task jobs completed and
released both shared reservations. The first pair remains unchanged; this session
resumed at task 3 without rerunning or dropping an attempt. No model was downloaded.
No model or API process remains.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 1 |
| `generation-refused:error:chat.transport.timeout` | 21 |
| `ungraded-proposal` | 8 |

Unambiguous backend counters cover 9/30 tasks:
60348 input / 52900 output tokens.
One counter-covered attempt has an incomplete API response and never reaches
strict checking; only eight complete responses are checked.
API usage was not requested; other counters remain unknown.
Largest individual refusal: `generation-refused:error:chat.transport.timeout` (21).
All six prepared backends now have complete current-source 30-task partial-path
measurements. This establishes neither task-solving improvement nor model roles.
T-303/T-306 remain open without new rows, sub-rows or completion ticks. See
[evidence](t303-qwen-measurement-2026-11-12.md) for bindings and per-task observations.

### Observed causes across the completed six-model campaign

The same 30 development tasks contribute 180 one-attempt observations at source
`42612ca9787f07e4df79402e6d92f39f578e5c67`. Each model's entry reports its own budgets, timing,
counters and limitations. This summary covers only the available partial path;
no task pass rate, retry result, held-out score or role selection is inferred.

| Outcome / cause | Attempts |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 2 |
| `generation-refused:error:chat.response.invalid` | 1 |
| `generation-refused:error:chat.transport.limit` | 37 |
| `generation-refused:error:chat.transport.timeout` | 45 |
| `strict-edit-refused:code.proposal.edit` | 16 |
| `strict-edit-refused:code.proposal.format` | 32 |
| `ungraded-proposal` | 47 |

Largest observed failure: `generation-refused:error:chat.transport.timeout`
(45). Under measure-first, the next
source work targets that generation failure within existing safety and resource
boundaries. Further limit, deadline, cancellation or bookkeeping work stays in
T-309 backlog unless an open high/medium finding or the row's done-when requires
it. Native execution and original grading-control admission remain blocked;
these observations do not replace end-to-end task measurements.

## Thinking-disabled development experiment — 2026-11-12

All 30 frozen development tasks ran once on each of two controlled model
profiles at product source `42612ca9787f07e4df79402e6d92f39f578e5c67`.
The measurement checkpoint is `f619ec3cdb681bc399bb779285f837332f76d3b2`;
the intervening commits change documentation only. **Partial path: Rust source
selection, credential-owning chat and strict edit checking. First-attempt and
within-budget task passes remain unavailable, not zero.** Native Git executable
ownership and missing original grading controls still block execution and grading.
No candidate tests, feedback, retries, approvals, apply, commit, push or held-out
tasks ran. Selection receives only the statement and editable base file; full
repository analysis and retrieval are not measured. No control was bypassed or
reconstructed. A strict proposal is an ungraded edit, never a solved task.

Changed variable versus each profile's previous complete development measurement:
`chat_template_kwargs` changes from `{}` to `{"enable_thinking":false}`.
This is a candidate experiment; no product profile or model role is adopted.
All 60 contexts and request bodies match their corresponding previous run except
the output allowance computed from template-rendered input. Rendering and the
resulting output reservation may change. Model/runtime bytes, decoding
(temperature zero, seed 42), prompts, strict checker and safety remain unchanged.
Context remains 32768 tokens, source-selection ceiling 32768 bytes, and output
allowance `min(32768, 32768 - rendered_input_tokens - 32)`, including reasoning.
32K is the largest prepared context tested, not an exhaustive hardware-limit
search. The effect-free adapter uses the existing chat allowance; the product
proposal route remains capped at 4096 output tokens. Total/idle time remains
300/60 s, client/relay 310 s, and each two-task reservation 900 s. Unchanged
4 MiB stream-body, 5 MiB wire and 1 MiB text ceilings can stop generation before
the token allowance. The earlier timeout audit did not prove reasoning was the
cause; this single repeat does not establish task-solving quality or generalization.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | qwen35-27b | 32768 | 24150–29867 | unavailable / unavailable (30 tasks) | 30/30 | 1132.698 | 1462.580 | 24.376 |
| 2026-11-12 | gemma4-31b | 32768 | 22996–29577 | unavailable / unavailable (30 tasks) | 0/30 | 1814.415 | 2337.099 | 38.952 |

One attempt per task and zero retries. GPU work minutes mean reserved model-job
wall/60, including verification, loading and cleanup, excluding queues and wrapper
checks. Active GPU kernel time was not measured. Request/check time excludes
preparation and preflight. All 30 two-task reservations completed and released
both shared slots. No model was downloaded; no model or API process from this
experiment remains.
API usage was not requested; unambiguous backend counters are reported below.

Largest individual failure in these 60 observations: `strict-edit-refused:code.proposal.format` (30).
The next generation/proposal work follows this measured failure; further limits,
deadlines, cancellation and bookkeeping remain T-309 backlog unless an open
high/medium finding or the row's own completion requirement needs them.
T-303/T-306 remain open, without new rows, sub-rows, model-role selection or ticks.

`qwen35-27b` causes: `ungraded-proposal`: 30.
Unambiguous counters cover 30/30 tasks: 188110 input / 35780 output tokens. API usage was not requested.

`gemma4-31b` causes: `strict-edit-refused:code.proposal.format`: 30.
Unambiguous counters cover 30/30 tasks: 214782 input / 47482 output tokens. API usage was not requested.

See [evidence](t303-thinking-profile-measurement-2026-11-12.md) for per-task results, bindings and reproduction.

Compared with the corresponding prior runs, timeouts change from 21/30 to 0/30
on 27B and from 22/30 to 0/30 on 31B. Strict ungraded proposals change from
8/30 to 30/30 and from 0/30 to 0/30, respectively. These are generation and
format observations; task correctness remains unknown. No model role or product
profile is adopted from this comparison.

## Raw JSON generation development measurement — 2026-11-12

Source: `a99fe467c72e2625022b639d8d38b72e845f612a`. All 30 frozen development tasks ran once on `gemma4-31b`. **Partial path: Rust source selection, credential-owning chat and strict edit checking. First-attempt and within-budget task passes remain unavailable, not zero.** No candidate tests, feedback, retries, approvals, apply, commit, push or held-out task ran. Native execution remains unresolved and the expected original grading-controls receipt is absent. Selection receives only the statement and editable frozen base file; full repository analysis and retrieval are not measured. No control was bypassed or reconstructed.

Changed variable versus this profile's preceding run: the fixed generation instruction now gives an original JSON syntax example and explicit raw-object and string-escaping requirements. Its increased size can change excerpt coverage under the unchanged 32768-byte message ceiling. The profile retains `enable_thinking:false`; runtime, weights, temperature zero, seed 42, 32768-token context, output formula `min(32768, 32768 - rendered_input_tokens - 32)`, 300/60-second total/idle, 310-second relay/client, byte caps, strict checking and two-task 900-second reservations remain unchanged. 32K is the largest prepared context tested, not an exhaustive hardware maximum. The freshly built source helper uses the unchanged reviewed chat-only API binary. The effect-free adapter uses the existing chat allowance; the product proposal route still caps output at 4096.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | gemma4-31b | 32768 | 23002–29511 | unavailable / unavailable (30 tasks) | 4/30 | 1653.191 | 2154.301 | 35.905 |

GPU work minutes mean reserved model-job wall/60, including verification, loading and cleanup, excluding queue and wrapper checks. Active GPU kernel time was not measured. Request/check time excludes preparation and preflight.

| Outcome / cause | Tasks |
| --- | --- |
| `strict-edit-refused:code.proposal.edit` | 1 |
| `strict-edit-refused:code.proposal.format` | 25 |
| `ungraded-proposal` | 4 |

Does-not-fit tasks: 0. One attempt per task, zero retries. No model or API process remains. All 15 two-task reservations completed and released both shared slots.

Unambiguous backend counters cover 30/30 tasks: 215953 input / 38865 output tokens. Other counters remain unknown; API usage was not requested.

Compared with the prior thinking-disabled 31B run, strict ungraded proposals change from 0/30 to 4/30. All 30 task statements and source outlines are unchanged; 19/30 user payloads and 19/30 excerpt selections are byte-identical. The longer instruction changes the remaining selections under the same message budget. Complete launch profiles match except the private state root. These observations do not isolate wording from changed source delivery or establish task correctness, generalization or a model role.

Largest individual remaining refusal is `strict-edit-refused:code.proposal.format` (25). Syntax-only inspection classifies 19 of those answers as JSON code fences and six as other JSON-format failures; none exceeds the existing 64 KiB answer ceiling. This does not normalize a response, execute a candidate or change acceptance. No further limit, deadline, cancellation or bookkeeping feature is justified by these observations.

Only this prepared profile has completed the current-source repeat. The other five prepared profiles (qwen35-27b, gpt-oss, muse, gemma4-e4b and gemma4-26b-a4b) remain pending before further source work except high/medium review findings. No product profile or model role is adopted. T-303/T-306 remain open, without new rows, sub-rows or completion ticks.

See [evidence](t303-raw-json-generation-2026-11-12.md) for all task observations, bindings and reproduction.

## Correction to raw JSON fence observations — 2026-11-12

Independent review of `52c3f10029e0dfb786993f26e4c45b586ff5ffba` found medium M1 in the syntax subclassification above. **All 25 format refusals begin with a JSON Markdown fence**: 19 close it on its own line and six join the closing fence to the last line. The earlier classifier conflated an opening fence with standalone closing placement; its six “other JSON-format failures” are also fenced. This corrects the fence count to 25/30 answers, versus 30/30 in the prior thinking-disabled 31B run, without claiming the payloads are valid. Strict ungraded proposals (4/30), refusal counts, budgets, tokens and times are unchanged. No inference, acceptance, candidate test, retry or held-out run occurs for this correction. The original sealed artifacts stay unchanged; the additive correction receipt and revised binding are in the [evidence](t303-raw-json-generation-2026-11-12.md#review-correction--2026-11-12). Independent re-review is required; T-303/T-306 remain open.

## Raw JSON instruction 27B development repeat — 2026-11-12

Source: `a99fe467c72e2625022b639d8d38b72e845f612a`. All 30 frozen development tasks ran once on `qwen35-27b`. **Partial path: Rust source selection, credential-owning chat and strict edit checking. First-attempt and within-budget task passes remain unavailable, not zero.** No candidate tests, feedback, retries, approvals, apply, commit, push or held-out tasks ran. Native executable trust and the absence of the expected original grading-controls receipt remain unresolved; no control is bypassed or reconstructed. Selection receives only the statement and editable frozen base file; full repository analysis and retrieval are not measured. A strict proposal is an ungraded edit, never a solved task.

This is the second prepared model measured after the raw-JSON instruction change, with no intervening product change. Versus this profile's preceding complete repeat, the fixed generation instruction adds an original JSON syntax example and raw-object and string-escaping requirements. Source selection and strict checking stay unchanged. The profile retains `enable_thinking:false`; runtime and model bytes, temperature zero, seed 42, 32768-token context, 32768-byte source selection, output formula `min(32768, 32768 - rendered_input_tokens - 32)`, 300/60-second total/idle, 310-second relay/client and byte ceilings are unchanged. 32K is the largest prepared context tested, not an exhaustive hardware maximum. The source-built effect-free helper uses the unchanged reviewed chat-only API binary. The product proposal route still caps output at 4096; this measurement adapter uses the existing chat allowance.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | qwen35-27b | 32768 | 24137–29804 | unavailable / unavailable (30 tasks) | 29/30 | 1385.953 | 1740.182 | 29.003 |

GPU work minutes mean reserved model-job wall/60, including verification, loading and cleanup, excluding queues and wrapper checks. Active GPU kernel time was not measured. Request/check time excludes preparation and preflight.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 1 |
| `ungraded-proposal` | 29 |

Does-not-fit tasks: 0. One attempt per task, zero retries. All 15 two-task reservations use both required wrappers, complete and release the shared slots; their model and API processes are reaped and temporary model keys/sockets removed. No model was downloaded.

Unambiguous backend counters cover 29/30 tasks: 186402 input / 34920 output tokens. Other counters remain unknown; API usage was not requested.

Compared with the prior thinking-disabled 27B run, strict ungraded proposals change from 30/30 to 29/30. All 30 task statements and source outlines are unchanged; 19/30 user payloads and 19/30 excerpt selections are byte-identical. Complete launch profiles match except the private state root. The longer instruction can change selections and rendered output reservation under the same message budget. This comparison does not isolate wording from source delivery or establish task correctness, generalization or a model role.

Largest individual refusal in this repeat: generation-refused:error:chat.transport.timeout (1). Across the two current-source model repeats, the 31B profile's 25 format refusals remain the largest individual refusal cause. Four prepared current-source repeats (gpt-oss, muse, gemma4-e4b and gemma4-26b-a4b) remain pending before further source work except high/medium review findings. No profile or model role is adopted. Further limit, deadline, cancellation and bookkeeping features remain T-309 backlog unless an open high/medium finding or the row's own completion requirement needs them. T-303/T-306 remain open, without new rows, sub-rows or completion ticks.

See [evidence](t303-raw-json-qwen-2026-11-12.md) for per-task observations, frozen bindings and reproduction.

## Review clarification to the 27B repeat — 2026-11-12

The [independent core review](../reviews/086c321ec5b0-raw-json-qwen-measurement.txt)
returns **Verdict: PASS** for exact commit
`086c321ec5b0f2b13ab261cf298d2a9dd48c6a2c`, with no high or medium finding.
For dev-004, continuous repetitive model output reached the existing 300-second
total limit. Its `chat.transport.timeout` terminal code does not establish an
infrastructure fault. The one recorded refusal and all measurement numbers
remain unchanged. No new model run or task grade is claimed by this clarification.
T-303/T-306 remain open.

## Raw JSON instruction GPT-OSS development repeat — 2026-11-12

Source: `a99fe467c72e2625022b639d8d38b72e845f612a`. All 30 frozen development tasks ran once on `gpt-oss`. **Partial path: Rust source selection, credential-owning chat and strict edit checking. First-attempt and within-budget task passes remain unavailable, not zero.** No candidate tests, feedback, retries, approvals, apply, commit, push or held-out tasks ran. Native executable trust and the absence of the expected original grading-controls receipt remain unresolved; no control is bypassed or reconstructed. Selection receives only the statement and editable frozen base file; full repository analysis and retrieval are not measured. A strict proposal is an ungraded edit, never a solved task.

This is the third prepared model measured after the raw-JSON instruction change, with no intervening product change. Versus this profile's preceding complete repeat, the fixed generation instruction adds an original JSON syntax example and raw-object and string-escaping requirements. Source selection and strict checking stay unchanged. The profile retains `reasoning_effort:medium` in its template kwargs; runtime and model bytes, temperature zero, seed 42, 32768-token context, 32768-byte source selection, output formula `min(32768, 32768 - rendered_input_tokens - 32)`, 300/60-second total/idle, 310-second relay/client and byte ceilings are unchanged. 32K is the largest prepared context tested, not an exhaustive hardware maximum. The source-built effect-free helper uses the unchanged reviewed chat-only API binary. The product proposal route still caps output at 4096; this measurement adapter uses the existing chat allowance.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | gpt-oss | 32768 | 24219–29748 | unavailable / unavailable (30 tasks) | 3/30 | 1902.747 | 2075.168 | 34.586 |

GPU work minutes mean reserved model-job wall/60, including verification, loading and cleanup, excluding queues and wrapper checks. Active GPU kernel time was not measured. Request/check time excludes preparation and preflight.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.limit` | 19 |
| `strict-edit-refused:code.proposal.edit` | 6 |
| `strict-edit-refused:code.proposal.format` | 2 |
| `ungraded-proposal` | 3 |

Does-not-fit tasks: 0. One attempt per task, zero retries. All 15 two-task reservations use both required wrappers, complete and release the shared slots; their model and API processes are reaped and temporary model keys/sockets removed. No model was downloaded.

Unambiguous backend counters cover 11/30 tasks: 75060 input / 62786 output tokens. Other counters remain unknown; API usage was not requested.

Strict ungraded proposals change from the preceding published full GPT-OSS measurement's 11/30 to 3/30 in this repeat. This new instruction repeat reports current-source observations, without isolating the instruction from changes to excerpt delivery or rendered output reservation. It establishes neither task correctness, generalization nor a model role. No new private comparison receipt is claimed.

Largest individual refusal in this repeat: generation-refused:error:chat.transport.limit (19). Across the three current-source model repeats, the 31B profile's 25 format refusals remain the largest individual refusal cause. Three prepared current-source repeats (muse, gemma4-e4b and gemma4-26b-a4b) remain pending before further source work except high/medium review findings. No profile or model role is adopted. Further limit, deadline, cancellation and bookkeeping features remain T-309 backlog unless an open high/medium finding or the row's own completion requirement needs them. T-303/T-306 remain open, without new rows, sub-rows or completion ticks.

See [evidence](t303-raw-json-gpt-2026-11-12.md) for per-task observations, frozen bindings and reproduction.

## Independent GPT-OSS review clarification — 2026-11-12

The [independent core review](../reviews/d252fce24ad6-raw-json-gpt-measurement.txt)
returns **Verdict: PASS** for exact commit
`d252fce24ad6bb09aa216d324d2199ef98b4b46e`, with no high or medium finding.
Its low L1 clarifies the effective output limit: all 19 size-limit refusals
reported that the server response exceeded a size limit. Their backend stop totals minus tokenizer preflight
place the cut at about 16,607–16,721 generated tokens, with `truncated = 0`;
18 emitted no visible text and one emitted 6,233 visible text bytes. The existing
4 MiB stream-body ceiling bounded these tasks before the published token
allowance. This is a stream-size limit, not evidence of token exhaustion or an
infrastructure fault. Counts and earlier measurement entries remain unchanged.
Changing that ceiling remains T-309 backlog unless a high/medium finding or a
row's completion requirement needs it. Low L2 retains the private-archive
hygiene backlog. No row closes.

## Raw JSON instruction Muse development repeat — 2026-11-12

Source: `a99fe467c72e2625022b639d8d38b72e845f612a`. All 30 frozen development tasks ran once on `muse`. **Partial path: Rust source selection, credential-owning chat and strict edit checking. First-attempt and within-budget task passes remain unavailable, not zero.** No candidate tests, feedback, retries, approvals, apply, commit, push or held-out tasks ran. Native executable trust and the absence of the expected original grading-controls receipt remain unresolved; no control is bypassed or reconstructed. Selection receives only the statement and editable frozen base file; full repository analysis and retrieval are not measured. A strict proposal is an ungraded edit, never a solved task.

This is the fourth prepared model measured after the raw-JSON instruction change, with no intervening product change. Versus this profile's preceding complete repeat, the fixed generation instruction adds an original JSON syntax example and raw-object and string-escaping requirements. Source selection and strict checking stay unchanged. The profile retains `reasoning_strength:medium` in its template kwargs; runtime and model bytes, temperature zero, seed 42, 32768-token context, 32768-byte source selection, output formula `min(32768, 32768 - rendered_input_tokens - 32)`, 300/60-second total/idle, 310-second relay/client and byte ceilings are unchanged. 32K is the largest prepared context tested, not an exhaustive hardware maximum. The source-built effect-free helper uses the unchanged reviewed chat-only API binary. The product proposal route still caps output at 4096; this measurement adapter uses the existing chat allowance.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | muse | 32768 | 24395–29826 | unavailable / unavailable (30 tasks) | 19/30 | 4711.068 | 5284.175 | 88.070 |

GPU work minutes mean reserved model-job wall/60, including verification, loading and cleanup, excluding queues and wrapper checks. Active GPU kernel time was not measured. Request/check time excludes preparation and preflight.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 4 |
| `strict-edit-refused:code.proposal.edit` | 3 |
| `strict-edit-refused:code.proposal.format` | 4 |
| `ungraded-proposal` | 19 |

Does-not-fit tasks: 0. One attempt per task, zero retries. All 15 two-task reservations use both required wrappers, complete and release the shared slots; their model and API processes are reaped and temporary model keys/sockets removed. No model was downloaded.

Unambiguous backend counters cover 26/30 tasks: 166355 input / 150086 output tokens. Other counters remain unknown; API usage was not requested.

Strict ungraded proposals change from the preceding published full Muse measurement's 18/30 to 19/30 in this repeat. This new instruction repeat reports current-source observations, without isolating the instruction from changes to excerpt delivery or rendered output reservation. It establishes neither task correctness, generalization nor a model role. No new private comparison receipt is claimed.

Largest individual refusal count in this repeat: 4; tied causes: generation-refused:error:chat.transport.timeout, strict-edit-refused:code.proposal.format. Across the four current-source model repeats, the 31B profile's 25 format refusals remain the largest individual refusal cause. Two prepared current-source repeats (gemma4-e4b and gemma4-26b-a4b) remain pending before further source work except high/medium review findings. No profile or model role is adopted. Further limit, deadline, cancellation and bookkeeping features remain T-309 backlog unless an open high/medium finding or the row's own completion requirement needs them. T-303/T-306 remain open, without new rows, sub-rows or completion ticks.

See [evidence](t303-raw-json-muse-2026-11-12.md) for per-task observations, frozen bindings and reproduction.

## Independent Muse review clarification — 2026-11-12

The [independent Muse measurement review](../reviews/fc035e714fdb-raw-json-muse-measurement.txt)
returns **Verdict: PASS** for exact commit
`fc035e714fdb8f30aba3e77f3d7d7c95e4cb4f7f`, with no high or medium finding.
Low L1 clarifies the effective output limit: the 300-second total timeout cut
four attempts at about 12,955–13,531 generated tokens, below their published
token allowances. All four backend stops report cancellation without context
truncation. At the observed rate, the time limit bounds this profile's effective
output near 13.0–13.5K tokens; the token allowance alone overstates its room.
The builder rechecked all four retained stop records and visible text sizes.
Counts and earlier entries remain unchanged. A timeout change and low L2's
private-archive hygiene remain T-309 backlog. No row closes.

## Raw JSON instruction E4B development repeat — 2026-11-12

Source: `a99fe467c72e2625022b639d8d38b72e845f612a`. All 30 frozen development tasks ran once on `gemma4-e4b`. **Partial path: Rust source selection, credential-owning chat and strict edit checking. First-attempt and within-budget task passes remain unavailable, not zero.** No candidate tests, feedback, retries, approvals, apply, commit, push or held-out tasks ran. Native executable trust and the absence of the expected original grading-controls receipt remain unresolved; no control is bypassed or reconstructed. Selection receives only the statement and editable frozen base file; full repository analysis and retrieval are not measured. A strict proposal is an ungraded edit, never a solved task.

This is the fifth prepared model measured after the raw-JSON instruction change, with no intervening product change. Versus this profile's preceding complete repeat, the fixed generation instruction adds an original JSON syntax example and raw-object and string-escaping requirements. Source selection and strict checking stay unchanged. The profile retains empty template kwargs; runtime and model bytes, temperature zero, seed 42, 32768-token context, 32768-byte source selection, output formula `min(32768, 32768 - rendered_input_tokens - 32)`, 300/60-second total/idle, 310-second relay/client and byte ceilings are unchanged. 32K is the largest prepared context tested, not an exhaustive hardware maximum. The source-built effect-free helper uses the unchanged reviewed chat-only API binary. The product proposal route still caps output at 4096; this measurement adapter uses the existing chat allowance.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | gemma4-e4b | 32768 | 23004–29513 | unavailable / unavailable (30 tasks) | 13/30 | 1372.293 | 1495.892 | 24.932 |

GPU work minutes mean reserved model-job wall/60, including verification, loading and cleanup, excluding queues and wrapper checks. Active GPU kernel time was not measured. Request/check time excludes preparation and preflight.

| Outcome / cause | Tasks |
| --- | --- |
| `strict-edit-refused:code.proposal.edit` | 2 |
| `strict-edit-refused:code.proposal.format` | 15 |
| `ungraded-proposal` | 13 |

Does-not-fit tasks: 0. One attempt per task, zero retries. All 15 two-task reservations use both required wrappers, complete and release the shared slots; their model and API processes are reaped and temporary model keys/sockets removed. No model was downloaded.

Unambiguous backend counters cover 30/30 tasks: 215893 input / 138484 output tokens. Other counters remain unknown; API usage was not requested.

Strict ungraded proposals change from the preceding published full E4B measurement's 10/30 to 13/30 in this repeat. This new instruction repeat reports current-source observations, without isolating the instruction from changes to excerpt delivery or rendered output reservation. It establishes neither task correctness, generalization nor a model role. No new private comparison receipt is claimed.

Largest individual refusal count in this repeat: 15; tied causes: strict-edit-refused:code.proposal.format. Across the five current-source model repeats, the 31B profile's 25 format refusals remain the largest individual refusal cause. One prepared current-source repeat (gemma4-26b-a4b) remains pending before further source work except high/medium review findings. No profile or model role is adopted. Further limit, deadline, cancellation and bookkeeping features remain T-309 backlog unless an open high/medium finding or the row's own completion requirement needs them. T-303/T-306 remain open, without new rows, sub-rows or completion ticks.

See [evidence](t303-raw-json-e4b-2026-11-12.md) for per-task observations, frozen bindings and reproduction.

## Independent E4B review clarification — 2026-11-12

The [independent E4B measurement review](../reviews/ed6dd3f62ae2-raw-json-e4b-measurement.txt)
returns **Verdict: PASS** for exact commit
`ed6dd3f62ae225367b08b0134e1b792cea5d6e31`, with no high or medium finding.
Low L1 clarifies that the 15 format refusals are the single largest cause in
that repeat; the earlier phrase "tied causes" refers to only that one cause.
Counts and earlier entries remain unchanged. The additive A4B reporter uses
"cause" for a single cause and "tied causes" only for a tie; sealed E4B scripts
remain unchanged. Low L2's private-archive hygiene remains T-309 backlog.
No row closes.

## Raw JSON instruction A4B development repeat — 2026-11-12

Source: `a99fe467c72e2625022b639d8d38b72e845f612a`. All 30 frozen development tasks ran once on `gemma4-26b-a4b`. **Partial path: Rust source selection and credential-owning chat, followed by strict edit checking only when a complete answer is available. First-attempt and within-budget task passes remain unavailable, not zero.** No candidate tests, feedback, retries, approvals, apply, commit, push or held-out tasks ran. Native executable trust and the absence of the expected original grading-controls receipt remain unresolved; no control is bypassed or reconstructed. Selection receives only the statement and editable frozen base file; full repository analysis and retrieval are not measured. A strict proposal is an ungraded edit, never a solved task.

This is the sixth prepared model measured after the raw-JSON instruction change, with no intervening product change. Versus this profile's preceding complete repeat, the fixed generation instruction adds an original JSON syntax example and raw-object and string-escaping requirements. Source selection and strict checking stay unchanged. The profile retains empty template kwargs; runtime and model bytes, temperature zero, seed 42, 32768-token context, 32768-byte source selection, output formula `min(32768, 32768 - rendered_input_tokens - 32)`, 300/60-second total/idle, 310-second relay/client and byte ceilings are unchanged. 32K is the largest prepared context tested, not an exhaustive hardware maximum. The source-built effect-free helper uses the unchanged reviewed chat-only API binary. The product proposal route still caps output at 4096; this measurement adapter uses the existing chat allowance.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | gemma4-26b-a4b | 32768 | 23004–29513 | unavailable / unavailable (30 tasks) | 1/30 | 4192.995 | 4734.106 | 78.902 |

GPU work minutes mean reserved model-job wall/60, including verification, loading and cleanup, excluding queues and wrapper checks. Active GPU kernel time was not measured. Request/check time excludes preparation and preflight.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.limit` | 29 |
| `ungraded-proposal` | 1 |

Does-not-fit tasks: 0. Complete answers reaching strict checking: 1/30. One attempt per task, zero retries. All 15 two-task reservations use both required wrappers, complete and release the shared slots; their model and API processes are reaped and temporary model keys/sockets removed. No model was downloaded.

Unambiguous backend counters cover 1/30 tasks: 5395 input / 8344 output tokens. Other counters remain unknown; API usage was not requested.

The 29 size-limit refusals report that the server response exceeded a size limit. The existing 4 MiB stream-body ceiling bounds these attempts before the published token allowance. Stop totals minus tokenizer preflight are available for 29/29 size-limit refusals and put their cut at about 16,195–16,270 generated tokens, without context truncation. This is a stream-size limit, not evidence of token exhaustion or an infrastructure fault. Changing the ceiling remains T-309 backlog unless a high/medium finding or a row completion requirement needs it.

Strict ungraded proposals change from the preceding published full A4B measurement's 0/30 to 1/30 in this repeat. This new instruction repeat reports current-source observations, without isolating the instruction from changes to excerpt delivery or rendered output reservation. It establishes neither task correctness, generalization nor a model role. No new private comparison receipt is claimed.

Across the six current-source published repeat lines (180 attempts), strict ungraded proposals total 69; task pass rates remain unavailable. The largest aggregate refusal cause is `generation-refused:error:chat.transport.limit` (48). These totals add the existing cause tables; they do not combine partial paths into a full coding workflow or qualify a model.

Largest individual refusal count in this repeat: 29; cause: generation-refused:error:chat.transport.limit. Across the six current-source model repeats, the A4B profile's 29 refusals (generation-refused:error:chat.transport.limit) remain the largest individual refusal cause. All six prepared current-source repeats are complete. The next product work must address the largest observed failure cause, subject to high/medium review findings; these partial-path measurements do not complete coding-loop measurement. No profile or model role is adopted. Further limit, deadline, cancellation and bookkeeping features remain T-309 backlog unless an open high/medium finding or the row's own completion requirement needs them. T-303/T-306 remain open, without new rows, sub-rows or completion ticks.

See [evidence](t303-raw-json-a4b-2026-11-12.md) for per-task observations, frozen bindings and reproduction.

## Independent A4B review clarification — 2026-11-12

The [independent A4B measurement review](../reviews/9c715ed2d1c2-raw-json-a4b-measurement.txt)
returns **Verdict: PASS** for exact commit
`9c715ed2d1c264091ca5682c5f6124f4f903e4dd`, with no high or medium finding.
Low L1 adds a checked distinction: none of the 29 size-limit refusals emitted
visible text before the cut; the one proposal emitted 558 visible bytes for
8,344 output tokens. Stream overhead and non-answer output consumed the byte
allowance; these records do not isolate their respective costs. Low L2
clarifies that A4B's 29 refusals are now the largest individual refusal cause,
replacing the preceding leader of 25. Counts, original entries and sealed
reporters remain unchanged. Low L3's private-archive hygiene remains T-309
backlog. No row closes.

## Complete-response campaign queued — 2026-11-12

[Product source](t303-complete-responses-2026-11-12.md):
`7d860097edbf2e16cae279191b0fa7ac292b1283`. This is a scheduling note, **not a
scoreboard run**: zero model tasks executed. The first reservation waited at the
shared build slot and ended before acquiring the GPU slot or starting a model.
The source is ready for independent core review; its full affected gate still
has 53 inherited failures. The next measurements use complete responses with
the explicitly chosen existing 300-second idle budget, unchanged 300-second
total and unchanged byte/token ceilings. All 30 development tasks on every
prepared profile remain pending before further source tuning, except fixes for
high or medium findings. No task pass, improvement or model role is inferred.

## Complete-response review and pre-fix partial observations — 2026-11-12

The [independent core review](../reviews/05c53085b860-bounded-complete-model-responses.txt)
returns **Verdict: FINDINGS** for exact commit
`05c53085b86074ac461185c0027361230ec136e7`. Medium M1 identifies an acceptance
change: complete mode accepts empty answer text that streaming refuses. The
repair restores the incomplete-response refusal, with independent re-review
required. Low L1–L3 remain T-309 backlog. No row closes.

Before the verdict, source `7d860097edbf2e16cae279191b0fa7ac292b1283` ran only
dev-001 and dev-002 on `gemma4-26b-a4b`, with 32,768-token context and output
allowances 23,234 and 23,172. Both used their allowances and returned empty
visible text with `length`; the collector refused both. Zero accepted edits;
first-attempt and within-budget task passes remain unavailable. No candidate
tests, feedback, retries or held-out task ran. This is **partial pre-fix
progress, not a full development scoreboard run or a framing comparison**.

Request/check time totals 378.595 seconds. The pair reserved 410.633 seconds;
a separate startup-only failure reserved 190.465 seconds and reached no task.
Combined reserved GPU work is 10.018 minutes, excluding queues and including
verification, load and cleanup; active kernel time was not measured. The
startup failure came from an overlong private socket path, corrected only in
an additive private state-path adapter. Both runs used both reservation wrappers
and cleaned up. All 2,299 input hashes matched at the post-pair audit; a frozen
source snapshot preserves them separately from the M1 repair. A later pair waited
only for a build slot and executed no task. The [evidence](t303-complete-responses-2026-11-12.md)
retains the observations and their limits. All 30 tasks on every prepared model
remain required on the repaired source before further tuning, except high or
medium review fixes.

## Complete-response A4B development repeat — 2026-11-12

Source: `974d2bb55afa6be47bd58ffcf1dd30a6450f70cd`. This completes all 30 development tasks
for this profile on the repaired source; the coding-loop path remains partial.

| Date | Model | Context / output tokens | First-attempt task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU work min | Observed input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | gemma4-26b-a4b | 32768 / 23004–29513 | Unavailable | Unavailable | 2/30 | Not run | 5943.181 | 108.456 | 215893 / 743647 |

Changed variables: complete upstream framing and explicitly chosen 300-second
idle instead of 60; total remains 300 seconds. Output is
`min(32768, 32768 - rendered input tokens - 32)`, within the current prepared
32K policy. All 30 inputs fit; does not fit: 0. The original model profile,
retrieval, prompts, checker and byte ceilings remain unchanged. The empty-answer
repair preserves streaming's refusal rule; its independent re-review is pending.

Causes: 26 incomplete refusals (`chat.response.incomplete`), 2 strict format
refusals, 2 strict ungraded proposals. All 26 incomplete refusals emit no visible
text and reach their output allowance. Backend counters cover all 30 attempts;
output includes non-answer tokens. GPU work is 6507.374 seconds of measured job
wall inside 15 separate reservations, including verify/load/cleanup and excluding
queues and wrapper overhead; active kernel time is not measured.

The measured path is existing Rust retrieval → credential-owning local chat
API → original Rust strict checker. Native artifact admission and the original
grading-controls receipt remain unresolved. No candidate test, feedback, retry,
held-out task, native workflow, task grade or model role is claimed. Both
reservation wrappers and all cleanup checks pass. Full source/record audit,
30-context/check reproduction and the 21,845-file archive verification pass.
The [evidence](t303-complete-a4b-2026-11-12.md) retains per-task budgets, tokens,
time and raw-input bindings. Five prepared profiles still require the full
repeat before further tuning, except high/medium review fixes. T-303/T-306
remain open; independent core review is required and no row closes.

## Independent empty-answer repair review — 2026-11-12

The [independent re-review](../reviews/974d2bb55afa-complete-response-empty-answer-repair.txt)
returns **Verdict: PASS** for exact source commit
`974d2bb55afa6be47bd58ffcf1dd30a6450f70cd`. Medium M1 is resolved; no high
or medium finding remains. Low L4 records classification differences for null
or missing content and malformed usage across framings. Both refuse those
answers without a completion, so this does not reopen acceptance parity.
The A4B repeat observes empty-string content, the repaired case; its counts
remain unchanged. L4 joins L1–L3 as T-309 backlog, with no source change or
new row. This is a review-retention note, not another model measurement or
row closure. The five remaining complete-response profile repeats still
precede tuning; T-303/T-306 remain open.

## Independent A4B complete-response measurement review — 2026-11-12

The [independent core review](../reviews/a363361186f4-complete-response-a4b-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`a363361186f407df9eb8c3b7e8b922ad6d309e46`, with no high or medium finding.
It independently reproduces all 30 contexts and strict checks using its own
helper build, verifies every reported task count, token allowance and timing,
and confirms the measurement's partial scope. T-303/T-306 remain open.

Low L1 corrects the A4B evidence’s earlier lint description: the CLI linted all three Markdown
files with zero issues; its summary counts files with issues. The separate
direct lint also covers all three. Low L3 adds the omitted platform disclosure:
the recorded GPU driver package is `xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`,
and the profile marks its platform tuple changed since preparation. Reverified
runtime and model bytes are not platform or coding qualification. No measured
count or sealed input changes. Low L2, accepting an explicit archive root for
the private reproduction scripts, joins T-309 backlog; the reviewer reproduced
the result from the archive despite the original builder-only paths.

This is review retention and clarification, not another model run or row
closure. Five prepared profiles still require their complete-response repeats
before further tuning, except high/medium fixes. The independent source-repair
PASS is retained separately. No task grade, retry result or held-out run is
claimed.

## Complete-response 31B development repeat — 2026-11-12

Source: `974d2bb55afa6be47bd58ffcf1dd30a6450f70cd`. All 30 development tasks
complete on this profile; the measured coding path remains partial.

| Date | Model | Context / output tokens | First-attempt task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU work min | Observed input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | gemma4-31b | 32768 / 23002–29511 | Unavailable | Unavailable | 4/30 | Not run | 1632.393 | 36.244 | 215953 / 38865 |

Changed variables: complete upstream framing and explicitly chosen 300-second
idle instead of 60; total stays 300 seconds. Output is
`min(32768, 32768 - rendered input tokens - 32)`. All 30 inputs fit;
does not fit: 0. The existing thinking-disabled profile, prompts, source
selection, strict checker and byte ceilings stay unchanged. The source repair
has an independent PASS for the exact commit; no row closes.

Causes: 1 `strict-edit-refused:code.proposal.edit`, 25 `strict-edit-refused:code.proposal.format`, 4 `ungraded-proposal`.

GPU work means 2174.622 seconds of job wall inside
15 separate reservations, including verification/load/cleanup and excluding
queues and wrapper overhead; active kernel time was not measured. Backend
counters cover 30/30 and include non-answer output. No
candidate tests, feedback, retries, held-out tasks or task grades ran. The
native artifact and original grading-control blockers remain; the available
path is Rust source selection → credential-owning chat → strict checker.
No whole-loop, model-role or quality-improvement claim is made.

Full source/record audit, 30-context/check reproduction and archive verification
pass. See [evidence](t303-complete-31b-2026-11-12.md) for per-task results,
exact input bindings and limits. Four prepared profiles still require the full
repeat before tuning, except high/medium fixes. T-303/T-306 remain open;
independent core review is required and no row closes.

## Independent 31B measurement review — 2026-11-12

The [independent core review](../reviews/86495ed05152-complete-response-31b-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`86495ed05152ca8749cf801d59b802b55b8fb276`, with no high or medium finding.
It independently reproduces all 30 contexts and strict checks, including the
four accepted proposals, and verifies the counts, budgets, costs and archive.
The reviewer could not run Markdown lint because Node was absent; the bound
builder logs pass, and the review records compensating structural checks.

Low L1 is clarified here additively: the recorded GPU driver package is
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`; the platform tuple changed
since preparation and remains unqualified for coding. The evidence already
disclosed this. No measured count, sealed input or earlier entry changes.
T-303/T-306 remain open; no task grade, retry, held-out result or row closure
is claimed. This is review retention and clarification, not a new model run.

## Complete-response 27B development repeat — 2026-11-12

Source: `974d2bb55afa6be47bd58ffcf1dd30a6450f70cd`. All 30 development tasks
complete on this profile; the measured coding path remains partial.

| Date | Model | Context / output tokens | First-attempt task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU work min | Observed input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | qwen35-27b | 32768 / 24137–29804 | Unavailable | Unavailable | 29/30 | Not run | 1436.046 | 31.016 | 186402 / 34920 |

Changed variables: complete upstream framing and explicitly chosen 300-second
idle instead of 60; total stays 300 seconds. Output is
`min(32768, 32768 - rendered input tokens - 32)`. All 30 inputs fit;
does not fit: 0. The existing thinking-disabled profile, prompts, source
selection, strict checker and byte ceilings stay unchanged. The source repair
has an independent PASS for the exact commit; no row closes.
The recorded GPU driver package is
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`; the platform tuple changed
since preparation and remains unqualified for coding.

Causes: 1 `generation-refused:error:chat.transport.timeout`, 29 `ungraded-proposal`.
Dev-004 reaches the existing 300-second deadline with no visible text event;
its backend slice has 98 progress counters reaching `n_gen = 13220` at
298.310 seconds after slot launch. Generation continued until the total
deadline, before its 29,804-token allowance; the cause of the long output is
not established. There is no final eval summary or stop line. The final output
count is unavailable, with an observed lower bound of 13,220 tokens.

GPU work means 1860.960 seconds of job wall inside
15 separate reservations, including verification/load/cleanup and excluding
queues and wrapper overhead; active kernel time was not measured. Backend
final counters cover 29/30 and include non-answer output.
Tasks without final counters: dev-004; totals exclude it and do not add its
13,220-token lower bound. Missing final counters are unavailable, not zero tokens. No
candidate tests, feedback, retries, held-out tasks or task grades ran. The
native artifact and original grading-control blockers remain; the available
path is Rust source selection → credential-owning chat → strict checker.
No whole-loop, model-role or quality-improvement claim is made.

Full source/record audit, 30-context/check reproduction and archive verification
pass. See [evidence](t303-complete-qwen-2026-11-12.md) for per-task results,
exact input bindings and limits. Three prepared profiles still require the full
repeat before tuning, except high/medium fixes. T-303/T-306 remain open;
independent core review is required and no row closes.

## Independent 27B measurement review and correction — 2026-11-12

The [independent core review](../reviews/355e4616aedd-complete-response-qwen-measurement.txt)
returns **Verdict: FINDINGS** for exact reporting commit
`355e4616aedd135fa066d760480cc321d60545f5`. Medium M1 identifies a false
statement about dev-004's backend counters. The corrected entry above and
[evidence](t303-complete-qwen-2026-11-12.md) distinguish its observed
13,220-token progress lower bound from missing final counters. Task outcomes,
aggregate totals, sealed records and all earlier entries stay unchanged.
Low L1 is clarified by the effective deadline disclosure; deadline changes
remain T-309 backlog. This is a reporting correction, not a new scoreboard run.
Independent re-review is required; T-303/T-306 remain open and no row closes.

## Independent 27B timing re-review — 2026-11-12

The [independent core re-review](../reviews/95616eb9f850-complete-response-qwen-timing-repair.txt)
returns **Verdict: PASS** for exact reporting repair commit
`95616eb9f85079819a3e089e6f30aad83fdc91ee`, resolving medium M1 with no
high or medium finding. It independently verifies the progress lower bound,
missing final counters, unchanged totals and original archive bindings.
The reviewer could not run Markdown lint because Node was absent; the bound
builder log passes, and the review records compensating checks.

Low L1 is clarified here additively: at the observed throughput of about
45 tokens per second, the existing 300-second total deadline would cap
output near 13,400 tokens, below every allowance in the profile's
24,137–29,804 range. This is an estimate from observed throughput, not an
additional measurement or a guarantee of constant generation speed. Any
deadline change remains T-309 backlog. Measured counts, original sealed
inputs and earlier entries stay unchanged. T-303/T-306 remain open; no task
grade, retry, held-out result or row closure is claimed.

## Complete-response GPT-OSS development repeat — 2026-11-12

Source: `974d2bb55afa6be47bd58ffcf1dd30a6450f70cd`. All 30 development tasks
complete on this profile; the measured coding path remains partial.

| Date | Model | Context / output tokens | First-attempt task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU work min | Observed input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | gpt-oss | 32768 / 24219–29748 | Unavailable | Unavailable | 5/30 | Not run | 2743.824 | 48.771 | 188409 / 539597 |

Changed variables: complete upstream framing and explicitly chosen 300-second
idle instead of 60; total stays 300 seconds. Output is
`min(32768, 32768 - rendered input tokens - 32)`. All 30 inputs fit;
does not fit: 0. The existing medium-reasoning profile, prompts, source
selection, strict checker and byte ceilings stay unchanged. The source repair
has an independent PASS for the exact commit; no row closes.
The recorded GPU driver package is
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`; the platform tuple changed
since preparation and remains unqualified for coding.

Causes: 13 `generation-refused:error:chat.response.incomplete`, 8 `strict-edit-refused:code.proposal.edit`, 4 `strict-edit-refused:code.proposal.format`, 5 `ungraded-proposal`.

All 13 incomplete refusals reached their output allowance; the reason no completed answer was accepted is not established.

GPU work means 2926.278 seconds of job wall inside
15 separate reservations, including verification/load/cleanup and excluding
queues and wrapper overhead; active kernel time was not measured. Backend
final counters cover 30/30 and include non-answer output.
Final counters are available for all 30 attempts. No
candidate tests, feedback, retries, held-out tasks or task grades ran. The
native artifact and original grading-control blockers remain; the available
path is Rust source selection → credential-owning chat → strict checker.
No whole-loop, model-role or quality-improvement claim is made.

Full source/record audit, 30-context/check reproduction and archive verification
pass. See [evidence](t303-complete-gpt-2026-11-12.md) for per-task results,
exact input bindings and limits. Two prepared profiles still require the full
repeat before tuning, except high/medium fixes. T-303/T-306 remain open;
independent core review is required and no row closes.

## Independent GPT-OSS complete-response measurement review — 2026-11-12

The [independent core review](../reviews/62d675983c1b-complete-response-gpt-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`62d675983c1ba74d264c628a9ca38d6a79a1cf49`, with no high or medium finding.
It independently verifies every per-task row and reproduces all 30 contexts
and strict checks, including the five accepted ungraded proposals. Markdown
lint could not run in the review lane without Node; the review retains the
bound passing builder log and its own structural checks.

Low L1 is clarified additively: all 13 incomplete refusals exhausted their
output allowances and returned empty answer content. This follows from the
complete decoder's empty-content branch and the recorded token counters;
why the model generated no answer content is not established. Low L2 is
clarified here: the first controller stopped before pair 02 acquired a build
reservation; the later controller skipped completed pair 00 and ran pairs
02–28. No task attempt was replayed or replaced. Low L3, streaming-specific
error wording used for complete responses, remains T-309 source backlog.

No measured count, sealed input or earlier entry changes. This is review
retention and clarification, not a new model run. T-303/T-306 remain open;
no grade, retry, held-out result or row closure is claimed.

## Complete-response Muse development repeat — 2026-11-12

Campaign date denotes its start day; attempts span 2026-11-12–13 UTC.

Source: `974d2bb55afa6be47bd58ffcf1dd30a6450f70cd`. All 30 development tasks
complete on this profile; the measured coding path remains partial.

| Date | Model | Context / output tokens | First-attempt task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU work min | Observed input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-12 | muse | 32768 / 24395–29826 | Unavailable | Unavailable | 19/30 | Not run | 4755.558 | 88.812 | 174326 / 163130 |

Changed variables: complete upstream framing and explicitly chosen 300-second
idle instead of 60; total stays 300 seconds. Output is
`min(32768, 32768 - rendered input tokens - 32)`. All 30 inputs fit;
does not fit: 0. The existing medium-reasoning profile, prompts, source
selection, strict checker and byte ceilings stay unchanged. The source repair
has an independent PASS for the exact commit; no row closes.
The recorded GPU driver package is
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`; the platform tuple changed
since preparation and remains unqualified for coding.

Causes: 3 `generation-refused:error:chat.transport.timeout`, 3 `strict-edit-refused:code.proposal.edit`, 5 `strict-edit-refused:code.proposal.format`, 19 `ungraded-proposal`.

Existing deadlines and output ceilings remain unchanged; backend termination counts are reported separately in the evidence.

Dev-004 has 99 progress counters, reaching a 13,407-token lower bound at
299.919 seconds after slot launch, before its 29,805-token allowance.
Dev-011 has 95 counters, reaching 12,517 at 297.552 seconds, before 26,793.
Dev-012 has 94 counters, reaching 12,512 at 286.791 seconds, before 29,795.
Their final token counters are unavailable; these lower bounds are excluded
from aggregate totals. The four counters before dev-012's own slot launch
belong to the preceding request and are excluded from its count.

GPU work means 5328.730 seconds of job wall inside
15 separate reservations, including verification/load/cleanup and excluding
queues and wrapper overhead; active kernel time was not measured. Backend
final counters cover 27/30 and include non-answer output.
Tasks without final counters: dev-004, dev-011, dev-012; totals exclude them.
Missing final counters are unavailable, not zero tokens. No
candidate tests, feedback, retries, held-out tasks or task grades ran. The
native artifact and original grading-control blockers remain; the available
path is Rust source selection → credential-owning chat → strict checker.
No whole-loop, model-role or quality-improvement claim is made.

Full source/record audit, 30-context/check reproduction and archive verification
pass. See [evidence](t303-complete-muse-2026-11-12.md) for per-task results,
exact input bindings and limits. One prepared profile still requires the full
repeat before tuning, except high/medium fixes. T-303/T-306 remain open;
independent core review is required and no row closes.

## Independent Muse complete-response measurement review — 2026-11-13

The [independent core review](../reviews/19300085342d-complete-response-muse-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`19300085342d37c119ed2a606fc2eea9141d2fc6`, with no high or medium finding.
It independently recomputes all 30 table rows, counters and costs, and checks
all 27 answers and their candidate sources. The review could not run Markdown
lint without Node; it retains the bound passing builder log and its own
structural checks.

Low L1 is clarified additively: dev-012's 300-second request/check window
included about 11 seconds of backend generation for the cancelled dev-011
request before dev-012's own slot launch. The four earlier progress counters
remain excluded from dev-012's counts. Complete-mode timeout cancellation
latency joins T-309 backlog. No published count or cause changes; all sealed
inputs remain unchanged. This is review retention, not a new model run.
T-303/T-306 remain open; no task grade, retry, held-out result or row closure
is claimed.

## Complete-response E4B development repeat — 2026-11-13

Source: `974d2bb55afa6be47bd58ffcf1dd30a6450f70cd`. All 30 development tasks
complete on this profile; the measured coding path remains partial.

| Date | Model | Context / output tokens | First-attempt task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU work min | Observed input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gemma4-e4b | 32768 / 23004–29513 | Unavailable | Unavailable | 13/30 | Not run | 1372.592 | 24.998 | 215893 / 138484 |

Changed variables: complete upstream framing and explicitly chosen 300-second
idle instead of 60; total stays 300 seconds. Output is
`min(32768, 32768 - rendered input tokens - 32)`. All 30 inputs fit;
does not fit: 0. The existing profile, prompts, source
selection, strict checker and byte ceilings stay unchanged. The source repair
has an independent PASS for the exact commit; no row closes.
The recorded GPU driver package is
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`; the platform tuple changed
since preparation and remains unqualified for coding.

Across all six repaired-source repeats (180 development attempts), the largest observed refusal is `strict-edit-refused:code.proposal.format` (51/180). The next source work follows that cause, subject to high/medium review findings and existing blockers. These are partial-path refusal counts, not task pass rates or model-role selection.

Causes: 2 `strict-edit-refused:code.proposal.edit`, 15 `strict-edit-refused:code.proposal.format`, 13 `ungraded-proposal`.

Existing deadlines and output ceilings remain unchanged; backend termination counts are reported separately in the evidence.

Final token counters are available for every attempt.

GPU work means 1499.878 seconds of job wall inside
15 separate reservations, including verification/load/cleanup and excluding
queues and wrapper overhead; active kernel time was not measured. Backend
final counters cover 30/30 and include non-answer output.
Final counters are available for every attempt; totals cover all 30. No
candidate tests, feedback, retries, held-out tasks or task grades ran. The
native artifact and original grading-control blockers remain; the available
path is Rust source selection → credential-owning chat → strict checker.
No whole-loop, model-role or quality-improvement claim is made.

Full source/record audit, 30-context/check reproduction and archive verification
pass. See [evidence](t303-complete-e4b-2026-11-13.md) for per-task results,
exact input bindings and limits. All six prepared profiles have completed
the full repeat, with task grades unavailable. T-303/T-306 remain open;
independent core review is required and no row closes.

The six repeats at the source above total 180 development attempts, each using
32,768-token context and the same output-budget formula. These are strict
proposal counts only; task passes and passes with retries remain unavailable.

| Model profile | Strict ungraded proposals | Recorded refusals |
| --- | --- | --- |
| gemma4-26b-a4b | 2/30 | 28/30 |
| gemma4-31b | 4/30 | 26/30 |
| qwen35-27b | 29/30 | 1/30 |
| gpt-oss | 5/30 | 25/30 |
| muse | 19/30 | 11/30 |
| gemma4-e4b | 13/30 | 17/30 |

Across the 180 attempts: 72 strict ungraded proposals, 51 format refusals,
39 incomplete responses, 14 edit refusals and 4 timeouts. Format refusal is
the next source priority. No model role is selected from these counts, and
held-out tasks remain unrun.

## Independent E4B complete-response measurement review — 2026-11-13

The [independent core review](../reviews/b826d82e0019-complete-response-e4b-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`b826d82e001980eafae7b9ae145a969dcfd14a8c`, with no high or medium finding.
It independently recomputes every E4B task row, all costs and the six-profile
aggregate from the retained raw inputs. The reviewer could not run Markdown
lint without Node; the bound builder lint log and independent structural
checks are retained.

Low L1 is clarified additively: the preceding table's “Recorded refusals”
means **non-proposal outcomes**. Its 108 outcomes include 39 incomplete
responses and four timeouts as well as 65 strict edit/format refusals. The
A4B profile's 28 comprise 26 incomplete responses and two format refusals.
The repeated counter-coverage sentences remain historical reporting text;
report concision joins T-309 backlog. Counts and sealed inputs stay unchanged.
This is review retention, not a new model run. T-303/T-306 remain open;
no task grade, retry, held-out result or row closure is claimed.

## JSON-object repeat queued — 2026-11-13

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. This is a scheduling
checkpoint, **not a model measurement or pass-rate entry**. The new API binary
and 2,283 repository plus 16 private measurement inputs are hash-frozen.
Changed variable: trusted JSON-object output request; complete framing, prompts,
strict checker, profiles, byte ceilings and deadlines remain unchanged.

| Date | First planned profile | Development attempts | First / within-budget passes | Held-out | GPU job min | Outcome |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gemma4-e4b | 0/30 | Unavailable / unavailable | Not run | 0 | Shared build reservation unavailable; no model launched |

Planned context is 32,768 tokens; planned output remains
`min(32768, 32768 - rendered input tokens - 32)`. No new input preflight,
fit classification, model failure, token counter or task grade exists. The
owned waiter was checkpointed after at least 467 seconds queued, before any
GPU reservation or job directory. All owned processes exited. This wait is
excluded from GPU work and is not a failed task attempt. A fresh resumable
controller preserves the original queue log; no completed attempt can be
replayed because none began.

The complete 30-task development repeat remains first work on resumption,
followed by the other five prepared profiles before further source tuning.
Existing native artifact and original grading-control blockers still limit
measurement to the partial path; no candidate tests, retries or held-out run
are authorized by this checkpoint. Independent core review of the source
increment remains required. T-303/T-306 remain open; no row closes.

## Independent JSON-object source review — 2026-11-13

The [independent core review](../reviews/de8c204cbc52-json-object-model-output.txt)
returns **Verdict: PASS** for exact commit
`de8c204cbc52c7b090f8039a4bfaf15666854852`, with no high or medium finding.
Low L1's format-aware size preflight joins T-309 backlog; current behavior
fails closed. Explicit mode/format record fields remain a later backlog option.
This retains a source verdict, not a new measurement. The frozen E4B repeat is
running; all six profiles precede further tuning. No task grade, retry,
held-out result, model role or row closure is claimed.

## JSON-object E4B development repeat — 2026-11-13

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. Available partial path:
Rust source selection → credential-owning chat → unchanged strict checker;
full repository retrieval, native execution, tests and retries remain unavailable.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gemma4-e4b | 32768 / 23004–29513 | Unavailable | Unavailable | 24/30 | Not run | 1363.743 | 24.748 | 215893 / 138991 |

Changed variable: trusted JSON-object output request, verified in all 30 upstream
bodies. Complete framing, prompts, source selection, strict checker, profiles,
byte ceilings and 300-second idle/total deadlines stay unchanged. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0.
Causes: 4 `strict-edit-refused:code.proposal.edit`, 2 `strict-edit-refused:code.proposal.format`, 24 `ungraded-proposal`.

Final backend counters cover 30/30 and include non-answer output.
GPU minutes are reserved job wall, including load/verification/cleanup and
excluding queues, wrapper overhead and pauses; active kernel time is not measured.
The changed driver tuple remains unqualified. No candidate tests, feedback,
retries or held-out tasks ran; native artifact and original grading-control
blockers remain. A strict proposal is not a solved task. No model role or
task-quality improvement is claimed. The five remaining profiles precede tuning.
Source/record audit, helper reproduction, cleanup and archive verification pass;
see [evidence](t303-json-object-e4b-2026-11-13.md). T-303/T-306 remain open;
independent core review is required and no row closes.

The [independent E4B review](../reviews/95a53df95d2b-json-object-e4b-measurement.txt)
returns PASS for exact reporting commit
`95a53df95d2bd26b45abeef46ea11ce7ab2f2357`, with no high or medium finding.
Low L1 clarifies that only the GPU wrapper applies the 900-second execution
cap; both resource wrappers are used. Counts and sealed inputs stay unchanged.
T-303/T-306 remain open; the five remaining profile repeats still precede tuning.

## JSON-object 31B development repeat — 2026-11-13

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. Available partial path:
Rust source selection → credential-owning chat → unchanged strict checker;
full repository retrieval, native execution, tests and retries remain unavailable.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gemma4-31b | 32768 / 23002–29511 | Unavailable | Unavailable | 26/30 | Not run | 1395.989 | 32.638 | 215953 / 31604 |

Changed variable: trusted JSON-object output request, verified in all 30 upstream
bodies. Complete framing, prompts, source selection, strict checker, profiles,
byte ceilings and 300-second idle/total deadlines stay unchanged. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0.
Causes: 3 `strict-edit-refused:code.proposal.edit`, 1 `strict-edit-refused:code.proposal.format`, 26 `ungraded-proposal`.

Final backend counters cover 30/30 and include non-answer output.
GPU minutes are reserved job wall, including load/verification/cleanup and
excluding queues, wrapper overhead and pauses; active kernel time is not measured.
The changed driver tuple remains unqualified. No candidate tests, feedback,
retries or held-out tasks ran; native artifact and original grading-control
blockers remain. A strict proposal is not a solved task. No model role or
task-quality improvement is claimed. The four remaining profiles precede tuning.
Source/record audit, helper reproduction, cleanup and archive verification pass;
see [evidence](t303-json-object-31b-2026-11-13.md). T-303/T-306 remain open;
independent core review is required and no row closes.

Independent [31B measurement review](../reviews/ebf1db8defc7-json-object-31b-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`ebf1db8defc74ef4d49fe4c840ef8551f39cca36`, with no findings at any severity.
Counts, earlier entries and sealed inputs remain unchanged. T-303/T-306 stay
open; no task grade or row closure is claimed.

## JSON-object 27B development repeat — 2026-11-13

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. Available partial path:
Rust source selection → credential-owning chat → unchanged strict checker;
full repository retrieval, native execution, tests and retries remain unavailable.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | qwen35-27b | 32768 / 24137–29804 | Unavailable | Unavailable | 29/30 | Not run | 1434.814 | 30.778 | 186402 / 34920 |

Changed variable: trusted JSON-object output request, verified in all 30 upstream
bodies. Complete framing, prompts, source selection, strict checker, profiles,
byte ceilings and 300-second idle/total deadlines stay unchanged. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0.
Causes: 1 `generation-refused:error:chat.transport.timeout`, 29 `ungraded-proposal`.

Final backend counters cover 29/30 and include non-answer output.

dev-004 reaches the existing total deadline. Its backend slice retains 98 progress counters, reaching an observed lower bound of 13,012 output tokens at 298.319 seconds after its own slot launch. Its final eval summary and stop line are absent, so its final input/output totals remain unavailable. The lower bound is not added to the final-counter totals; missing final counters are not zero tokens. The cause of the long output is not established. Deadline changes remain T-309 backlog.

GPU minutes are reserved job wall, including load/verification/cleanup and
excluding queues, wrapper overhead and pauses; active kernel time is not measured.
The changed driver tuple remains unqualified. No candidate tests, feedback,
retries or held-out tasks ran; native artifact and original grading-control
blockers remain. A strict proposal is not a solved task. No model role or
task-quality improvement is claimed. The three remaining profiles precede tuning.
Source/record audit, helper reproduction, cleanup and archive verification pass;
see [evidence](t303-json-object-qwen-2026-11-13.md). T-303/T-306 remain open;
independent core review is required and no row closes.

## Independent JSON-object 27B review retained — 2026-11-13

The [independent core review](../reviews/9bdf034fa328-json-object-qwen-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`9bdf034fa32856ee173e46a9e0c2d2873aa2b30d`, with no findings at any severity.
Counts and sealed inputs stay unchanged. The reviewer could not rerun Markdown
lint because Node was absent; bound builder logs and independent structural
checks were used. This retains a reporting verdict, not a new measurement.
The GPT-OSS repeat is queued; all three remaining profiles precede further
tuning. No task grade, retry, held-out result, model role or row closure is claimed.

## GPT-OSS JSON-object checkpoint — 2026-11-13 (incomplete)

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. **4/30 tasks attempted;
26 remain.** Available partial path: Rust source selection → credential-owning
chat → unchanged strict checker. Native execution, tests and retries remain
blocked; this is not a complete development measurement or a solved-task score.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gpt-oss (4/30 attempted) | 32768 / 24497–29748 | Unavailable | Unavailable | 0/4 | Not run | 452.918 | 8.009 | 23752 / 89272 |

Changed variable: trusted JSON-object output request, verified in all four
upstream bodies. All other measurement settings stay fixed: complete framing,
prompts, selection, checker, profiles, byte ceilings and 300-second idle/total
deadlines. Output remains `min(32768, 32768 - rendered input tokens - 32)`;
does not fit: 0/4 attempted. Causes: one `strict-edit-refused:code.proposal.edit`
and three `generation-refused:error:chat.response.incomplete`. Each incomplete
response exhausted its output allowance; final counters cover all four attempts
and include non-answer output. The cause of the long output is not established.

GPU minutes are reserved job wall, including verification/load/cleanup, excluding
queues, wrapper overhead and scheduler pauses; active kernel time is unmeasured.
The next pair's 404.835-second build wait was checkpointed before GPU entry or
any attempt. Resume tasks 5–6; preserve the four completed attempts. Audit,
helper reproduction, cleanup and read/hash archive verification pass; see the
[checkpoint evidence](t303-json-object-gpt-checkpoint-2026-11-13.md).
The driver remains unqualified. No native execution, grading, retry, hidden or
held-out test ran. The remaining GPT-OSS tasks, Muse and A4B precede tuning,
except high/medium fixes. T-303/T-306 stay open; independent core review is
required and no row closes.

## Independent GPT-OSS checkpoint review retained — 2026-11-13

The [independent core review](../reviews/77cd9eefb3c2-json-object-gpt-checkpoint.txt)
returns **Verdict: PASS** for exact checkpoint commit
`77cd9eefb3c2f9ed2d4d51df2bf7478dbdb7cbd8`, with no high or medium finding.
Low L1 (bind the reporting scope script) and L2 (capture preparation stderr)
join T-309 backlog. Counts and sealed inputs remain unchanged; this is a
reporting verdict, not another measurement. The reviewer could not rerun
Markdown lint because Node was absent and relied on bound builder logs and
independent structural checks. The remaining GPT-OSS tasks are resuming;
T-303/T-306 remain open and no task grade or row closure is claimed.

## JSON-object GPT-OSS development repeat — 2026-11-13

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. Available partial path:
Rust source selection → credential-owning chat → unchanged strict checker;
full repository retrieval, native execution, tests and retries remain unavailable.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gpt-oss | 32768 / 24219–29748 | Unavailable | Unavailable | 12/30 | Not run | 2396.339 | 42.852 | 188409 / 475158 |

Changed variable: trusted JSON-object output request, verified in all 30 upstream
bodies. Complete framing, prompts, source selection, strict checker, profiles,
byte ceilings and 300-second idle/total deadlines stay unchanged. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0.
Causes: 10 `generation-refused:error:chat.response.incomplete`, 4 `strict-edit-refused:code.proposal.edit`, 4 `strict-edit-refused:code.proposal.format`, 12 `ungraded-proposal`.

Final backend counters cover 30/30 and include non-answer output.
GPU minutes are reserved job wall, including load/verification/cleanup and
excluding queues, wrapper overhead and pauses; active kernel time is not measured.
The changed driver tuple remains unqualified. No candidate tests, feedback,
retries or held-out tasks ran; native artifact and original grading-control
blockers remain. A strict proposal is not a solved task. No model role or
task-quality improvement is claimed. The two remaining profiles precede tuning.
Source/record audit, helper reproduction, cleanup and archive verification pass;
see [evidence](t303-json-object-gpt-2026-11-13.md). T-303/T-306 remain open;
independent core review is required and no row closes.

## Muse JSON-object checkpoint — 2026-11-13 (incomplete)

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. **2/30 tasks attempted; 28 remain.**
Available partial path: Rust source selection → credential-owning chat → unchanged
strict checker. Full repository retrieval, native execution, tests and retries
remain unavailable; this is not a solved-task score or a completed measurement.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | muse (2/30 attempted) | 32768 / 24509–24616 | Unavailable | Unavailable | 1/2 | Not run | 326.925 | 5.900 | 16347 / 14067 |

Changed variable: trusted JSON-object output request, verified in both upstream
bodies. Complete framing, prompts, selection, strict checker, profiles, byte
ceilings and 300-second idle/total deadlines stay fixed. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0/2 attempted.
Causes: 1 `strict-edit-refused:code.proposal.edit`, 1 `ungraded-proposal`. Final counters cover 2/2 and include
non-answer output. GPU minutes are reserved job wall including load/verification/
cleanup, excluding queues, wrapper overhead and pauses; kernel time is unmeasured.
The next pair's 733.337-second shared-build
wait ended before GPU entry or a new attempt. Resume tasks 3–4 without replay.
Audit, helper reproduction, cleanup and archive verification pass; see the
[checkpoint evidence](t303-json-object-muse-checkpoint-2026-11-13.md). No candidate tests, feedback, retries, hidden or
held-out tests ran; native admission and original-control blockers remain. The
driver remains unqualified. Remaining Muse tasks and A4B precede tuning except
high/medium fixes. T-303/T-306 stay open; independent core review is required.

## Independent GPT-OSS measurement review retained — 2026-11-13

The [independent core review](../reviews/be161e975746-json-object-gpt-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`be161e975746f8f00ff9c3b7843904d334c4b1db`, with no findings at any severity.
Measured counts and sealed inputs remain unchanged; this is a reporting verdict,
not another measurement or a solved-task score. Muse is resuming and A4B follows
before further tuning, except high/medium fixes. T-303/T-306 remain open and no
row closes.

## Independent Muse checkpoint review retained — 2026-11-13

The [independent core review](../reviews/8af9f066d621-json-object-muse-checkpoint.txt)
returns **Verdict: PASS** for exact reporting commit
`8af9f066d621b673d9c30f7e0505e5c4dcb4c111`, with no high or medium finding.
Low L1 (plain prose for backend terminations) and L2 (fresh post-cleanup process
scan in the receipt) join T-309 backlog. Counts and sealed inputs stay unchanged;
this approves the two-task checkpoint report only. Muse is resuming; A4B follows
before tuning except high/medium fixes. T-303/T-306 stay open and no row closes.

## Muse JSON-object cumulative checkpoint — 2026-11-13 (12 of 30 tasks)

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. **12/30 tasks attempted; 18 remain.**
This includes the earlier two-task prefix; do not add the checkpoint lines.
Available partial path: Rust source selection → credential-owning chat → unchanged
strict checker. Full repository retrieval, native execution, tests and retries
remain unavailable; this is not a solved-task score or a completed measurement.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | muse (12/30 attempted) | 32768 / 24412–29826 | Unavailable | Unavailable | 6/12 | Not run | 2170.744 | 38.326 | 58297 / 54577 |

Changed variable: trusted JSON-object output request, verified in all 12 upstream
bodies. Complete framing, prompts, selection, strict checker, profiles, byte
ceilings and 300-second idle/total deadlines stay fixed. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0/12 attempted.
Causes: 3 `generation-refused:error:chat.transport.timeout`, 2 `strict-edit-refused:code.proposal.edit`, 1 `strict-edit-refused:code.proposal.format`, 6 `ungraded-proposal`. Final counters cover 9/12 and include
non-answer output; missing counters are not zero tokens. GPU minutes are reserved
job wall including load/verification/cleanup, excluding queues, wrapper overhead
and pauses; kernel time is unmeasured. The next pair's
836.680-second shared-build wait ended before
GPU entry or a new attempt. Resume tasks 13–14 without replay. Audit, helper
reproduction, cleanup and archive verification pass; see the
[checkpoint evidence](t303-json-object-muse-checkpoint-2026-11-13-12-tasks.md). No candidate tests, feedback, retries, hidden or
held-out tests ran; native admission and original-control blockers remain. The
driver remains unqualified. Remaining Muse tasks and A4B precede tuning except
high/medium fixes. T-303/T-306 stay open; independent core review is required.

## Independent cumulative Muse checkpoint review retained — 2026-11-13

The [independent core review](../reviews/dd19baea2e52-json-object-muse-checkpoint12.txt)
returns **Verdict: PASS** for exact reporting commit
`dd19baea2e52f64a3d632c573c2ee1809fda0d98`, with no high or medium finding.
Low L1 clarifies that dev-012's request/check window includes about 11 seconds
of backend generation for cancelled dev-011 before its own slot launch.
Cancellation latency remains T-309 backlog. Counts and sealed inputs stay
unchanged; this approves only the cumulative 12/30 partial-path report.
Muse is resuming, with A4B next before tuning except high/medium fixes.
T-303/T-306 stay open and no row closes.

## Muse JSON-object cumulative checkpoint — 2026-11-13 (22 of 30 tasks)

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. **22/30 tasks attempted; 8 remain.**
This includes the earlier two-task and 12-task prefixes; do not add checkpoint lines.
Available partial path: Rust source selection → credential-owning chat → unchanged
strict checker. Full repository retrieval, native execution, tests and retries
remain unavailable; this is not a solved-task score or a completed measurement.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | muse (22/30 attempted) | 32768 / 24395–29826 | Unavailable | Unavailable | 14/22 | Not run | 3607.273 | 65.515 | 125470 / 115890 |

Changed variable: trusted JSON-object output request, verified in all 22 upstream
bodies. Complete framing, prompts, selection, strict checker, profiles, byte
ceilings and 300-second idle/total deadlines stay fixed. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0/22 attempted.
Causes: 3 `generation-refused:error:chat.transport.timeout`, 2 `strict-edit-refused:code.proposal.edit`, 3 `strict-edit-refused:code.proposal.format`, 14 `ungraded-proposal`. Final counters cover 19/22 and include
non-answer output; missing counters are not zero tokens. GPU minutes are reserved
job wall including load/verification/cleanup, excluding queues, wrapper overhead
and pauses; kernel time is unmeasured. The next pair's
911.025-second shared-build wait ended before
GPU entry or a new attempt. Resume tasks 23–24 without replay. Audit, helper
reproduction, cleanup and archive verification pass; see the
[checkpoint evidence](t303-json-object-muse-checkpoint-2026-11-13-22-tasks.md). No candidate tests, feedback, retries, hidden or
held-out tests ran; native admission and original-control blockers remain. The
driver remains unqualified. Remaining Muse tasks and A4B precede tuning except
high/medium fixes. T-303/T-306 stay open; independent core review is required.

## Muse JSON-object cumulative checkpoint — 2026-11-13 (24 of 30 tasks)

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. **24/30 tasks attempted; 6 remain.**
This includes the earlier 2-, 12- and 22-task checkpoints; never add their totals.
Tasks 23–24 were already complete; no new attempt started in this reporting session.
Available partial path: Rust source selection → credential-owning chat → unchanged
strict checker. Full retrieval, native execution, tests and retries remain
unavailable; this is not a solved-task score or a completed measurement.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | muse (24/30 attempted) | 32768 / 24395–29826 | Unavailable | Unavailable | 16/24 | Not run | 3773.161 | 68.555 | 138192 / 122753 |

Changed variable: trusted JSON-object output request, verified in all 24 upstream
bodies. Complete framing, prompts, selection, checker, profiles, byte ceilings and
300-second idle/total deadlines stay fixed. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0/24 attempted.
Causes: 3 `generation-refused:error:chat.transport.timeout`, 2 `strict-edit-refused:code.proposal.edit`, 3 `strict-edit-refused:code.proposal.format`, 16 `ungraded-proposal`. Final counters cover 21/24 and include
non-answer output; missing counters are not zero tokens. GPU minutes are reserved
job wall including load, verification and cleanup; queues, overhead and pauses
are excluded and active kernel time is unmeasured. The latest queue checkpoint
waited 509.793 seconds for a shared build
slot without GPU entry or a new attempt. Audit, helper reproduction, cleanup and
archive verification pass; see [evidence](t303-json-object-muse-checkpoint-2026-11-13-24-tasks.md). No candidate tests, retries,
hidden or held-out tests ran. Native admission and original-control blockers
remain; driver unqualified. Resume tasks 25–26 without replay. Remaining Muse
and A4B precede tuning except high/medium fixes. T-303/T-306 remain open;
independent core review is required and no row closes.

## JSON-object Muse development repeat — 2026-11-13

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. Available partial path:
Rust source selection → credential-owning chat → unchanged strict checker;
full repository retrieval, native execution, tests and retries remain unavailable.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | muse | 32768 / 24395–29826 | Unavailable | Unavailable | 19/30 | Not run | 4716.747 | 85.519 | 174326 / 163130 |

This full repeat includes the earlier 2-, 12-, 22- and 24-task checkpoints; do not add
their totals together.

Changed variable: trusted JSON-object output request, verified in all 30 upstream
bodies. Complete framing, prompts, source selection, strict checker, profiles,
byte ceilings and 300-second idle/total deadlines stay unchanged. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0.
Causes: 4 `generation-refused:error:chat.transport.timeout`, 3 `strict-edit-refused:code.proposal.edit`, 4 `strict-edit-refused:code.proposal.format`, 19 `ungraded-proposal`.

Final backend counters cover 27/30 and include non-answer output.
GPU minutes are reserved job wall, including load/verification/cleanup and
excluding queues, wrapper overhead and pauses; active kernel time is not measured.
The changed driver tuple remains unqualified. No candidate tests, feedback,
retries or held-out tasks ran; native artifact and original grading-control
blockers remain. A strict proposal is not a solved task. No model role or
task-quality improvement is claimed. The final remaining profile precedes tuning.
Dev-012's request/check window includes about 11 seconds of cancelled dev-011
generation before its own slot launch; cancellation latency stays T-309 backlog.
Source/record audit, helper reproduction, cleanup and archive verification pass;
see [evidence](t303-json-object-muse-2026-11-13.md). T-303/T-306 remain open;
independent core review is required and no row closes.

## A4B JSON-object checkpoint — 2026-11-13 (incomplete)

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. **2/30 tasks attempted; 28 remain.**
Available partial path: Rust source selection → credential-owning chat → unchanged
strict checker. Full retrieval, native execution, tests and retries remain
unavailable. These are ungraded proposals, not solved tasks.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gemma4-26b-a4b (2/30 attempted) | 32768 / 23172–23234 | Unavailable | Unavailable | 0/2 | Not run | 373.350 | 6.770 | 19066 / 46406 |

Changed variable: trusted JSON-object output request, verified in both upstream
bodies. Complete framing, prompts, selection, checker, profiles, byte ceilings
and 300-second idle/total deadlines stay fixed. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0/2 attempted.
Causes: 2 `generation-refused:error:chat.response.incomplete`. Final counters cover 2/2 and include
non-answer output; missing counters are not zero. GPU minutes are reserved job
wall including load, verification and cleanup; queues and wrapper overhead are
excluded, active kernel time unmeasured. The controller checkpoint let the pair
finish normally before stopping; no model or worker was signalled. Resume
tasks 3–4 without replay. Audit, helper reproduction, cleanup and archive
verification pass; see [evidence](t303-json-object-a4b-checkpoint-2026-11-13.md). No candidate tests, feedback,
retries, hidden or held-out tasks ran; native admission and original grading
controls remain blockers, driver unqualified. Finish A4B before tuning except
high/medium fixes. T-303/T-306 remain open; independent core review is required.

## A4B JSON-object checkpoint — 2026-11-13 (6/30, incomplete)

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. **6/30 tasks attempted; 24 remain.**
Available partial path: Rust source selection → credential-owning chat → unchanged
strict checker. Full retrieval, native execution, tests and retries remain
unavailable. These are ungraded proposals, not solved tasks.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gemma4-26b-a4b (6/30 attempted) | 32768 / 23172–29513 | Unavailable | Unavailable | 0/6 | Not run | 1202.848 | 21.379 | 44811 / 151605 |

Changed variable: trusted JSON-object output request, verified in all six upstream
bodies. Complete framing, prompts, selection, checker, profiles, byte ceilings
and 300-second idle/total deadlines stay fixed. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0/6 attempted.
Causes: 6 `generation-refused:error:chat.response.incomplete`. Final counters cover 6/6 and include
non-answer output; missing counters are not zero. GPU minutes are reserved job
wall including load, verification and cleanup; queues and wrapper overhead are
excluded, active kernel time unmeasured. The controller checkpoint let the pair
finish normally before stopping; no model or worker was signalled. Resume
tasks 7–8 without replay. Audit, helper reproduction, cleanup and archive
verification pass; see [evidence](t303-json-object-a4b-checkpoint-2026-11-13-6-tasks.md). No candidate tests, feedback,
retries, hidden or held-out tasks ran; native admission and original grading
controls remain blockers, driver unqualified. Finish A4B before tuning except
high/medium fixes. T-303/T-306 remain open; independent core review is required.

## JSON-object A4B development repeat — 2026-11-13

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`. Available partial path:
Rust source selection → credential-owning chat → unchanged strict checker;
full repository retrieval, native execution, tests and retries remain unavailable.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gemma4-26b-a4b | 32768 / 23004–29513 | Unavailable | Unavailable | 3/30 | Not run | 6052.758 | 108.798 | 215893 / 743394 |

Changed variable: trusted JSON-object output request, verified in all 30 upstream
bodies. Complete framing, prompts, source selection, strict checker, profiles,
byte ceilings and 300-second idle/total deadlines stay unchanged. Output remains
`min(32768, 32768 - rendered input tokens - 32)`; does not fit: 0.
Causes: 26 `generation-refused:error:chat.response.incomplete`, 1 `strict-edit-refused:code.proposal.format`, 3 `ungraded-proposal`.

Final backend counters cover 30/30 and include non-answer output.
GPU minutes are reserved job wall, including load/verification/cleanup and
excluding queues, wrapper overhead and pauses; active kernel time is not measured.
The changed driver tuple remains unqualified. No candidate tests, feedback,
retries or held-out tasks ran; native artifact and original grading-control
blockers remain. A strict proposal is not a solved task. No model role or
task-quality improvement is claimed. All six prepared profiles have completed this partial-path repeat.
The largest observed refusal cause selects the next work after review findings.
Source/record audit, helper reproduction, cleanup and archive verification pass;
see [evidence](t303-json-object-a4b-2026-11-13.md). T-303/T-306 remain open;
independent core review is required and no row closes.

## A4B thinking diagnostic — 2026-11-13 (two tasks per configuration)

Source: `40cad6472560e47314a55ddb1f45e2c4c22b1792`, using the previously frozen
API and helper. These are additional diagnostic attempts on dev-001/dev-002,
separate from the completed 180-attempt repeat. They do not form another full
suite or measure the newly added API thinking setting.

| Date | Model / server template option | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gemma4-26b-a4b / default | 32768 / 23172–23234 | Unavailable | Unavailable | 0/2 | Not run | 385.558 | 6.805 | 19066 / 46406 |
| 2026-11-13 | gemma4-26b-a4b / enable_thinking:false | 32768 / 23170–23232 | Unavailable | Unavailable | 2/2 | Not run | 54.061 | 1.282 | 19070 / 2893 |

Changed variable: the server template option only. JSON-object output, complete
framing, prompts, strict checks, 300-second idle/total deadlines and output formula
`min(32768, 32768 - rendered input tokens - 32)` stay fixed. The template choice
adds two rendered input tokens per task; does not fit: 0 in either pair.
Baseline causes: 2 `chat.response.incomplete`, both empty answers with separate
reasoning and output-limit termination. Disabled causes/outcomes: 2 strict
ungraded proposals, both normal stops. Counters cover all four attempts and
include non-answer output. No candidate tests, feedback, retries or held-out
execution ran. Native admission and original grading controls remain blockers.

GPU minutes are reserved job wall, including load, verification and cleanup,
excluding queues and wrapper overhead; active kernel time is unmeasured. Each
pair used a separate 900-second GPU reservation through both resource wrappers,
with both released between pairs. An earlier diagnostic startup used another
191.567 seconds / 3.193 GPU job minutes and ran zero tasks: its Unix socket path
was too long. That failed reservation is separate from both rows. No resource
limit was raised. The driver tuple remains unqualified. A two-task diagnostic
establishes no model role or task-quality improvement. The full post-change
development repeat remains pending; T-303/T-306 stay open.

## Trusted API thinking-control checkpoint — 2026-11-13 (2/30)

Source: `a896f055256e96baec09966c71c5d0a7f1714d96`. This checkpoint includes
**2/30 development tasks; the remaining 28 are queued under one resumable
controller**. It is a real check of the new API setting, separate from the
server-configuration diagnostic above. The available path remains source
selection → credential-owning chat → unchanged strict checker.

| Date | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU job min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | gemma4-26b-a4b (2/30 included) | 32768 / 23170–23232 | Unavailable | Unavailable | 2/2 | Not run | 55.335 | 1.534 | 19070 / 2893 |

Changed variable: trusted API `thinking_mode: disabled`, encoded as
`chat_template_kwargs: {"enable_thinking": false}` in both upstream requests.
The server retains empty default template arguments. Complete framing,
JSON-object output, prompts, strict checks, 300-second idle/total deadlines,
32K context and the output-budget formula remain fixed. Token preflight uses
the same template option; backend input counters agree. Does not fit: 0.
Both attempts stopped normally and produced strict ungraded proposals; no
incomplete response occurred in this pair. Counters cover 2/2 and include all
output. GPU minutes are reserved job wall, including load/verification/cleanup
and excluding queues and wrapper overhead; active kernel time is unmeasured.

No candidate tests, feedback, retries or held-out tasks ran. Native artifact
admission and original grading controls remain blockers. The driver remains
unqualified. No model role, task-quality improvement or row closure follows.
The completed first pair is skipped on resume; later pairs use both resource
wrappers with 900-second caps and 30-second pauses outside reservations.
Source review can run beside that measurement. Further tuning waits for the
full development repeat. The other five profiles have no new result here.
See [evidence](t303-thinking-control-2026-11-13.md); T-303/T-306 remain open.

## Thinking-disabled A4B development repeat — 2026-11-13

Source: `a896f055256e96baec09966c71c5d0a7f1714d96`. All 30 frozen development tasks ran through
source selection → credential-owning chat → unchanged strict checker. The earlier
2/30 API checkpoint is included, never added. This is the available partial path;
full repository retrieval, native effects, tests, feedback and retries remain
blocked by native admission and the original grading-control receipt.

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | a896f055256e96baec09966c71c5d0a7f1714d96 | gemma4-26b-a4b (30/30) | 32768 / 23002–29511 | Unavailable | Unavailable | 16/30 | Not run | 1289.452 | 29.820 | 215953 / 111638 |

Changed variable: trusted API `thinking_mode: disabled`; every captured request
carries `chat_template_kwargs: {"enable_thinking": false}`, while the server keeps
its default template arguments. Complete framing, JSON-object output, prompts,
checker, profiles, 300-second idle/total deadlines and the output-budget formula
remain fixed. Does not fit: 0. Outcomes: 16 ungraded proposals, 11 strict
`code.proposal.format` refusals and 3 `complete:length` generation refusals.
All three reach their output allowances; the other 27 backend stops are below
limits. Counter coverage: 30/30. No incomplete empty-answer response occurred.

Reserved work totals 1789.198 seconds; timers start after frozen-input
validation and include model/runtime verification, loading and cleanup. Queues,
wrapper/pre-entry overhead and session gaps are excluded. Active GPU kernel time
and continuous end-to-end elapsed time across sessions are unmeasured. Each pair
uses both resource wrappers with a 900-second cap and releases both; the resumed
controller pauses 30 seconds outside reservations between jobs. All owners exit.
No candidate, hidden or held-out test ran. First-attempt and retry task passes
remain unavailable; the driver/profile is unqualified. The other five profiles
still need fresh post-change measurements before tuning. See the
[evidence and stale-policy-label erratum](t303-thinking-disabled-a4b-2026-11-13.md).
T-303/T-306 remain open; independent core review is required.

## Thinking-disabled E4B development repeat — 2026-11-13

Source: `a896f055256e96baec09966c71c5d0a7f1714d96`. All 30 frozen development tasks ran through statement/editable-base
source selection → credential-owning chat → unchanged strict checker. This is the
available partial path; native admission and the original grading-control receipt
still block full retrieval, native effects, tests, feedback and retries.

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13 | a896f055256e96baec09966c71c5d0a7f1714d96 | gemma4-e4b (30/30) | 32768 / 23006–29515 | Unavailable | Unavailable | 16/30 | Not run | 608.793 | 12.150 | 215833 / 36730 |

Changed variable: trusted API `thinking_mode: disabled`, encoded in every upstream
request as `chat_template_kwargs: {"enable_thinking": false}`; the server retains
empty default template arguments. Complete framing, JSON-object output, prompts,
checker, profiles, 300-second idle/total deadlines and the output-budget formula
remain fixed. Does not fit: 0. Outcomes: `{"strict-edit-refused:code.proposal.edit": 6, "strict-edit-refused:code.proposal.format": 8, "ungraded-proposal": 16}`.
Backend counters cover 30/30; per-task times/tokens and raw captures are retained.

Reserved work totals 728.970 seconds, starting after frozen-input validation;
it includes load, verification, calls and cleanup. Queues, wrapper/pre-entry
overhead and session gaps are excluded. Active GPU kernel time and continuous
end-to-end elapsed time are unmeasured. Each two-task job uses both wrappers
with a 900-second cap and releases both, followed by a 30-second pause outside
reservations. All owners exit. No candidate, hidden or held-out test ran; task
passes remain unavailable and the driver/profile is unqualified. Four other
profiles still need fresh post-change measurements before tuning. See the
[evidence and metadata erratum](t303-thinking-disabled-e4b-2026-11-13.md).
T-303/T-306 remain open; independent core review is required.

## Thinking-disabled 31B development repeat — 2026-11-13–14 UTC

Source: `a896f055256e96baec09966c71c5d0a7f1714d96`. All 30 frozen development tasks ran through statement/editable-base
source selection → credential-owning chat → unchanged strict checker. This is the
available partial path; native admission and the original grading-control receipt
still block full retrieval, native effects, tests, feedback and retries.

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-13–14 | a896f055256e96baec09966c71c5d0a7f1714d96 | gemma4-31b (30/30) | 32768 / 23002–29511 | Unavailable | Unavailable | 26/30 | Not run | 1417.912 | 33.005 | 215953 / 31604 |

Changed variable: trusted API `thinking_mode: disabled`, encoded in every upstream
request as `chat_template_kwargs: {"enable_thinking": false}`; the prepared
server template retains its existing `enable_thinking: false`. Complete framing, JSON-object output, prompts,
checker, profiles, 300-second idle/total deadlines and the output-budget formula
remain fixed. Does not fit: 0. Outcomes: `{"strict-edit-refused:code.proposal.edit": 3, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 26}`.
Backend counters cover 30/30; per-task times/tokens and raw captures are retained.

Reserved work totals 1980.295 seconds, starting after frozen-input validation;
it includes load, verification, calls and cleanup. Queues, wrapper/pre-entry
overhead and session gaps are excluded. Active GPU kernel time and continuous
end-to-end elapsed time are unmeasured. Each two-task job uses both wrappers
with a 900-second execution cap after GPU admission and releases both, followed by a 30-second pause outside
reservations. All owners exit. No candidate, hidden or held-out test ran; task
passes remain unavailable and the driver/profile is unqualified. Three other
profiles still need fresh post-change measurements before tuning. See the
[evidence and metadata erratum](t303-thinking-disabled-31b-2026-11-13.md).
T-303/T-306 remain open; independent core review is required.

## Thinking-disabled Qwen-label development repeat — 2026-11-14 UTC

Source: `a896f055256e96baec09966c71c5d0a7f1714d96`. All 30 frozen development tasks ran through statement/editable-base
source selection → credential-owning chat → unchanged strict checker. This is the
available partial path; native admission and the original grading-control receipt
still block full retrieval, native effects, tests, feedback and retries.

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-14 | a896f055256e96baec09966c71c5d0a7f1714d96 | qwen35-27b (30/30) | 32768 / 24137–29804 | Unavailable | Unavailable | 29/30 | Not run | 1427.169 | 30.774 | 186402 / 34920 |

Changed variable: trusted API `thinking_mode: disabled`, encoded in every upstream
request as `chat_template_kwargs: {"enable_thinking": false}`; the prepared
server template retains its existing `enable_thinking: false`. Complete framing, JSON-object output, prompts,
checker, profiles, 300-second idle/total deadlines and the output-budget formula
remain fixed. Does not fit: 0. Outcomes: `{"generation-refused:error:chat.transport.timeout": 1, "ungraded-proposal": 29}`.
Backend counters cover 29/30; upstream response captures cover 29/30.
Missing response captures: `["dev-004"]`. Recorded timeouts and
available logs remain; unavailable final counters are never counted as zero.

Reserved work totals 1846.469 seconds, starting after frozen-input validation;
it includes load, verification, calls and cleanup. Queues, wrapper/pre-entry
overhead and session gaps are excluded. Active GPU kernel time and continuous
end-to-end elapsed time are unmeasured. Each two-task job uses both wrappers
with a 900-second execution cap after GPU admission and releases both, followed by a 30-second pause outside
reservations. All owners exit. No candidate, hidden or held-out test ran; task
passes remain unavailable and the driver/profile is unqualified. Two other
profiles still need fresh post-change measurements before tuning. See the
[evidence and metadata erratum](t303-thinking-disabled-qwen-2026-11-14.md).
T-303/T-306 remain open; independent core review is required.

## Independent Qwen-label measurement review retained — 2026-11-14

The [independent core review](../reviews/bd1249c0b5a7-thinking-disabled-qwen-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`bd1249c0b5a782c2fbdc0cacb4cd7cb2798174ae`, with no high or medium finding.
Two low documentation findings are corrected: the evidence's metadata erratum
now names `collector_policy.changed_variable`, and the single timeout uses
singular wording. All numbers and sealed inputs remain unchanged. T-303/T-306
stay open; this reporting review closes no row. GPT-OSS and Muse still need
fresh post-change repeats before tuning.

## Thinking-disabled GPT-OSS development repeat — 2026-11-14 UTC

Source: `a896f055256e96baec09966c71c5d0a7f1714d96`. All 30 frozen development tasks ran through statement/editable-base
source selection → credential-owning chat → unchanged strict checker. This is the
available partial path; native admission and the original grading-control receipt
still block full retrieval, native effects, tests, feedback and retries.

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-14 | a896f055256e96baec09966c71c5d0a7f1714d96 | gpt-oss (30/30) | 32768 / 24219–29748 | Unavailable | Unavailable | 11/30 | Not run | 2287.191 | 41.378 | 188409 / 431625 |

Changed variable: trusted API `thinking_mode: disabled`, encoded in every upstream
request as `chat_template_kwargs: {"enable_thinking": false}`; the prepared
server template retains `reasoning_effort: medium`. This records the requested
option without claiming model support. Complete framing, JSON-object output,
prompts, checker, profiles, 300-second idle/total deadlines and budget formula
remain fixed. Does not fit: 0. Outcomes: `{"generation-refused:error:chat.response.incomplete": 8, "strict-edit-refused:code.proposal.edit": 8, "strict-edit-refused:code.proposal.format": 3, "ungraded-proposal": 11}`.
Backend counters cover 30/30 including non-answer output; response captures
cover 30/30. Missing response captures: `[]`.
Unavailable final counters are never zero or reconstructed.

Reserved work totals 2482.685 seconds after frozen-input validation, including
verification, loading, calls and cleanup. Queues, wrapper/pre-entry overhead and
pauses are excluded. Active GPU kernel time and continuous end-to-end elapsed
time are unmeasured. Each two-task job uses both wrappers with a 900-second cap
after GPU admission and releases both before its 30-second pause. All owners
exit. No candidate, hidden or held-out test ran; task passes remain unavailable.
One profile still needs a fresh repeat before tuning. See the
[evidence and metadata erratum](t303-thinking-disabled-gpt-2026-11-14.md).
T-303/T-306 remain open; independent core review is required.

## Independent GPT-OSS measurement review retained — 2026-11-14

The [independent core review](../reviews/4c7ecf16c6d0-thinking-disabled-gpt-measurement.txt)
returns **Verdict: PASS** for exact reporting commit
`4c7ecf16c6d0301c29dc06c86929f5bc2c44b34d`, with no high or medium finding.
Two low documentation findings are corrected: the evidence now states that
30/30 captures contain reasoning and eight output-cap refusals have no answer
text; an unretained launch-preparation claim is dropped. Numbers and sealed
inputs remain unchanged. T-303/T-306 stay open; no row closes. Muse remains
the final fresh post-change repeat required before tuning and is queued.

## Thinking-disabled Muse checkpoint — 2026-11-15 UTC, 20/30

Source: `a896f055256e96baec09966c71c5d0a7f1714d96`. Incomplete cumulative development repeat:
**twenty attempted, ten pending**. Available path: statement/editable-base source
selection → credential-owning chat → unchanged strict checker. Native admission
and the original grading-control receipt still block full retrieval, effects,
tests, feedback and retries. Proposals are not task passes.

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-14–15 | a896f055256e96baec09966c71c5d0a7f1714d96 | muse (20/30; ten pending) | 32768 / 24395–29826 | Unavailable | Unavailable | 13/20 | Not run | 3304.704 | 62.228 | 111197 / 99327 (17/20 counters) |

Changed variable: trusted API `thinking_mode: disabled`, encoded as
`chat_template_kwargs: {"enable_thinking": false}`; the prepared server template
retains `reasoning_strength: medium`. Support is unclaimed. Other prompts,
checker, profiles, complete/JSON-object framing, deadlines and budget formula
are fixed. Does not fit: 0. Four strict edit refusals (two edit, two format),
three transport timeouts, thirteen strict ungraded proposals. Missing counters
and response captures for dev-004/011/012 are unavailable, never zero or rebuilt.

Reserved work totals 3733.673 seconds after frozen-input validation, including
verification, loading, calls and cleanup; queues, pre-entry overhead and pauses
are excluded. Active GPU kernel time and continuous elapsed time are unmeasured.
Two-task jobs use both wrappers with a 900-second cap after GPU admission and
30-second released pauses. All ten completed pairs revoke settings and reap
their owners. The next pair remained queued after 1332.798 seconds of build
wait, with no new task attempt. This wait is not a model failure. Read-only
prefix audit and archive verification pass; helper reproduction is not rerun.
No candidate, hidden or held-out test ran. Finish the ten pending tasks before
tuning; later full totals include this checkpoint, never add it. T-303/T-306
remain open; independent core review required. See the
[checkpoint evidence](t303-thinking-disabled-muse-checkpoint-2026-11-15.md).

## Independent twenty-task Muse checkpoint review retained — 2026-11-15

The [independent core review](../reviews/cc521088e3e8-thinking-disabled-muse-checkpoint20.txt)
returns **Verdict: PASS** for exact reporting commit
`cc521088e3e82ebd7597d2186cbcf01857e62226`, with no high or medium finding.
Its low disclosure finding is addressed: all 17 captured Muse responses contain
non-empty reasoning output despite the requested thinking option; the three
timeouts have no capture. Counts and sealed artifacts remain unchanged. This
accepts the cumulative 20/30 checkpoint only; ten tasks remain before tuning.
T-303/T-306 stay open and no row closes.

## Muse checkpoint: 22 of 30 attempts — 2026-11-15

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-14–15 | a896f055256e96baec09966c71c5d0a7f1714d96 | muse (22/30; eight pending) | 32768 / 24395–29826 | Unavailable | Unavailable | 14/22 | Not run | 3688.973 | 69.846 | 125470 / 115890 (19/22 counters) |

Cumulative checkpoint: includes the earlier twenty attempts, never added to them.
Changed variable remains trusted API `thinking_mode: disabled`; the prepared
Muse template retains `reasoning_strength: medium`. All nineteen retained
responses contain reasoning; model support for the requested option is unclaimed.
Complete framing, JSON-object output, prompts, checker, profile, deadlines and
budget formula are unchanged. Input 2910–8341; does not fit: 0. Causes: 14 ungraded
proposals, 3 format refusals, 2 edit refusals, 3 timeouts. Three missing captures and
counters remain unavailable. Timers exclude queues, wrapper/pre-entry overhead
and pauses; active GPU kernel time is unmeasured. Native admission and original
grading controls still block full retrieval, effects, tests, feedback and retries.
No candidate, hidden or held-out test ran. Read-only audit and archive verification
pass; original-helper reproduction is not rerun. The exact independent e24 review
is PASS with no findings; this checkpoint needs independent core review and closes
no row. Finish the remaining eight tasks before tuning. See the
[checkpoint evidence](t303-thinking-disabled-muse-checkpoint22-2026-11-15.md).

## Independent checkpoint-22 review retained — 2026-11-15

The [independent core review](../reviews/d1b408016c28-thinking-disabled-muse-checkpoint22.txt)
returns **Verdict: PASS** for exact reporting commit
`d1b408016c284e3aaf36e7546f0631ef89a19b0c`, with no findings at any severity.
This accepts the cumulative 22/30 Muse checkpoint only. Eight tasks remain
before tuning; task correctness and retries are still unavailable. Counts and
sealed artifacts remain unchanged. T-303/T-306 stay open; no row closes.

## Thinking-disabled Muse development repeat — 2026-11-14–15 UTC

Source: `a896f055256e96baec09966c71c5d0a7f1714d96`. All 30 frozen development tasks ran through statement/editable-base
source selection → credential-owning chat → unchanged strict checker. This is the
available partial path; native admission and the original grading-control receipt
still block full retrieval, native effects, tests, feedback and retries.

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-14–15 | a896f055256e96baec09966c71c5d0a7f1714d96 | muse (30/30) | 32768 / 24395–29826 | Unavailable | Unavailable | 19/30 | Not run | 4798.465 | 90.508 | 166355 / 150086 |

Changed variable: trusted API `thinking_mode: disabled`, encoded in every upstream
request as `chat_template_kwargs: {"enable_thinking": false}`; the prepared
server template retains `reasoning_strength: medium`. This records the requested
option without claiming model support. Complete framing, JSON-object output,
prompts, checker, profiles, 300-second idle/total deadlines and budget formula
remain fixed. Does not fit: 0. Outcomes: `{"generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 3, "strict-edit-refused:code.proposal.format": 4, "ungraded-proposal": 19}`.
Backend counters cover 26/30 including non-answer output; response captures
cover 27/30. Missing response captures: `["dev-004", "dev-011", "dev-012"]`.
Unavailable final counters are never zero or reconstructed.
Captures comprise 26 complete JSON responses and 1
truncated prefix (`dev-027`, 16384 bytes), not a completed response. Its final
counters are unavailable. Non-empty reasoning output appears in
26/26 complete retained responses despite the
requested option; missing captures and partial-response reasoning are unavailable.

Reserved work totals 5430.492 seconds after frozen-input validation, including
verification, loading, calls and cleanup. Queues, wrapper/pre-entry overhead and
pauses are excluded. Active GPU kernel time and continuous end-to-end elapsed
time are unmeasured. Each two-task job uses both wrappers with a 900-second cap
after GPU admission and releases both before its 30-second pause. All owners
exit. No candidate, hidden or held-out test ran; task passes remain unavailable.
Two jobs completed before the November 14 stop; thirteen completed after the
November 15 owner restart, skipping the four recorded attempts without replay.
All controller segments, the queued-session absence and stop receipts remain retained.
All six prepared profiles now have fresh post-change measurements. See the
[evidence and metadata erratum](t303-thinking-disabled-muse-2026-11-15.md).
T-303/T-306 remain open; independent core review is required.

All six profiles now cover 180 development attempts: 117/180 strict ungraded proposals. The largest refusal cause is `strict-edit-refused:code.proposal.format` (27/180). That measured cause sets the next work after high/medium review findings. Task correctness and retries remain unavailable; no held-out task ran.

## Inline-schema API checkpoint — 2026-11-15 UTC, 2/30

Source: `72c002a42957340de0b6fa0eb4d48385db2c7e62`. Two frozen development tasks
ran through source selection → credential-owning chat API → unchanged Rust strict
checker. Both produced **strict ungraded proposals**, not task passes. These
same two tasks also produced proposals in the preceding repeat; no improvement
is established. Twenty-eight tasks on this profile and all five other profiles
remain before further tuning.

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-15 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-26b-a4b (2/30; 28 pending) | 32768 / 23170–23232 | Unavailable | Unavailable | 2/2 | Not run | 64.846 | 1.708 | 19070 / 3665 (2/2 counters) |

Changed variable: trusted `response_format` selects a named inline edit schema,
encoded upstream as `json_schema` with `strict: true`. Captured bodies establish
the actual requests; support beyond these observations is unclaimed. Messages
and selected context are byte-identical to the preceding repeat. Prompts, helper,
checker, model/profile, complete framing, requested thinking option, 300-second
idle/total deadlines and output formula remain fixed. Inputs 9504–9566; does not
fit: 0. Both complete captures finish with `stop`; reasoning metadata is absent
and remains unavailable, never reconstructed as zero.

One earlier **preparation failure attempted zero tasks**: a 111-byte private
Unix socket path prevented the health client from connecting, although the
server log records loading in 28.209 seconds. Its startup deadline expired.
The failed preparation used 189.866 seconds /
3.164 reserved minutes; no task counters exist.
A short private namespace produces 87-byte socket paths. Only addressing
changed; no deadline, resource ceiling, priority or generation input changed.
The failed artifacts remain intact and no task was replayed. Combined reserved
work, including that failure, is **292.327 seconds / 4.872 minutes**.

Reserved-work timers start after frozen-input validation and exclude queues,
wrapper/pre-entry overhead and pauses. Active GPU kernel time and continuous
elapsed time are unmeasured. The completed job revokes settings and reaps API/model
owners; keys/sockets are absent and two API tokens are retired. Process scanning
covers this PID namespace only. Reservation-wrapper exit statuses are unavailable;
completed worker reporting and owner absence are observed, not substituted for
those statuses. Native admission and original grading controls still block full
retrieval, effects, tests, feedback and retries. No candidate, hidden or held-out
test ran. T-303/T-306 remain open; independent core review and the full fresh
six-profile repeat are required. See the [source and checkpoint evidence](t303-inline-output-schema-2026-11-15.md).

## Inline-schema A4B checkpoint — 2026-11-15–16 UTC, 22/30

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-15–16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-26b-a4b (22/30; 8 pending) | 32768 / 23002–29511 | Unavailable | Unavailable | 13/22 | Not run | 755.953 | 19.594 | 159313 / 52532 (22/22 counters) |

Changed generation variable: trusted named inline `json_schema` request with
`strict: true`. Contexts/messages, helper, prompts, strict checker, model/profile,
framing, thinking request, deadlines and output formula match the preceding
repeat. Input 3225–9734; does not fit: 0. Causes: thirteen strict ungraded proposals,
nine `code.proposal.edit` refusals and zero format refusals. These are not task
passes. Admission and original grading controls still block native execution,
tests, feedback and retries. No candidate, hidden or held-out test ran.

All 22 captures are complete JSON; reasoning metadata is absent/unavailable,
never zero. Reserved-work timers total 1175.639 seconds after frozen-input
validation, excluding queues, wrapper/pre-entry overhead and pauses. Active GPU
kernel time and continuous elapsed time remain unmeasured. The earlier zero-task
startup failure adds 189.866 seconds / 3.164 reserved minutes;
combined work is 1365.504 seconds / 22.758 minutes. No task/retry is added.
The earlier 2/30 checkpoint is included, never added to this cumulative prefix.

The last pair ended queued before any task attempt; exact owner/queue stop
receipts and cleanup are retained. Actual host low priority, nice 19, all limits
and both wrappers stayed fixed. Read-only audit and retained-artifact verification
pass; independent checker reproduction is not rerun. Eight A4B tasks and five
full profile repeats remain before tuning or new capabilities. T-303/T-306 remain
open; independent core review is required. See [checkpoint evidence](t303-inline-schema-a4b-checkpoint22-2026-11-16.md).

## Inline-schema A4B full development repeat — 2026-11-15–16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-15–16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-26b-a4b (30/30) | 32768 / 23002–29511 | Unavailable | Unavailable | 19/30 | Not run | 1201.878 | 30.189 | 215953 / 94310 (30/30 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, helper, prompts, strict checker, model/profile, framing,
thinking request, deadlines and output formula match the preceding repeat.
Input 3225–9734; does not fit: 0. Causes:
`{"generation-refused:complete:length": 1, "strict-edit-refused:code.proposal.edit": 10, "ungraded-proposal": 19}`. Capture counts:
`{"complete-choice-json": 30}`; reasoning metadata:
`{"unavailable": 30}`. Missing metadata is not zero.

These are generation/checker outcomes, not task passes. Native admission and
original grading controls still block effects, tests, feedback and retries;
no candidate, hidden or held-out test ran. Prior 2/30 and 22/30 counts are included,
never added. Five other full profiles remain before tuning or capability work.

Reserved-work timers total 1811.358 seconds after frozen-input validation,
excluding queues, wrapper/pre-entry overhead and released pauses. GPU kernel
time and continuous elapsed time remain unmeasured. The earlier zero-task startup
failure adds 189.866 seconds / 3.164 minutes; combined reserved work
is 2001.224 seconds / 33.354 minutes. No task or retry is
added for that failure. Both wrappers release between pairs; resource limits and
host low priority remain fixed. Audit, wire/context comparison, original-helper
reproduction and cleanup pass. T-303/T-306 remain open; independent core review
is required. See [full evidence](t303-inline-schema-a4b-2026-11-16.md).

## Inline-schema E4B development repeat — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-e4b (2/30; 28 pending) | 32768 / 23174–23236 | Unavailable | Unavailable | 1/2 | Not run | 54.476 | 1.084 | 19062 / 4081 (2/2 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding repeat. Input 9500–9562; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 1}`. Captures:
`{"complete-choice-json": 2}`; reasoning metadata:
`{"unavailable": 2}`. Missing metadata is not zero.

These are generation/checker outcomes. Task passes and retries remain unavailable:
native admission and original grading controls still block effects, tests,
feedback and retries. No candidate, hidden or held-out test ran.
The remaining 28 E4B tasks and four other full profiles (120 attempts) remain before tuning or capability work.

Reserved-work timers total 65.061 seconds after frozen-input validation,
excluding queues, wrapper/pre-entry overhead and released pauses. GPU kernel time
and continuous elapsed time remain unmeasured. Both wrappers release between
two-task jobs; resource limits and host low priority remain fixed. Audit,
comparison, original-helper reproduction and cleanup pass. T-303/T-306 remain
open; independent core review is required. See [evidence](t303-inline-schema-e4b-2026-11-16.md).

The next pair was cancelled while waiting for shared build capacity, before GPU
entry or any further attempt. The completed prefix is 2/30; 28 E4B tasks remain
unattempted. Queue cancellation adds no model failure or retry.

## Inline-schema E4B resumed development repeat — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-e4b (4/30; 26 pending) | 32768 / 23174–29515 | Unavailable | Unavailable | 3/4 | Not run | 156.464 | 3.064 | 27033 / 12342 (4/4 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding repeat. Input 3221–9562; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 3}`. Captures:
`{"complete-choice-json": 4}`; reasoning metadata:
`{"unavailable": 4}`. Missing metadata is not zero.

These are generation/checker outcomes. Task passes and retries remain unavailable:
native admission and original grading controls still block effects, tests,
feedback and retries. No candidate, hidden or held-out test ran.
The remaining 26 E4B tasks and four other full profiles (120 attempts) remain before tuning or capability work.
These cumulative totals include the earlier two attempts; checkpoints are never added.

Reserved-work timers total 183.857 seconds after frozen-input validation,
excluding queues, wrapper/pre-entry overhead and released pauses. GPU kernel time
and continuous elapsed time remain unmeasured. Both wrappers release between
two-task jobs; resource limits and host low priority remain fixed. Audit,
comparison, original-helper reproduction and cleanup pass. T-303/T-306 remain
open; independent core review is required. See [evidence](t303-inline-schema-e4b-resumed-2026-11-16.md).

The next pair was cancelled while queued for independent checkpoint review,
before GPU entry or any further task. Its queue time is excluded from model
work; it adds no model failure or retry. The 26 pending tasks stay unattempted.

## Inline-schema E4B continued development repeat — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-e4b (8/30; 22 pending) | 32768 / 23006–29515 | Unavailable | Unavailable | 7/8 | Not run | 327.197 | 6.196 | 63899 / 28399 (8/8 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding repeat. Input 3221–9730; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 7}`. Captures:
`{"complete-choice-json": 8}`; reasoning metadata:
`{"unavailable": 8}`. Missing metadata is not zero.

These are generation/checker outcomes. Task passes and retries remain unavailable:
native admission and original grading controls still block effects, tests,
feedback and retries. No candidate, hidden or held-out test ran.
The remaining 22 E4B tasks and four other full profiles (120 attempts) remain before tuning or capability work.
These cumulative totals include the earlier four attempts; checkpoints are never added.

Reserved-work timers total 371.782 seconds after frozen-input validation,
excluding queues, wrapper/pre-entry overhead and released pauses. GPU kernel time
and continuous elapsed time remain unmeasured. Both wrappers release between
two-task jobs; resource limits and host low priority remain fixed. Audit,
comparison, original-helper reproduction and cleanup pass. T-303/T-306 remain
open; independent core review is required. See [evidence](t303-inline-schema-e4b-continuation-2026-11-16.md).

The next pair was cancelled while queued, before GPU entry or any further task.
Its queue time is excluded from model work and adds no model failure or retry.
The 22 pending tasks stay unattempted.

## Inline-schema E4B further development repeat — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-e4b (10/30; 20 pending) | 32768 / 23006–29515 | Unavailable | Unavailable | 9/10 | Not run | 350.444 | 6.763 | 71621 / 28867 (10/10 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding repeat. Input 3221–9730; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 1, "ungraded-proposal": 9}`. Captures:
`{"complete-choice-json": 10}`; reasoning metadata:
`{"unavailable": 10}`. Missing metadata is not zero.

These are generation/checker outcomes. Task passes and retries remain unavailable:
native admission and original grading controls still block effects, tests,
feedback and retries. No candidate, hidden or held-out test ran.
The remaining 20 E4B tasks and four other full profiles (120 attempts) remain before tuning or capability work.
These cumulative totals include the earlier eight attempts; checkpoints are never added.

Reserved-work timers total 405.790 seconds after frozen-input validation,
excluding queues, wrapper/pre-entry overhead and released pauses. GPU kernel time
and continuous elapsed time remain unmeasured. Both wrappers release between
two-task jobs; resource limits and host low priority remain fixed. Audit,
comparison, original-helper reproduction and cleanup pass. T-303/T-306 remain
open; independent core review is required. See [evidence](t303-inline-schema-e4b-progress-2026-11-16.md).

The next pair was cancelled while queued, before GPU entry or any further task.
Its queue time is excluded from model work and adds no model failure or retry.
The 20 pending tasks stay unattempted.

## Inline-schema E4B fourteen-task development checkpoint — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-e4b (14/30; 16 pending) | 32768 / 23006–29515 | Unavailable | Unavailable | 11/14 | Not run | 459.052 | 8.869 | 98269 / 37885 (14/14 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding repeat. Input 3221–9730; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 11}`. Captures:
`{"complete-choice-json": 14}`; reasoning metadata:
`{"unavailable": 14}`. Missing metadata is not zero.

These are generation/checker outcomes. Task passes and retries remain unavailable:
native admission and original grading controls still block effects, tests,
feedback and retries. No candidate, hidden or held-out test ran.
The remaining 16 E4B tasks and four other full profiles (120 attempts) remain before tuning or capability work.
These cumulative totals include the earlier ten attempts; checkpoints are never added.

Reserved-work timers total 532.169 seconds after frozen-input validation,
excluding queues, wrapper/pre-entry overhead and released pauses. GPU kernel time
and continuous elapsed time remain unmeasured. Both wrappers release between
two-task jobs; resource limits and host low priority remain fixed. Audit,
comparison, original-helper reproduction and cleanup pass. T-303/T-306 remain
open; independent core review is required. See [evidence](t303-inline-schema-e4b-next-2026-11-16.md).

The next pair was cancelled while queued, before GPU entry or any further task.
Its queue time is excluded from model work and adds no model failure or retry.
The 16 pending tasks stay unattempted.

## Inline-schema E4B continued development measurement — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-e4b (18/30; 12 pending) | 32768 / 23006–29515 | Unavailable | Unavailable | 12/18 | Not run | 560.341 | 10.882 | 124924 / 43182 (18/18 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding repeat. Input 3221–9730; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 6, "ungraded-proposal": 12}`. Captures:
`{"complete-choice-json": 18}`; reasoning metadata:
`{"unavailable": 18}`. Missing metadata is not zero.

These are generation/checker outcomes. Task passes and retries remain unavailable:
native admission and original grading controls still block effects, tests,
feedback and retries. No candidate, hidden or held-out test ran.
The remaining 12 E4B tasks and four other full profiles (120 attempts) remain before tuning or capability work.
These cumulative totals include the earlier fourteen attempts; checkpoints are never added.

Reserved-work timers total 652.940 seconds after frozen-input validation,
excluding queues, wrapper/pre-entry overhead and released pauses. GPU kernel time
and continuous elapsed time remain unmeasured. Both wrappers release between
two-task jobs; resource limits and host low priority remain fixed. Audit,
comparison, original-helper reproduction and cleanup pass. T-303/T-306 remain
open; independent core review is required. See [evidence](t303-inline-schema-e4b-continued-2026-11-16.md).

The next pair was cancelled while queued, before GPU entry or any further task.
Its queue time is excluded from model work and adds no model failure or retry.
The 12 pending tasks stay unattempted.

## Inline-schema E4B twenty-four-task checkpoint — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-e4b (24/30; 6 pending) | 32768 / 23006–29515 | Unavailable | Unavailable | 16/24 | Not run | 798.299 | 15.339 | 174031 / 60931 (24/24 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding repeat. Input 3221–9730; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 8, "ungraded-proposal": 16}`. Captures:
`{"complete-choice-json": 24}`; reasoning metadata:
`{"unavailable": 24}`. Missing metadata is not zero.

These are generation/checker outcomes. Task passes and retries remain unavailable:
native admission and original grading controls still block effects, tests,
feedback and retries. No candidate, hidden or held-out test ran.
The remaining 6 E4B tasks and four other full profiles (120 attempts) remain before tuning or capability work.
These cumulative totals include the earlier eighteen attempts; checkpoints are never added.

Reserved-work timers total 920.325 seconds after frozen-input validation,
excluding queues, wrapper/pre-entry overhead and released pauses. GPU kernel time
and continuous elapsed time remain unmeasured. Both wrappers release between
two-task jobs; resource limits and host low priority remain fixed. Audit,
comparison, original-helper reproduction and cleanup pass. T-303/T-306 remain
open; independent core review is required. See [evidence](t303-inline-schema-e4b-checkpoint24-2026-11-16.md).

The next pair was cancelled while queued, before GPU entry or any further task.
Its queue time is excluded from model work and adds no model failure or retry.
The 6 pending tasks stay unattempted.

## Inline-schema E4B complete development measurement — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-e4b (30/30) | 32768 / 23006–29515 | Unavailable | Unavailable | 17/30 | Not run | 960.589 | 18.482 | 215833 / 73110 (30/30 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding repeat. Input 3221–9730; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 13, "ungraded-proposal": 17}`. Captures:
`{"complete-choice-json": 30}`; reasoning metadata:
`{"unavailable": 30}`. Missing metadata is not zero.

These are generation/checker outcomes. Task passes and retries remain unavailable:
native admission and original grading controls still block effects, tests,
feedback and retries. No candidate, hidden or held-out test ran.
Four other full profiles (120 attempts) remain before tuning or capability work.
These cumulative totals include the earlier twenty-four attempts; checkpoints are never added.

Reserved-work timers total 1108.945 seconds after frozen-input validation,
excluding queues, wrapper/pre-entry overhead and released pauses. GPU kernel time
and continuous elapsed time remain unmeasured. Both wrappers release between
two-task jobs; resource limits and host low priority remain fixed. Audit,
comparison, original-helper reproduction and cleanup pass. T-303/T-306 remain
open; independent core review is required. See [evidence](t303-inline-schema-e4b-complete-2026-11-16.md).

## Inline-schema 31B development measurement — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-31b (2/30; 28 pending) | 32768 / 23170–23232 | Unavailable | Unavailable | 0/2 | Not run | 133.147 | 2.964 | 19070 / 3121 (2/2 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding 31B repeat. Input 9504–9566; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 2}`. Captures:
`{"complete-choice-json": 2}`; reasoning metadata:
`{"unavailable": 2}`. Missing metadata is not zero.

Task passes and retries remain unavailable: native admission and original grading
controls still block effects, tests, feedback and retries. No candidate, hidden
or held-out test ran. The remaining 28 31B tasks and three full profiles (118 attempts) remain before tuning
or capability work. Earlier checkpoints are never added or replayed.

Reserved work totals 177.860 seconds after frozen-input validation, excluding
queues, wrapper/pre-entry overhead and released pauses. GPU kernel time and
continuous elapsed time are unmeasured. Both wrappers release between two-task
jobs; resource limits and host low priority stay fixed. Audit, comparison,
original-helper reproduction and cleanup pass. T-303/T-306 remain open;
independent core review is required. See [evidence](t303-inline-schema-31b-2026-11-16.md).

The next pair was stopped after 667.278 seconds waiting for shared build capacity, before GPU entry or a new attempt. Its exact own session was authenticated, signalled and rescanned empty. Queue time adds no model failure or retry. The 28 unattempted 31B tasks remain pending.

## Inline-schema 31B continued development measurement — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-31b (4/30; 26 pending) | 32768 / 23170–29511 | Unavailable | Unavailable | 1/4 | Not run | 201.006 | 4.734 | 27049 / 4610 (4/4 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding 31B repeat. Input 3225–9566; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 3, "ungraded-proposal": 1}`. Captures:
`{"complete-choice-json": 4}`; reasoning metadata:
`{"unavailable": 4}`. Missing metadata is not zero.

Task passes and retries remain unavailable: native admission and original grading
controls still block effects, tests, feedback and retries. No candidate, hidden
or held-out test ran. The remaining 26 31B tasks and three full profiles (116 attempts) remain before tuning
or capability work. These cumulative totals include the two prior attempts; checkpoints are never added or replayed.

Reserved work totals 284.010 seconds after frozen-input validation, excluding
queues, wrapper/pre-entry overhead and released pauses. GPU kernel time and
continuous elapsed time are unmeasured. Both wrappers release between two-task
jobs; resource limits and host low priority stay fixed. Audit, comparison,
original-helper reproduction and cleanup pass. T-303/T-306 remain open;
independent core review is required. See [evidence](t303-inline-schema-31b-continued-2026-11-16.md).

The next pair was stopped after 291.498 seconds waiting for shared build capacity, before GPU entry or a new attempt. Its exact own session was authenticated, signalled and rescanned empty. Queue time adds no model failure or retry. The 26 unattempted 31B tasks remain pending.

## Inline-schema 31B resumed development measurement — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-31b (6/30; 24 pending) | 32768 / 23170–29511 | Unavailable | Unavailable | 2/6 | Not run | 305.215 | 7.620 | 44823 / 5522 (6/6 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding 31B repeat. Input 3225–9566; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 4, "ungraded-proposal": 2}`. Captures:
`{"complete-choice-json": 6}`; reasoning metadata:
`{"unavailable": 6}`. Missing metadata is not zero.

Task passes and retries remain unavailable: native admission and original grading
controls still block effects, tests, feedback and retries. No candidate, hidden
or held-out test ran. The remaining 24 31B tasks and three full profiles (114 attempts) remain before tuning
or capability work. These cumulative totals include the four prior attempts; checkpoints are never added or replayed.

Reserved work totals 457.192 seconds after frozen-input validation, excluding
queues, wrapper/pre-entry overhead and released pauses. GPU kernel time and
continuous elapsed time are unmeasured. Both wrappers release between two-task
jobs; resource limits and host low priority stay fixed. Audit, comparison,
original-helper reproduction and cleanup pass. T-303/T-306 remain open;
independent core review is required. See [evidence](t303-inline-schema-31b-resumed-2026-11-16.md).

The next pair was stopped after 280.559 seconds waiting for shared build capacity, before GPU entry or a new attempt. Its exact own session was authenticated, signalled and rescanned empty. Queue time adds no model failure or retry. The 24 unattempted 31B tasks remain pending.

## Inline-schema 31B development progress — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-31b (18/30; 12 pending) | 32768 / 23002–29511 | Unavailable | Unavailable | 9/18 | Not run | 1042.136 | 23.511 | 124996 / 22993 (18/18 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding 31B repeat. Input 3225–9734; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 9, "ungraded-proposal": 9}`. Captures:
`{"complete-choice-json": 18}`; reasoning metadata:
`{"unavailable": 18}`. Missing metadata is not zero.

Task passes and retries remain unavailable: native admission and original grading
controls still block effects, tests, feedback and retries. No candidate, hidden
or held-out test ran. The remaining 12 31B tasks and three full profiles (102 attempts) remain before tuning
or capability work. These cumulative totals include the six prior attempts; checkpoints are never added or replayed.

Reserved work totals 1410.662 seconds after frozen-input validation, excluding
queues, wrapper/pre-entry overhead and released pauses. GPU kernel time and
continuous elapsed time are unmeasured. Both wrappers release between two-task
jobs; resource limits and host low priority stay fixed. Audit, comparison,
original-helper reproduction and cleanup pass. T-303/T-306 remain open;
independent core review is required. See [evidence](t303-inline-schema-31b-progress-2026-11-16.md).

The next pair was stopped after 1238.925 seconds waiting for shared build capacity, before GPU entry or a new attempt. Its exact own session was authenticated, signalled and rescanned empty. Queue time adds no model failure or retry. The 12 unattempted 31B tasks remain pending.

## Inline-schema 31B development checkpoint — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-31b (26/30; 4 pending) | 32768 / 23002–29511 | Unavailable | Unavailable | 16/26 | Not run | 1448.612 | 33.171 | 184337 / 31288 (26/26 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding 31B repeat. Input 3225–9734; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 10, "ungraded-proposal": 16}`. Captures:
`{"complete-choice-json": 26}`; reasoning metadata:
`{"unavailable": 26}`. Missing metadata is not zero.

Task passes and retries remain unavailable: native admission and original grading
controls still block effects, tests, feedback and retries. No candidate, hidden
or held-out test ran. The remaining 4 31B tasks and three full profiles (94 attempts) remain before tuning
or capability work. These cumulative totals include the 18 prior attempts; checkpoints are never added or replayed.

Reserved work totals 1990.235 seconds after frozen-input validation, excluding
queues, wrapper/pre-entry overhead and released pauses. GPU kernel time and
continuous elapsed time are unmeasured. Both wrappers release between two-task
jobs; resource limits and host low priority stay fixed. Audit, comparison,
original-helper reproduction and cleanup pass. T-303/T-306 remain open;
independent core review is required. See [evidence](t303-inline-schema-31b-resume18-2026-11-16.md).

The next pair was stopped after 1484.618 seconds waiting for shared build capacity, before GPU entry or a new attempt. Its exact own session was authenticated, signalled and rescanned empty. Queue time adds no model failure or retry. The 4 unattempted 31B tasks remain pending.

## Inline-schema 31B development completion — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gemma4-31b (30/30) | 32768 / 23002–29511 | Unavailable | Unavailable | 17/30 | Not run | 1620.652 | 37.519 | 215953 / 34733 (30/30 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
complete framing, requested thinking option, deadlines and output formula match
the preceding 31B repeat. Input 3225–9734; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 13, "ungraded-proposal": 17}`. Captures:
`{"complete-choice-json": 30}`; reasoning metadata:
`{"unavailable": 30}`. Missing metadata is not zero.

Task passes and retries remain unavailable: native admission and original grading
controls still block effects, tests, feedback and retries. No candidate, hidden
or held-out test ran. Three full profiles (90 attempts) remain before tuning
or capability work. These cumulative totals include the 26 prior attempts; checkpoints are never added or replayed.

Reserved work totals 2251.151 seconds after frozen-input validation, excluding
queues, wrapper/pre-entry overhead and released pauses. GPU kernel time and
continuous elapsed time are unmeasured. Both wrappers release between two-task
jobs; resource limits and host low priority stay fixed. Audit, comparison,
original-helper reproduction and cleanup pass. T-303/T-306 remain open;
independent core review is required. See [evidence](t303-inline-schema-31b-finish-2026-11-16.md).

## Inline-schema 27B development measurement — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | qwen35-27b (30/30) | 32768 / 24137–29804 | Unavailable | Unavailable | 28/30 | Not run | 1260.537 | 27.771 | 189334 / 37798 (30/30 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
framing, thinking request, deadlines and output formula match the preceding
27B repeat. Input 2932–8599; does not fit: 0.
Causes: `{"strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 28}`. Captures:
`{"complete-choice-json": 30}`; reasoning metadata:
`{"unavailable": 30}`. Missing metadata is not zero.

Native admission/original grading controls still block effects, project tests,
feedback and retries. Task passes remain unavailable. No candidate, hidden or
held-out test ran. 60 attempts remain before tuning or new capabilities.
Earlier checkpoints are never added or replayed. All original frozen input
bindings pass against the complete read-only Git-backed source snapshot;
the licence batch's changed working-tree bytes are outside this measurement.
All 30 development contexts were reproduced before inference.

Completed-job reserved work 1666.284 seconds excludes queues, pre-entry
validation, wrapper overhead and released pauses. The recorded controller
launch-to-exit UTC interval is 3800.162 seconds including queues and
released pauses; outer authentication, preflight and reporting are outside it.
Earlier failed preparations
used another 18.443 plus 44.628 seconds and zero task attempts, recorded
separately. GPU kernel time and complete end-to-end latency are unmeasured.
Audit, comparison, original-helper reproduction and cleanup pass.
T-303/T-306 remain open. See [evidence](t303-inline-schema-qwen-2026-11-16.md).

## Inline-schema gpt-oss development measurement — 2026-11-16 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-16 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gpt-oss (4/30) | 32768 / 24497–29748 | Unavailable | Unavailable | 0/4 | Not run | 500.353 | 8.863 | 23752 / 88912 (4/4 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
framing, thinking request, deadlines and output formula match the preceding
gpt-oss repeat. Profile `reasoning_effort: medium` and the transport's thinking
disabled request are unchanged; internal reasoning is not inferred.
Input 2988–8239; does not fit: 0.
Causes: `{"generation-refused:error:chat.response.incomplete": 3, "strict-edit-refused:code.proposal.edit": 1}`. Captures:
`{"complete-choice-json": 4}`; reasoning metadata:
`{"non-empty": 4}`. Missing metadata is not zero.

Native admission/original grading controls still block effects, project tests,
feedback and retries. Task passes remain unavailable. No candidate, hidden or
held-out test ran. 56 attempts remain before tuning or new capabilities.
Earlier attempts are never added or replayed. All original frozen input bindings
pass against the complete read-only Git-backed source snapshot; the licence
batch's working-tree changes are outside this measurement. All 30 contexts
were reproduced before inference against this model's own preceding repeat.

Completed-job reserved work 531.759 seconds excludes queues, pre-entry
validation, wrapper overhead and released pauses. The recorded controller
launch-to-exit UTC interval is 623.220 seconds including queues and
released pauses; outer authentication, preflight and reporting are outside it.
GPU kernel time and complete end-to-end latency are unmeasured.
Audit, comparison, original-helper reproduction and cleanup pass.
T-303/T-306 remain open. See [evidence](t303-inline-schema-gpt-2026-11-16.md).

The builder requested a checkpoint through the exact controller's stop flag. The active second pair finished normally; the next pair never started. No model attempt was cancelled. The stop method, process identity, request and exit are retained for a resume without replay.

## Inline-schema gpt-oss development completion — 2026-11-17 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-17 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | gpt-oss (30/30) | 32768 / 24219–29748 | Unavailable | Unavailable | 6/30 | Not run | 2605.278 | 47.321 | 188409 / 470692 (30/30 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
framing, thinking request, deadlines and output formula match the preceding
gpt-oss repeat. Profile `reasoning_effort: medium` and the transport's thinking
disabled request are unchanged; internal reasoning is not inferred.
Input 2988–8517; does not fit: 0.
Causes: `{"generation-refused:complete:length": 1, "generation-refused:error:chat.response.incomplete": 8, "strict-edit-refused:code.proposal.edit": 15, "ungraded-proposal": 6}`. Captures:
`{"complete-choice-json": 30}`; reasoning metadata:
`{"non-empty": 30}`. Missing metadata is not zero.

Native admission/original grading controls still block effects, project tests,
feedback and retries. Task passes remain unavailable. No candidate, hidden or
held-out test ran. 30 attempts remain before tuning or new capabilities.
Counts are cumulative: the first four attempts are carried once and never
replayed. All original frozen input bindings pass against the complete
read-only Git-backed source snapshot; the licence
batch's working-tree changes are outside this measurement. All 30 contexts
were reproduced before inference against this model's own preceding repeat.

Completed-job reserved work 2839.246 seconds excludes queues, pre-entry
validation, wrapper overhead and released pauses. The sum of recorded controller
launch-to-exit UTC intervals is 8257.204 seconds including queues and
released pauses; the gap between sessions, outer authentication, preflight
and reporting are outside it.
The preceding segment is 623.220 seconds and the resumed
segment is 7633.984 seconds. GPU kernel time and complete
end-to-end latency are unmeasured.
Observed response field combinations: `{"empty content / non-empty reasoning_content": 8, "non-empty content / non-empty reasoning_content": 22}`.
These describe captured fields; internal reasoning is not inferred.
Audit, comparison, original-helper reproduction and cleanup pass.
T-303/T-306 remain open. See [evidence](t303-inline-schema-gpt-complete-2026-11-17.md).

## Inline-schema muse development measurement — 2026-11-17 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-17 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | muse (4/30) | 32768 / 24509–29805 | Unavailable | Unavailable | 1/4 | Not run | 771.168 | 15.096 | 20645 / 19594 (3/4 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
framing, thinking request, deadlines and output formula match the preceding
muse repeat. Profile `reasoning_strength: medium` and the transport's thinking
disabled request are unchanged; internal reasoning is not inferred.
Input 2931–8227; does not fit: 0.
Causes: `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 2, "ungraded-proposal": 1}`. Captures:
`{"complete-choice-json": 3, "missing": 1}`; reasoning metadata:
`{"non-empty": 3}`. Missing metadata is not zero.

Native admission/original grading controls still block effects, project tests,
feedback and retries. Task passes remain unavailable. No candidate, hidden or
held-out test ran. 26 attempts remain before tuning or new capabilities.
Earlier attempts are never added or replayed. All original frozen input bindings
pass against the complete read-only Git-backed source snapshot; the licence
batch's working-tree changes are outside this measurement. All 30 contexts
were reproduced before inference against this model's own preceding repeat.

Completed-job reserved work 905.766 seconds excludes queues, pre-entry
validation, wrapper overhead and released pauses. The recorded controller
launch-to-exit UTC interval is 2169.571 seconds including queues and
released pauses; outer authentication, preflight and reporting are outside it.
GPU kernel time and complete end-to-end latency are unmeasured.
Audit, comparison, original-helper reproduction and cleanup pass.
T-303/T-306 remain open. See [evidence](t303-inline-schema-muse-2026-11-17.md).

The builder requested a checkpoint through the exact controller's stop flag. The active second pair finished normally; the next pair never started. The checkpoint stop did not interrupt an attempt; original request deadlines still apply. The stop method, process identity, request and exit are retained for a resume without replay.

Observed response field combinations: `{"non-empty content / non-empty reasoning_content": 3, "unavailable content / unavailable reasoning_content": 1}`. Internal reasoning is not inferred.

## Inline-schema Muse development checkpoint8 — 2026-11-17 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-17 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | muse (8/30) | 32768 / 24412–29805 | Unavailable | Unavailable | 4/8 | Not run | 1444.583 | 27.752 | 51476 / 47181 (7/8 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
framing, thinking request, deadlines and output formula match the preceding
muse repeat. Profile `reasoning_strength: medium` and the transport's thinking
disabled request are unchanged; internal reasoning is not inferred.
Input 2931–8324; does not fit: 0.
Causes: `{"generation-refused:error:chat.transport.timeout": 1, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 1, "ungraded-proposal": 4}`. Captures:
`{"complete-choice-json": 7, "missing": 1}`; reasoning metadata:
`{"non-empty": 7}`. Missing metadata is not zero.

Native admission/original grading controls still block effects, project tests,
feedback and retries. Task passes remain unavailable. No candidate, hidden or
held-out test ran. 22 attempts remain before tuning or new capabilities.
Earlier attempts are preserved and counted once; none is replayed. All original frozen input bindings
pass against the complete read-only Git-backed source snapshot; the licence
batch's working-tree changes are outside this measurement. All 30 contexts
were reproduced before inference against this model's own preceding repeat.

Completed-job reserved work 1665.107 seconds excludes queues, pre-entry
validation, wrapper overhead and released pauses. The sum of recorded controller
launch-to-exit UTC intervals is 5606.979 seconds including queues and
released pauses; the gap between sessions, outer authentication, preflight
and reporting are outside it.
The preceding segment is 2169.571 seconds and the resumed
segment is 3437.408 seconds. GPU kernel time and complete
end-to-end latency are unmeasured.
Observed response field combinations: `{"non-empty content / non-empty reasoning_content": 7, "unavailable content / unavailable reasoning_content": 1}`.
These describe captured fields; internal reasoning is not inferred.
Audit, comparison, original-helper reproduction and cleanup pass.
T-303/T-306 remain open. See [evidence](t303-inline-schema-muse-checkpoint8-2026-11-17.md).

The builder requested a checkpoint through the exact controller's stop flag. The active second resumed pair finished normally; the next pair never started. No model attempt was cancelled by the checkpoint. Original request deadlines still apply. The stop method, process identity, request and exit are retained for a resume without replay.

## Inline-schema Muse development checkpoint14 — 2026-11-17 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-17 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | muse (14/30) | 32768 / 24403–29826 | Unavailable | Unavailable | 7/14 | Not run | 2689.397 | 49.523 | 72544 / 75130 (11/14 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
framing, thinking request, deadlines and output formula match the preceding
muse repeat. Profile `reasoning_strength: medium` and the transport's thinking
disabled request are unchanged; internal reasoning is not inferred.
Input 2910–8333; does not fit: 0.
Causes: `{"generation-refused:error:chat.transport.timeout": 3, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 2, "ungraded-proposal": 7}`. Captures:
`{"complete-choice-json": 11, "missing": 3}`; reasoning metadata:
`{"non-empty": 11}`. Missing metadata is not zero.

Native admission/original grading controls still block effects, project tests,
feedback and retries. Task passes remain unavailable. No candidate, hidden or
held-out test ran. 16 attempts remain before tuning or new capabilities.
Earlier attempts are preserved and counted once; none is replayed. All original frozen input bindings
pass against the complete read-only Git-backed source snapshot; the licence
batch's working-tree changes are outside this measurement. All 30 contexts
were reproduced before inference against this model's own preceding repeat.

Completed-job reserved work 2971.395 seconds excludes queues, pre-entry
validation, wrapper overhead and released pauses. The sum of recorded controller
launch-to-exit UTC intervals is 7165.059 seconds including queues and
released pauses; the gap between sessions, outer authentication, preflight
and reporting are outside it.
The preceding segments total 5606.979 seconds and the resumed
segment is 1558.080 seconds. GPU kernel time and complete
end-to-end latency are unmeasured.
Observed response field combinations: `{"non-empty content / non-empty reasoning_content": 11, "unavailable content / unavailable reasoning_content": 3}`.
These describe captured fields; internal reasoning is not inferred.
Audit, comparison, original-helper reproduction and cleanup pass.
T-303/T-306 remain open. See [evidence](t303-inline-schema-muse-checkpoint14-2026-11-17.md).

The builder requested a checkpoint through the exact controller's stop flag. The active reserved pair finished normally; the next pair never started. No model attempt was cancelled by the checkpoint. Original request deadlines still apply. The stop method, process identity, request and exit are retained for a resume without replay.

## Inline-schema Muse development checkpoint20 — 2026-11-17 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-17 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | muse (20/30) | 32768 / 24395–29826 | Unavailable | Unavailable | 13/20 | Not run | 3299.220 | 63.313 | 111197 / 99327 (17/20 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
framing, thinking request, deadlines and output formula match the preceding
muse repeat. Profile `reasoning_strength: medium` and the transport's thinking
disabled request are unchanged; internal reasoning is not inferred.
Input 2910–8341; does not fit: 0.
Causes: `{"generation-refused:error:chat.transport.timeout": 3, "strict-edit-refused:code.proposal.edit": 2, "strict-edit-refused:code.proposal.format": 2, "ungraded-proposal": 13}`. Captures:
`{"complete-choice-json": 17, "missing": 3}`; reasoning metadata:
`{"non-empty": 17}`. Missing metadata is not zero.

Native admission/original grading controls still block effects, project tests,
feedback and retries. Task passes remain unavailable. No candidate, hidden or
held-out test ran. 10 attempts remain before tuning or new capabilities.
Earlier attempts are preserved and counted once; none is replayed. All original frozen input bindings
pass against the complete read-only Git-backed source snapshot; the licence
batch's working-tree changes are outside this measurement. All 30 contexts
were reproduced before inference against this model's own preceding repeat.

Completed-job reserved work 3798.786 seconds excludes queues, pre-entry
validation, wrapper overhead and released pauses. The sum of recorded controller
launch-to-exit UTC intervals is 9622.000 seconds including queues and
released pauses; the gap between sessions, outer authentication, preflight
and reporting are outside it.
The preceding segments total 7165.059 seconds and the resumed
segment is 2456.941 seconds. GPU kernel time and complete
end-to-end latency are unmeasured.
Observed response field combinations: `{"non-empty content / non-empty reasoning_content": 17, "unavailable content / unavailable reasoning_content": 3}`.
These describe captured fields; internal reasoning is not inferred.
Audit, comparison, original-helper reproduction and cleanup pass.
T-303/T-306 remain open. See [evidence](t303-inline-schema-muse-checkpoint20-2026-11-17.md).

The builder requested a checkpoint through the exact controller's stop flag. The active reserved pair finished normally; the next pair never started. No model attempt was cancelled by the checkpoint. Original request deadlines still apply. The stop method, process identity, request and exit are retained for a resume without replay.

## Inline-schema Muse development completion — 2026-11-17 UTC

| Date | Source commit | Model | Context / output allowance | First task passes | Within-budget task passes | Strict ungraded proposals | Held-out | Request/check s | GPU reserved work min | Input / output tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-17 | 72c002a42957340de0b6fa0eb4d48385db2c7e62 | muse (30/30) | 32768 / 24395–29826 | Unavailable | Unavailable | 19/30 | Not run | 4851.758 | 92.631 | 166355 / 150086 (26/30 counters) |

Changed variable: trusted named inline `json_schema` request with `strict: true`.
Contexts/messages, model/profile, runtime, helper, prompts, strict checker,
framing, thinking request, deadlines and output formula match the preceding
muse repeat. Profile `reasoning_strength: medium` and the transport's thinking
disabled request are unchanged; internal reasoning is not inferred.
Input 2910–8341; does not fit: 0.
Causes: `{"generation-refused:error:chat.transport.timeout": 4, "strict-edit-refused:code.proposal.edit": 3, "strict-edit-refused:code.proposal.format": 4, "ungraded-proposal": 19}`. Captures:
`{"complete-choice-json": 26, "missing": 4}`; reasoning metadata:
`{"non-empty": 26}`. Missing metadata is not zero.

Native admission/original grading controls still block effects, project tests,
feedback and retries. Task passes remain unavailable. No candidate, hidden or
held-out test ran. 0 attempts remain before tuning or new capabilities.
Earlier attempts are preserved and counted once; none is replayed. All original frozen input bindings
pass against the complete read-only Git-backed source snapshot; the licence
batch's working-tree changes are outside this measurement. All 30 contexts
were reproduced before inference against this model's own preceding repeat.

Completed-job reserved work 5557.855 seconds excludes queues, pre-entry
validation, wrapper overhead and released pauses. The sum of recorded controller
launch-to-exit UTC intervals is 14310.373 seconds including queues and
released pauses; the gap between sessions, outer authentication, preflight
and reporting are outside it.
The preceding segments total 9622.000 seconds and the resumed
segment is 4688.373 seconds. GPU kernel time and complete
end-to-end latency are unmeasured.
Observed response field combinations: `{"non-empty content / non-empty reasoning_content": 26, "unavailable content / unavailable reasoning_content": 4}`.
These describe captured fields; internal reasoning is not inferred.
Audit, comparison, original-helper reproduction and cleanup pass.
T-303/T-306 remain open. See [evidence](t303-inline-schema-muse-complete-2026-11-17.md).

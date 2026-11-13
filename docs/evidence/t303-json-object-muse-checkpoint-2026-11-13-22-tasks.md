# Muse JSON-object checkpoint — 2026-11-13 (22 of 30 tasks)

**22/30 frozen development tasks attempted; 8 remain.** At source
`40cad6472560e47314a55ddb1f45e2c4c22b1792`, Muse produced **14/22 strict ungraded
proposals**. First-attempt and within-budget task passes remain **unavailable**.
T-303/T-306 stay open. This cumulative prefix includes the earlier two-task
and 12-task checkpoints; checkpoint lines must not be added together. This is an incomplete
partial-path measurement, not a solved-task score.

## Measured path and settings

Rust source selection receives each statement and its editable frozen base file,
then uses credential-owning chat and the unchanged strict edit checker. Full
repository retrieval is not measured. Native executable artifact admission and
the missing original grading-control receipt still block execution, tests and
retries. No candidate code, test, feedback, retry, hidden or held-out test ran.
No original control is reconstructed and no admission boundary is bypassed.

The changed variable from the preceding complete-response measurement remains
the trusted JSON-object output request. All 22 captured upstream bodies include
`response_format: {"type":"json_object"}`. Complete framing, prompts,
selection, checker, profiles, byte ceilings and 300-second idle/total deadlines
stay fixed. Context: 32,768 tokens. Output:
`min(32768, 32768 - rendered input tokens - 32)`, ranging 24395–29826.
Input ranges 2910–8341. **Does not fit: 0/22 attempted**, with no claim about
the remaining 8. These are the largest allowances in the prepared 32K profile
and transport policy, not every physically possible context.

All 2,283 repository and 16 private input hashes were verified before each job.
The API SHA-256 is
`a8995c1515165d869e9382cab85cd1cd38c5902d3bd3dea8c641bdef50316dc2`;
the unchanged helper SHA-256 is
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
Muse SHA-256: `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`
(16,756,683,904 bytes). Profile: four threads,
temperature 0, seed 42, Q8 key/value caches, batch 256, microbatch 128,
`reasoning_strength: medium`. The driver remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, changed since preparation
and unqualified for coding. No download or role adoption occurs.
The [source review](../reviews/de8c204cbc52-json-object-model-output.txt) passes
exact source increment `de8c204cbc52c7b090f8039a4bfaf15666854852` only;
its low preflight finding stays T-309 backlog.

## Results and costs

Causes: 3 `generation-refused:error:chat.transport.timeout`, 2 `strict-edit-refused:code.proposal.edit`, 3 `strict-edit-refused:code.proposal.format`, 14 `ungraded-proposal`. Task passes with and without retries remain unavailable.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8120 | 24616 | 8203 | `strict-edit-refused:code.proposal.edit` | 191.646 |
| dev-002 | 8227 | 24509 | 5864 | `ungraded-proposal` | 135.279 |
| dev-003 | 4298 | 28438 | 5527 | `strict-edit-refused:code.proposal.edit` | 130.085 |
| dev-004 | 2931 | 29805 | Unavailable | `generation-refused:error:chat.transport.timeout` | 300.007 |
| dev-005 | 6622 | 26114 | 3792 | `ungraded-proposal` | 93.375 |
| dev-006 | 7898 | 24838 | 5183 | `ungraded-proposal` | 119.735 |
| dev-007 | 8324 | 24412 | 8073 | `ungraded-proposal` | 189.667 |
| dev-008 | 7987 | 24749 | 10539 | `strict-edit-refused:code.proposal.format` | 238.147 |
| dev-009 | 2910 | 29826 | 2274 | `ungraded-proposal` | 57.552 |
| dev-010 | 3911 | 28825 | 5122 | `ungraded-proposal` | 115.219 |
| dev-011 | 5943 | 26793 | Unavailable | `generation-refused:error:chat.transport.timeout` | 300.026 |
| dev-012 | 2941 | 29795 | Unavailable | `generation-refused:error:chat.transport.timeout` | 300.006 |
| dev-013 | 8333 | 24403 | 12578 | `strict-edit-refused:code.proposal.format` | 290.706 |
| dev-014 | 5914 | 26822 | 7975 | `ungraded-proposal` | 179.992 |
| dev-015 | 8341 | 24395 | 4413 | `ungraded-proposal` | 109.472 |
| dev-016 | 7888 | 24848 | 6489 | `ungraded-proposal` | 150.952 |
| dev-017 | 3987 | 28749 | 4125 | `ungraded-proposal` | 100.855 |
| dev-018 | 3008 | 29728 | 5387 | `ungraded-proposal` | 121.402 |
| dev-019 | 8121 | 24615 | 869 | `ungraded-proposal` | 30.481 |
| dev-020 | 7308 | 25428 | 2914 | `ungraded-proposal` | 70.411 |
| dev-021 | 7558 | 25178 | 8192 | `ungraded-proposal` | 192.997 |
| dev-022 | 6715 | 26021 | 8371 | `strict-edit-refused:code.proposal.format` | 189.262 |

Request/check time: **3607.273 seconds**.
Eleven two-task reservations total **3930.899 seconds /
65.515 GPU job minutes**, including verification,
load and cleanup. Each job uses both resource wrappers and a 900-second GPU cap.
Queue waits, wrapper overhead and 30-second pauses outside both reservations
are excluded; active GPU kernel time is not measured. Final backend counters
cover **19/22** attempts: **125,470 input /
115,890 output tokens**, including non-answer output.
Missing final counters are not zero tokens. API usage was not requested.
Backend terminations: 19 attempts stopped below token limits; 3 attempts had no unambiguous final backend termination.
Transport outcomes alone do not establish backend root cause. Dev-012's
request/check window includes about 11 seconds of backend generation for
cancelled dev-011 before its own slot launch, as clarified in the
[checkpoint review](../reviews/dd19baea2e52-json-object-muse-checkpoint12.txt).
Cancellation latency stays T-309 backlog.

## Queue checkpoint and continuation

The fourth controller completed pairs 12–20 after the third completed pairs
04–10, the second completed pair 02 and the first completed pair 00. Each task
started once. Earlier build-only checkpoints waited 733.337, 454.539 and
836.680 seconds; their receipts stay unchanged.
The next pair waited **911.025 seconds** for
a shared build reservation before this checkpoint. No GPU-wrapper entry, job
directory creation or task 23 attempt occurred. The owned process group was
paused and its exact command lines rechecked before termination. A fresh
post-cleanup scan records no remaining live group member. No other worker was
touched. Queue-only terminations are scheduler checkpoints, not model failures
or GPU execution time. Resume at tasks 23–24 using the prepared fifth
controller and fresh log, without replaying the completed prefix.

## Verification and retention

Source/record audit, original-helper context/check reproduction and cleanup pass.
All eleven settings were revoked, APIs reaped, model keys and sockets removed,
with no campaign model/API/controller process remaining. The separate
read/hash-only archive verifier passes **17,120 files** and
reconstructs all four controller histories, outcomes, requests, budgets,
counters, times and frozen bindings. It extracts nothing and executes no
archived code. Twenty-two retired API tokens and all weights are excluded;
raw answers and candidates stay private. All 120 earlier sealed artifact
bindings are preserved. Full-report helpers remain inactive; their presence
is not evidence that the remaining tasks ran.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-muse-checkpoint4-20261113-inputs.tar.gz` | `c1c1a24af21f8465642829c9fecbef49a15a096a42ffae4f3b646a170a7a8297` |
| `json-object-muse-checkpoint4-20261113-manifest.json` | `72b7e94b437d946b0707553d93318294d81ed37c22ecaa4478a661c95344ad97` |
| Frozen inputs | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

Reporting-only fast gate, direct Markdown lint and scope/privacy checks are
recorded with the core review request. The last affected-source gate retained
53 inherited failures and is not rerun for this report. No source, prompt,
profile, task, threshold, permission, supply-chain output or ledger row changes.
Remaining Muse tasks and A4B precede tuning except high/medium fixes.
No task-quality improvement, model qualification, row closure, release or
owner-test readiness is claimed. Independent core review is required.

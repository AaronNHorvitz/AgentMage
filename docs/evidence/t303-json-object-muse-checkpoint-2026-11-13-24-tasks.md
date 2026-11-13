# Muse JSON-object checkpoint — 2026-11-13 (24 of 30 tasks)

**24/30 frozen development tasks attempted; 6 remain.** Source:
`40cad6472560e47314a55ddb1f45e2c4c22b1792`. Muse produced **16/24 strict ungraded proposals**.
First-attempt and within-budget task passes remain **unavailable**. This report
adds the previously unreported tasks 23–24 to the public 22-task checkpoint;
no new attempt started during this reporting session. Earlier 2-, 12- and
22-task checkpoints are included; never add their totals. T-303/T-306 remain open.

## Measured path and settings

The available partial path is Rust source selection, credential-owning chat and
the unchanged strict checker. Selection receives each statement and editable
frozen base file; full repository retrieval is not measured. Native executable
artifact admission and the missing original grading-control receipt still block
execution, tests and retries. No candidate code, tests, feedback, retries, hidden
or held-out tests ran. No original control is reconstructed or boundary bypassed.
A strict proposal is not a solved task.

Changed variable from the preceding complete-response measurement: the trusted
JSON-object output request. All 24 upstream bodies contain
`response_format: {"type":"json_object"}`. Complete framing, prompts, selection,
checker, profiles, byte ceilings and 300-second idle/total deadlines stay fixed.
Context: 32,768 tokens. Output:
`min(32768, 32768 - rendered input tokens - 32)`, ranging 24395–29826.
Input ranges 2910–8341. **Does not fit: 0/24 attempted**; no claim is made about
remaining tasks. These are the largest allowances in the prepared 32K profile
and transport policy, not every physically possible context.

All 2,283 repository and 16 private input hashes were verified before each job.
API SHA-256:
`a8995c1515165d869e9382cab85cd1cd38c5902d3bd3dea8c641bdef50316dc2`.
Unchanged helper SHA-256:
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
Muse SHA-256: `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`
(16,756,683,904 bytes). Profile: four threads,
temperature 0, seed 42, Q8 key/value caches, batch 256, microbatch 128 and
`reasoning_strength: medium`. The driver remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, changed since preparation and
unqualified for coding. No download or model role adoption occurs.
The [source review](../reviews/de8c204cbc52-json-object-model-output.txt) passes
exact increment `de8c204cbc52c7b090f8039a4bfaf15666854852` only; its low
preflight finding remains T-309 backlog.

## Results and costs

Causes: 3 `generation-refused:error:chat.transport.timeout`, 2 `strict-edit-refused:code.proposal.edit`, 3 `strict-edit-refused:code.proposal.format`, 16 `ungraded-proposal`.

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
| dev-023 | 7619 | 25117 | 4066 | `ungraded-proposal` | 100.016 |
| dev-024 | 5103 | 27633 | 2797 | `ungraded-proposal` | 65.872 |

Request/check time: **3773.161 seconds**.
Twelve two-task reservations total **4113.321 seconds /
68.555 GPU job minutes**, including input
verification, model load and cleanup. Every job uses both resource wrappers and
a 900-second GPU cap. Queue waits, wrapper overhead and 30-second pauses outside
both reservations are excluded. Active GPU kernel time is unmeasured.
Final backend counters cover **21/24** attempts:
**138,192 input / 122,753 output tokens**,
including non-answer output. Missing counters are not zero tokens. API usage
was not requested. Transport outcomes alone do not establish backend root cause.
Dev-012's request/check window includes about 11 seconds of cancelled dev-011
backend generation before its own slot launch, as clarified in the
[checkpoint review](../reviews/dd19baea2e52-json-object-muse-checkpoint12.txt).
Cancellation latency remains T-309 backlog.

## Queue checkpoint and continuation

Controllers 1–5 completed pairs 00, 02, 04–10, 12–20 and 22 respectively.
Controllers 6–9 waited without starting another task. Every attempted task
started once. The retained build-only checkpoints are cumulative history:

| Controller | Completed tasks at checkpoint | Build wait s |
| --- | --- | --- |
| 1 | 2 | 733.337 |
| 2 | 4 | 454.539 |
| 3 | 12 | 836.680 |
| 4 | 22 | 911.025 |
| 5 | 24 | 569.440 |
| 6 | 24 | 666.229 |
| 7 | 24 | 386.389 |
| 8 | 24 | 584.815 |
| 9 | 24 | 509.793 |

The ninth controller's owned process group was paused and its exact controller,
build-wrapper and sleep arguments rechecked before termination. Unrecognized
processes, including any flock command, refuse checkpointing and resume the
group. The next job directory remains absent. No GPU-wrapper entry or task 25
attempt occurred. The fresh cleanup scan found no remaining campaign process.
No other worker was touched. Queue-only terminations are scheduler checkpoints,
not model failures or GPU time. Resume tasks 25–26 through a fresh controller
and logs, without replaying any completed task. A4B follows Muse before tuning,
except high/medium review fixes.

## Verification and retention

Source/record audit, original-helper context/check reproduction and cleanup
pass for all 24 tasks. All twelve settings were revoked, APIs reaped, model keys
and sockets removed, with no campaign model/API/controller process remaining.
The separate read/hash-only archive verifier passes **18,678 files**,
reconstructing all nine controller histories, outcomes, requests, budgets,
counters, times and frozen bindings. It extracts nothing and executes no archived
code. All 24 retired API token files and all weights are excluded; raw answers
and candidates remain private. All 145 earlier sealed artifact bindings remain
unchanged. Full-report helpers are prepared but inactive and cannot establish
completion after this checkpoint.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-muse-checkpoint9-20261113-inputs.tar.gz` | `eaf2ec60536c0ddeaa1c145aff02c57ddaa65eea63f894914cf960e00b35e4fe` |
| `json-object-muse-checkpoint9-20261113-manifest.json` | `34e304f83bac95f70e9e4af9f15f6b4ba73a1a2da9a3961c1ec91c383c858a60` |
| Frozen inputs | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

Reporting-only fast gate, direct Markdown lint and scope/privacy results are
retained for independent core review. The last affected-source gate retained 53
inherited failures and is not rerun for this report. No source, prompt, profile,
task, threshold, permission, supply-chain output or ledger row changes. No
qualification, row closure, release or owner-test readiness is claimed.

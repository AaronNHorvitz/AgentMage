# JSON-object Muse development repeat — 2026-11-13

All 30 frozen development tasks ran at source `40cad6472560e47314a55ddb1f45e2c4c22b1792`
with `muse`: **19/30 strict ungraded proposals**. First-attempt
and within-budget task passes remain **unavailable**. T-303/T-306 remain open.
No candidate code, test, feedback, retry or held-out task ran. This full repeat
includes the earlier 2-, 12-, 22- and 24-task checkpoints; their totals are never added.

## Scope and budgets

The measured partial path is the existing Rust source selection, credential-owning
chat API and unchanged strict edit checker. Selection receives the statement and
editable frozen base file; full repository retrieval is not measured. Native
executable artifact admission and the missing original grading-control receipt
still block edits, tests and retries through the native coding path. No boundary
is bypassed and no original control is reconstructed. A proposal is not a solved task.

The changed variable from the preceding complete-response repeat is the trusted
JSON-object output request. Each captured upstream body includes exactly
`response_format: {"type":"json_object"}`. Complete framing, prompts,
source selection, checker, profiles, byte ceilings and deadlines remain unchanged.
Context is 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)`, ranging 24395–29826.
Input ranges 2910–8341; **does not fit: 0**. These are the largest
allowances in the prepared 32K profile and transport policy, not a claim about
every physically possible context. Idle and total deadlines both remain 300 seconds.

The API SHA-256 is
`a8995c1515165d869e9382cab85cd1cd38c5902d3bd3dea8c641bdef50316dc2`;
the unchanged helper SHA-256 is
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
All 2,283 repository and 16 private input bindings are verified before each job.
The [independent source review](../reviews/de8c204cbc52-json-object-model-output.txt)
returns PASS for exact commit `de8c204cbc52c7b090f8039a4bfaf15666854852`;
its low preflight finding remains T-309 backlog. It does not approve this measurement.
The model SHA-256 is `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`
(16,756,683,904 bytes). Its profile retains four threads,
temperature 0, seed 42, Q8 key/value caches, batch 256, microbatch 128 and
`reasoning_strength: medium` as the retained template argument. The driver package remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, marked changed since
preparation and unqualified for coding. No download or model role adoption occurs.

## Results

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 4 |
| `strict-edit-refused:code.proposal.edit` | 3 |
| `strict-edit-refused:code.proposal.format` | 4 |
| `ungraded-proposal` | 19 |
| Does not fit | 0 |
| First-attempt task passes | Unavailable |
| Within-budget task passes | Unavailable |

Request/check time totals **4,716.747 seconds**.
Fifteen separate two-task reservations total **5,131.159
seconds / 85.519 GPU job minutes**, including
verification, load and cleanup, excluding queues and wrapper overhead. This is
reserved job wall, not active GPU kernel time. The scheduler pauses 30 seconds
outside both reservations between jobs; each job uses both resource wrappers
and the GPU wrapper caps execution at 900 seconds. API usage is not requested. Final backend counters cover
**27/30** attempts: **174,326 input /
163,130 output tokens**, including non-answer output.
Counters begin at each task's own slot launch. Token totals exclude attempts
without final counters; missing counters are not zero tokens. Backend terminations: 27 attempts stopped below the token limits; 3 attempts had no unambiguous final backend termination; transport outcomes alone do not establish root cause. Dev-012's request/check window includes about
11 seconds of backend generation for cancelled dev-011 before its own slot
launch, as clarified in the [checkpoint review](../reviews/dd19baea2e52-json-object-muse-checkpoint12.txt).
Cancellation latency remains T-309 backlog.

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
| dev-025 | 4109 | 28627 | 7631 | `strict-edit-refused:code.proposal.edit` | 178.141 |
| dev-026 | 4704 | 28032 | 1967 | `ungraded-proposal` | 47.572 |
| dev-027 | 7971 | 24765 | 13044 | `generation-refused:error:chat.transport.timeout` | 300.013 |
| dev-028 | 8131 | 24605 | 7426 | `strict-edit-refused:code.proposal.format` | 170.377 |
| dev-029 | 7852 | 24884 | 3281 | `ungraded-proposal` | 84.583 |
| dev-rust-001 | 3367 | 29369 | 7028 | `ungraded-proposal` | 162.900 |

## Verification and retention

The source/record audit verifies frozen inputs, 30 unique development tasks,
request bodies, strict checks, candidate/base hashes, resource guards, cleanup,
token counters and costs. The original helper reproduces all 30 contexts and
available strict checks without executing candidates or hidden tests. Cleanup
passes for all 15 pairs: settings revoked, API processes reaped, model keys and
sockets absent, no remaining campaign model/API/controller processes.

The separate read/hash-only archive verifier passes **22,362
files**, including the exact source snapshot, binaries, helpers and raw records.
It extracts nothing and imports or executes no archived code. All 30 retired
API token files are excluded; no weights are packaged. Answers and candidates
remain private. All 145 artifact bindings in the earlier evidence seals and all 25 artifacts
bound by the 24-task checkpoint stay unchanged. The first controller completed pair 00; the second controller completed pair 02; the third completed
pairs 04–10; the fourth completed pairs 12–20; the fifth completed pair 22; the sixth waited without starting a task;
the seventh waited without starting a task;
the eighth waited without starting a task;
the ninth waited without starting a task;
the tenth completed pairs 24–28 without replay. Each task started once.
Completed jobs released both reservations and paused between pairs. The first checkpoint
waited 733.337 seconds for a shared build slot before GPU entry or a new
attempt. The second checkpoint waited 454.539 seconds for a shared build slot
before GPU entry or a new attempt. The third checkpoint waited 836.680 seconds for a shared build slot before GPU
entry or a new attempt. The fourth checkpoint waited 911.025 seconds for a shared
build slot before GPU entry or a new attempt. The fifth checkpoint waited 569.440 seconds for a shared build slot before GPU
entry or a new attempt. The sixth checkpoint waited 666.229 seconds for a shared
build slot before GPU entry or a new attempt. The seventh checkpoint waited 386.389 seconds for a shared build slot before GPU
entry or a new attempt. The eighth checkpoint waited 584.815 seconds for a shared build slot before GPU
entry or a new attempt. The ninth checkpoint waited 509.793 seconds for a shared build slot before GPU
entry or a new attempt. All nine owned queue-only terminations are scheduler
checkpoints, not model failures. Queue waits are excluded
from GPU job time and model failures.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-muse-20261113-inputs.tar.gz` | `3fca14c9cb071bcb2816901368e7c828f6f7f690658fb780d6c996e82501046a` |
| `json-object-muse-20261113-manifest.json` | `2bba54b1afee2a0fae3ec007101d908a0a5a466e5d79f6dbb86031c844349a1b` |
| Frozen input manifest | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

This reporting increment changes no source, prompt, profile, task, threshold,
permission or supply-chain output. Reporting-only fast gate and direct Markdown
lint results are recorded with the review request. The last affected-source gate
recorded 53 inherited failures; it is not rerun for this report. The final
prepared profile must finish its development repeat before further
tuning, except high/medium review fixes. No task-quality improvement, native
qualification, row closure, release or owner-test readiness is claimed.
Independent core review of this measurement remains required.

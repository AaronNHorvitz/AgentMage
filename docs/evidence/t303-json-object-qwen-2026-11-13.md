# JSON-object 27B development repeat — 2026-11-13

All 30 frozen development tasks ran at source `40cad6472560e47314a55ddb1f45e2c4c22b1792`
with `qwen35-27b`: **29/30 strict ungraded proposals**. First-attempt
and within-budget task passes remain **unavailable**. T-303/T-306 remain open.
No candidate code, test, feedback, retry or held-out task ran.

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
`min(32768, 32768 - rendered input tokens - 32)`, ranging 24137–29804.
Input ranges 2932–8599; **does not fit: 0**. These are the largest
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
The model SHA-256 is `5ae89d6720ba0a27f643e6fbb0d1b557870d17aeff23156e5981e863ec16985c`
(16,547,400,032 bytes). Its profile retains four threads,
temperature 0, seed 42, Q8 key/value caches, batch 256, microbatch 128 and
`enable_thinking: false` as the retained template argument. The driver package remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, marked changed since
preparation and unqualified for coding. No download or model role adoption occurs.

## Results

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 1 |
| `ungraded-proposal` | 29 |
| Does not fit | 0 |
| First-attempt task passes | Unavailable |
| Within-budget task passes | Unavailable |

Request/check time totals **1,434.814 seconds**.
Fifteen separate two-task reservations total **1,846.660
seconds / 30.778 GPU work minutes**, including
verification, load and cleanup, excluding queues and wrapper overhead. This is
reserved job wall, not active GPU kernel time. The scheduler pauses 30 seconds
outside both reservations between jobs; each job uses both resource wrappers
and the GPU wrapper caps execution at 900 seconds. API usage is not requested. Final backend counters cover
**29/30** attempts: **186,402 input /
34,920 output tokens**, including non-answer output.
Counters begin at each task's own slot launch. Token totals exclude attempts
without final counters.
dev-004 reaches the existing total deadline. Its backend slice retains 98 progress counters, reaching an observed lower bound of 13,012 output tokens at 298.319 seconds after its own slot launch. Its final eval summary and stop line are absent, so its final input/output totals remain unavailable. The lower bound is not added to the final-counter totals; missing final counters are not zero tokens. The cause of the long output is not established. Deadline changes remain T-309 backlog.

Backend terminations are
`{'stopped-below-token-limits': 29, 'unobserved-or-ambiguous': 1}`; transport outcomes alone do not establish root cause.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8198 | 24538 | 1014 | `ungraded-proposal` | 42.243 |
| dev-002 | 8303 | 24433 | 1193 | `ungraded-proposal` | 34.243 |
| dev-003 | 4290 | 28446 | 723 | `ungraded-proposal` | 29.386 |
| dev-004 | 2932 | 29804 | Unavailable | `generation-refused:error:chat.transport.timeout` | 300.010 |
| dev-005 | 7154 | 25582 | 306 | `ungraded-proposal` | 22.938 |
| dev-006 | 8024 | 24712 | 958 | `ungraded-proposal` | 29.002 |
| dev-007 | 8449 | 24287 | 2777 | `ungraded-proposal` | 82.561 |
| dev-008 | 8061 | 24675 | 3104 | `ungraded-proposal` | 77.894 |
| dev-009 | 2939 | 29797 | 169 | `ungraded-proposal` | 15.598 |
| dev-010 | 3913 | 28823 | 1133 | `ungraded-proposal` | 29.874 |
| dev-011 | 6062 | 26674 | 4157 | `ungraded-proposal` | 108.492 |
| dev-012 | 2938 | 29798 | 1218 | `ungraded-proposal` | 30.340 |
| dev-013 | 8428 | 24308 | 1914 | `ungraded-proposal` | 62.412 |
| dev-014 | 6075 | 26661 | 2074 | `ungraded-proposal` | 52.292 |
| dev-015 | 8599 | 24137 | 1110 | `ungraded-proposal` | 41.854 |
| dev-016 | 8150 | 24586 | 1766 | `ungraded-proposal` | 47.259 |
| dev-017 | 4060 | 28676 | 533 | `ungraded-proposal` | 24.613 |
| dev-018 | 3069 | 29667 | 814 | `ungraded-proposal` | 21.595 |
| dev-019 | 8273 | 24463 | 168 | `ungraded-proposal` | 19.978 |
| dev-020 | 7461 | 25275 | 315 | `ungraded-proposal` | 14.107 |
| dev-021 | 7724 | 25012 | 1022 | `ungraded-proposal` | 38.812 |
| dev-022 | 6841 | 25895 | 1127 | `ungraded-proposal` | 31.676 |
| dev-023 | 7630 | 25106 | 834 | `ungraded-proposal` | 39.945 |
| dev-024 | 5138 | 27598 | 730 | `ungraded-proposal` | 22.293 |
| dev-025 | 4153 | 28583 | 2235 | `ungraded-proposal` | 73.055 |
| dev-026 | 4821 | 27915 | 132 | `ungraded-proposal` | 8.592 |
| dev-027 | 8036 | 24700 | 966 | `ungraded-proposal` | 49.243 |
| dev-028 | 8176 | 24560 | 690 | `ungraded-proposal` | 24.518 |
| dev-029 | 8023 | 24713 | 692 | `ungraded-proposal` | 32.602 |
| dev-rust-001 | 3414 | 29322 | 1046 | `ungraded-proposal` | 27.390 |

## Verification and retention

The source/record audit verifies frozen inputs, 30 unique development tasks,
request bodies, strict checks, candidate/base hashes, resource guards, cleanup,
token counters and costs. The original helper reproduces all 30 contexts and
available strict checks without executing candidates or hidden tests. Cleanup
passes for all 15 pairs: settings revoked, API processes reaped, model keys and
sockets absent, no remaining campaign model/API/controller processes.

The separate read/hash-only archive verifier passes **22,007
files**, including the exact source snapshot, binaries, helpers and raw records.
It extracts nothing and imports or executes no archived code. All 30 retired
API token files are excluded; no weights are packaged. Answers and candidates
remain private. The earlier E4B and 31B records and all previous seals stay intact. No completed
attempt was replayed; this profile starts each task once. Queue-only waits
before pairs 08 and 14 were checkpointed without a job directory or GPU entry;
fresh controller logs preserve each wait and resume completed pairs by their
finished reports. Queue cancellation is not a model failure or GPU work.
A later checkpoint inspection before tasks 23–24 briefly paused the owned
process group. The reservation had advanced to the GPU wrapper; the guard
resumed the group without terminating it. That pair then finished normally;
no attempt was restarted. The private inspection record is retained in the
sealed archive.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-qwen-20261113-inputs.tar.gz` | `d31ae0babe8f1cbf2f01184616a6fd568b69de87453383446680550a802aa71a` |
| `json-object-qwen-20261113-manifest.json` | `fea48d017e5212bf828170904467a1a27f0128fb56b9e71602eda0feb80e523a` |
| Frozen input manifest | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

This reporting increment changes no source, prompt, profile, task, threshold,
permission or supply-chain output. Reporting-only fast gate and direct Markdown
lint results are recorded with the review request. The last affected-source gate
recorded 53 inherited failures; it is not rerun for this report. The other
three prepared profiles must finish their development repeats before further
tuning, except high/medium review fixes. No task-quality improvement, native
qualification, row closure, release or owner-test readiness is claimed.
Independent core review of this measurement remains required.

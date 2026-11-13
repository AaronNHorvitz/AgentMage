# JSON-object A4B development repeat — 2026-11-13

All 30 frozen development tasks ran at source `40cad6472560e47314a55ddb1f45e2c4c22b1792`
with `gemma4-26b-a4b`: **3/30 strict ungraded proposals**. First-attempt
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
`min(32768, 32768 - rendered input tokens - 32)`, ranging 23004–29513.
Input ranges 3223–9732; **does not fit: 0**. These are the largest
allowances in the prepared 32K profile and transport policy, not a claim about
every physically possible context. Idle and total deadlines both remain 300 seconds.

The API SHA-256 is
`a8995c1515165d869e9382cab85cd1cd38c5902d3bd3dea8c641bdef50316dc2`;
the unchanged helper SHA-256 is
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
All 2,283 repository and 16 private input bindings are verified before each job.
The latest resume preflight also verifies 257 earlier retained artifacts, including
both preceding A4B checkpoints. Those checkpoints are included, never added.
The [independent source review](../reviews/de8c204cbc52-json-object-model-output.txt)
returns PASS for exact commit `de8c204cbc52c7b090f8039a4bfaf15666854852`;
its low preflight finding remains T-309 backlog. It does not approve this measurement.
The model SHA-256 is `a07f72221e8e3f77455ab0d7f7652d01a9f63c262b954aa6932a53275a0e895a`
(17,035,039,872 bytes). Its profile retains four threads,
temperature 0, seed 42, Q8 key/value caches, batch 256, microbatch 128 and
an empty retained template-argument object. The driver package remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, marked changed since
preparation and unqualified for coding. No download or model role adoption occurs.

## Results

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 26 |
| `strict-edit-refused:code.proposal.format` | 1 |
| `ungraded-proposal` | 3 |
| Does not fit | 0 |
| First-attempt task passes | Unavailable |
| Within-budget task passes | Unavailable |

Request/check time totals **6,052.758 seconds**.
Fifteen separate two-task reservations total **6,527.872
seconds / 108.798 GPU work minutes**, including
verification, load and cleanup, excluding queues and wrapper overhead. This is
reserved job wall, not active GPU kernel time. The scheduler pauses 30 seconds
outside both reservations between jobs; each job uses both resource wrappers
and the GPU wrapper caps execution at 900 seconds. API usage is not requested. Final backend counters cover
**30/30** attempts: **215,893 input /
743,394 output tokens**, including non-answer output.
Counters begin at each task's own slot launch. Token totals exclude attempts
without final counters; missing counters are not zero tokens. Backend terminations:
26 attempts reached the output allowance; 4 attempts stopped below the token limits; transport outcomes alone do not establish root cause.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 9502 | 23234 | 23234 | `generation-refused:error:chat.response.incomplete` | 200.139 |
| dev-002 | 9564 | 23172 | 23172 | `generation-refused:error:chat.response.incomplete` | 173.211 |
| dev-003 | 4752 | 27984 | 27984 | `generation-refused:error:chat.response.incomplete` | 232.599 |
| dev-004 | 3223 | 29513 | 29513 | `generation-refused:error:chat.response.incomplete` | 216.040 |
| dev-005 | 8460 | 24276 | 24276 | `generation-refused:error:chat.response.incomplete` | 205.712 |
| dev-006 | 9310 | 23426 | 23426 | `generation-refused:error:chat.response.incomplete` | 175.147 |
| dev-007 | 9732 | 23004 | 23004 | `generation-refused:error:chat.response.incomplete` | 207.701 |
| dev-008 | 9372 | 23364 | 23364 | `generation-refused:error:chat.response.incomplete` | 181.628 |
| dev-009 | 3411 | 29325 | 29325 | `generation-refused:error:chat.response.incomplete` | 248.776 |
| dev-010 | 4315 | 28421 | 26183 | `ungraded-proposal` | 198.167 |
| dev-011 | 6684 | 26052 | 26052 | `generation-refused:error:chat.response.incomplete` | 228.404 |
| dev-012 | 3245 | 29491 | 29491 | `generation-refused:error:chat.response.incomplete` | 218.983 |
| dev-013 | 9732 | 23004 | 23004 | `generation-refused:error:chat.response.incomplete` | 208.151 |
| dev-014 | 6995 | 25741 | 25283 | `strict-edit-refused:code.proposal.format` | 194.836 |
| dev-015 | 9524 | 23212 | 23212 | `generation-refused:error:chat.response.incomplete` | 205.422 |
| dev-016 | 9072 | 23664 | 22559 | `ungraded-proposal` | 174.841 |
| dev-017 | 4548 | 28188 | 28188 | `generation-refused:error:chat.response.incomplete` | 236.341 |
| dev-018 | 3519 | 29217 | 29217 | `generation-refused:error:chat.response.incomplete` | 216.717 |
| dev-019 | 9350 | 23386 | 23386 | `generation-refused:error:chat.response.incomplete` | 206.204 |
| dev-020 | 8464 | 24272 | 24272 | `generation-refused:error:chat.response.incomplete` | 187.121 |
| dev-021 | 8741 | 23995 | 23995 | `generation-refused:error:chat.response.incomplete` | 211.528 |
| dev-022 | 7754 | 24982 | 24982 | `generation-refused:error:chat.response.incomplete` | 189.694 |
| dev-023 | 8866 | 23870 | 23870 | `generation-refused:error:chat.response.incomplete` | 207.575 |
| dev-024 | 5944 | 26792 | 26792 | `generation-refused:error:chat.response.incomplete` | 200.041 |
| dev-025 | 4811 | 27925 | 27925 | `generation-refused:error:chat.response.incomplete` | 236.330 |
| dev-026 | 5395 | 27341 | 8349 | `ungraded-proposal` | 66.640 |
| dev-027 | 9374 | 23362 | 23362 | `generation-refused:error:chat.response.incomplete` | 204.780 |
| dev-028 | 9465 | 23271 | 23271 | `generation-refused:error:chat.response.incomplete` | 176.287 |
| dev-029 | 9074 | 23662 | 23662 | `generation-refused:error:chat.response.incomplete` | 216.797 |
| dev-rust-001 | 3695 | 29041 | 29041 | `generation-refused:error:chat.response.incomplete` | 226.946 |

## Completed six-profile partial-path measurement

All six prepared profiles have now attempted the same 30 development tasks,
180 attempts in total, at the same source and JSON-object settings. The earlier
five reports and this report are counted once each; checkpoints are not added.

| Outcome / cause | Attempts |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 36 |
| `generation-refused:error:chat.transport.timeout` | 5 |
| `strict-edit-refused:code.proposal.edit` | 14 |
| `strict-edit-refused:code.proposal.format` | 12 |
| `ungraded-proposal` | 113 |

The largest observed refusal cause is `generation-refused:error:chat.response.incomplete` (36). After review findings,
that cause selects the next development work under MEASURE FIRST. This ranks
refusals on the measured partial path. Native execution and the original grading
controls remain unavailable; strict proposals remain ungraded, task pass rates
remain unavailable, and no model role or quality improvement is established.

## Verification and retention

The source/record audit verifies frozen inputs, 30 unique development tasks,
request bodies, strict checks, candidate/base hashes, resource guards, cleanup,
token counters and costs. The original helper reproduces all 30 contexts and
available strict checks without executing candidates or hidden tests. Cleanup
passes for all 15 pairs: settings revoked, API processes reaped, model keys and
sockets absent, no remaining campaign model/API/controller processes.

The separate read/hash-only archive verifier passes **22,422
files**, including the exact source snapshot, binaries, helpers and raw records.
It extracts nothing and imports or executes no archived code. All 30 retired
API token files are excluded; no weights are packaged. Answers and candidates
remain private. All earlier model records and previous evidence seals stay intact. No completed
attempt was replayed; this profile starts each task once. The first controller
finished tasks 1–2 before its graceful checkpoint. The second controller finished tasks 3–6 before its checkpoint. The third
controller skips all three completed pairs and finishes tasks 7–30, with fresh
reservation logs. All three controller histories are retained and verified.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-a4b-20261113-inputs.tar.gz` | `ec136475089ebfaafc79377bed271d9c60fd37644f3775f657852bd9e4685126` |
| `json-object-a4b-20261113-manifest.json` | `8cb20492b0216994b775fac7cdd0e526e26650a6db2ed59d43b4e3fc615a6198` |
| Frozen input manifest | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

This reporting increment changes no source, prompt, profile, task, threshold,
permission or supply-chain output. Reporting-only fast gate and direct Markdown
lint results are recorded with the review request. The last affected-source gate
recorded 53 inherited failures; it is not rerun for this report. All six prepared profiles have now completed this partial-path repeat.
The largest observed refusal cause selects the next work after review findings.
Task grades, native execution, tests and retries remain blocked. No task-quality improvement, native
qualification, row closure, release or owner-test readiness is claimed.
Independent core review of this measurement remains required.

# JSON-object E4B development repeat — 2026-11-13

All 30 frozen development tasks ran at source `40cad6472560e47314a55ddb1f45e2c4c22b1792`
with `gemma4-e4b`: **24/30 strict ungraded proposals**. First-attempt
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
The [independent source review](../reviews/de8c204cbc52-json-object-model-output.txt)
returns PASS for exact commit `de8c204cbc52c7b090f8039a4bfaf15666854852`;
its low preflight finding remains T-309 backlog. It does not approve this measurement.
The model SHA-256 is `d35a3aa7a6d47de237fd488bb6f7f3efd62908d828f85a63da0ee93acdbefe9f`
(5,405,170,144 bytes). Its profile retains four threads,
temperature 0, seed 42, Q8 key/value caches, batch 256, microbatch 128 and no
extra template arguments. The driver package remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, marked changed since
preparation and unqualified for coding. No download or model role adoption occurs.

## Results

| Outcome / cause | Tasks |
| --- | --- |
| `strict-edit-refused:code.proposal.edit` | 4 |
| `strict-edit-refused:code.proposal.format` | 2 |
| `ungraded-proposal` | 24 |
| Does not fit | 0 |
| First-attempt task passes | Unavailable |
| Within-budget task passes | Unavailable |

Request/check time totals **1,363.743 seconds**.
Fifteen separate two-task reservations total **1,484.900
seconds / 24.748 GPU work minutes**, including
verification, load and cleanup, excluding queues and wrapper overhead. This is
reserved job wall, not active GPU kernel time. The scheduler pauses 30 seconds
outside both reservations between jobs; each job uses both resource wrappers
with a 900-second cap. API usage is not requested. Final backend counters cover
**30/30** attempts: **215,893 input /
138,991 output tokens**, including non-answer output.
Counters begin at each task's own slot launch. Backend terminations are
`{'stopped-below-token-limits': 30}`; transport outcomes alone do not establish root cause.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 9502 | 23234 | 4224 | `ungraded-proposal` | 51.658 |
| dev-002 | 9564 | 23172 | 3252 | `ungraded-proposal` | 25.829 |
| dev-003 | 4752 | 27984 | 4093 | `strict-edit-refused:code.proposal.edit` | 48.303 |
| dev-004 | 3223 | 29513 | 6878 | `strict-edit-refused:code.proposal.format` | 49.106 |
| dev-005 | 8460 | 24276 | 2678 | `ungraded-proposal` | 39.428 |
| dev-006 | 9310 | 23426 | 3485 | `ungraded-proposal` | 27.608 |
| dev-007 | 9732 | 23004 | 3393 | `ungraded-proposal` | 45.126 |
| dev-008 | 9372 | 23364 | 5163 | `ungraded-proposal` | 40.024 |
| dev-009 | 3411 | 29325 | 2428 | `ungraded-proposal` | 36.793 |
| dev-010 | 4315 | 28421 | 6441 | `ungraded-proposal` | 49.390 |
| dev-011 | 6684 | 26052 | 7908 | `strict-edit-refused:code.proposal.format` | 84.909 |
| dev-012 | 3245 | 29491 | 5439 | `strict-edit-refused:code.proposal.edit` | 42.798 |
| dev-013 | 9732 | 23004 | 6876 | `ungraded-proposal` | 76.368 |
| dev-014 | 6995 | 25741 | 7556 | `ungraded-proposal` | 60.591 |
| dev-015 | 9524 | 23212 | 6418 | `ungraded-proposal` | 67.277 |
| dev-016 | 9072 | 23664 | 5861 | `ungraded-proposal` | 48.750 |
| dev-017 | 4548 | 28188 | 3043 | `strict-edit-refused:code.proposal.edit` | 40.250 |
| dev-018 | 3519 | 29217 | 4993 | `ungraded-proposal` | 37.001 |
| dev-019 | 9350 | 23386 | 637 | `ungraded-proposal` | 27.045 |
| dev-020 | 8464 | 24272 | 3469 | `ungraded-proposal` | 28.697 |
| dev-021 | 8741 | 23995 | 4799 | `ungraded-proposal` | 55.454 |
| dev-022 | 7754 | 24982 | 4733 | `ungraded-proposal` | 36.376 |
| dev-023 | 8866 | 23870 | 5685 | `ungraded-proposal` | 61.595 |
| dev-024 | 5944 | 26792 | 3860 | `ungraded-proposal` | 29.614 |
| dev-025 | 4811 | 27925 | 6902 | `ungraded-proposal` | 67.892 |
| dev-026 | 5395 | 27341 | 3239 | `ungraded-proposal` | 25.156 |
| dev-027 | 9374 | 23362 | 4429 | `ungraded-proposal` | 52.355 |
| dev-028 | 9465 | 23271 | 2321 | `strict-edit-refused:code.proposal.edit` | 19.284 |
| dev-029 | 9074 | 23662 | 4876 | `ungraded-proposal` | 60.006 |
| dev-rust-001 | 3695 | 29041 | 3912 | `ungraded-proposal` | 29.063 |

## Verification and retention

The source/record audit verifies frozen inputs, 30 unique development tasks,
request bodies, strict checks, candidate/base hashes, resource guards, cleanup,
token counters and costs. The original helper reproduces all 30 contexts and
available strict checks without executing candidates or hidden tests. Cleanup
passes for all 15 pairs: settings revoked, API processes reaped, model keys and
sockets absent, no remaining campaign model/API/controller processes.

The separate read/hash-only archive verifier passes **21,935
files**, including the exact source snapshot, binaries, helpers and raw records.
It extracts nothing and imports or executes no archived code. All 30 retired
API token files are excluded; no weights are packaged. Answers and candidates
remain private. The previous queue checkpoint and all earlier seals stay intact.
The first queued pair had no attempts; this continuation launches it once.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-e4b-20261113-inputs.tar.gz` | `5b68853c3fbab11290bb055790e7734700ca7a48f42bb4d5f88b0400b14c5703` |
| `json-object-e4b-20261113-manifest.json` | `cdbeae9045a1e0896aab5f4b36aa525681cd3e0df40f97fd9cf16fbe185d8179` |
| Frozen input manifest | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

This reporting increment changes no source, prompt, profile, task, threshold,
permission or supply-chain output. Reporting-only fast gate and direct Markdown
lint results are recorded with the review request. The affected-source gate
still has 53 inherited failures; it is not rerun for this report. The other
five prepared profiles must finish their development repeats before further
tuning, except high/medium review fixes. No task-quality improvement, native
qualification, row closure, release or owner-test readiness is claimed.
Independent core review of this measurement remains required.

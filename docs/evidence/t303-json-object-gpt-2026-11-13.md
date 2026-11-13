# JSON-object GPT-OSS development repeat — 2026-11-13

All 30 frozen development tasks ran at source `40cad6472560e47314a55ddb1f45e2c4c22b1792`
with `gpt-oss`: **12/30 strict ungraded proposals**. First-attempt
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
`min(32768, 32768 - rendered input tokens - 32)`, ranging 24219–29748.
Input ranges 2988–8517; **does not fit: 0**. These are the largest
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
The model SHA-256 is `27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901`
(12,109,566,624 bytes). Its profile retains four threads,
temperature 0, seed 42, Q8 key/value caches, batch 256, microbatch 128 and
`reasoning_effort: medium` as the retained template argument. The driver package remains
`xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`, marked changed since
preparation and unqualified for coding. No download or model role adoption occurs.

## Results

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 10 |
| `strict-edit-refused:code.proposal.edit` | 4 |
| `strict-edit-refused:code.proposal.format` | 4 |
| `ungraded-proposal` | 12 |
| Does not fit | 0 |
| First-attempt task passes | Unavailable |
| Within-budget task passes | Unavailable |

Request/check time totals **2,396.339 seconds**.
Fifteen separate two-task reservations total **2,571.144
seconds / 42.852 GPU job minutes**, including
verification, load and cleanup, excluding queues and wrapper overhead. This is
reserved job wall, not active GPU kernel time. The scheduler pauses 30 seconds
outside both reservations between jobs; each job uses both resource wrappers
and the GPU wrapper caps execution at 900 seconds. API usage is not requested. Final backend counters cover
**30/30** attempts: **188,409 input /
475,158 output tokens**, including non-answer output.
Counters begin at each task's own slot launch. Token totals exclude attempts
without final counters; missing counters are not zero tokens. Backend terminations are
`{'output-cap': 10, 'stopped-below-token-limits': 20}`; transport outcomes alone do not establish root cause.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8176 | 24560 | 6640 | `strict-edit-refused:code.proposal.edit` | 38.246 |
| dev-002 | 8239 | 24497 | 24497 | `generation-refused:error:chat.response.incomplete` | 122.840 |
| dev-003 | 4349 | 28387 | 28387 | `generation-refused:error:chat.response.incomplete` | 146.869 |
| dev-004 | 2988 | 29748 | 29748 | `generation-refused:error:chat.response.incomplete` | 144.963 |
| dev-005 | 6766 | 25970 | 4795 | `ungraded-proposal` | 28.182 |
| dev-006 | 7916 | 24820 | 7165 | `ungraded-proposal` | 35.877 |
| dev-007 | 8357 | 24379 | 17033 | `strict-edit-refused:code.proposal.format` | 87.866 |
| dev-008 | 8005 | 24731 | 24674 | `strict-edit-refused:code.proposal.format` | 122.270 |
| dev-009 | 3006 | 29730 | 2254 | `ungraded-proposal` | 15.252 |
| dev-010 | 3983 | 28753 | 14146 | `strict-edit-refused:code.proposal.edit` | 65.905 |
| dev-011 | 6033 | 26703 | 26703 | `generation-refused:error:chat.response.incomplete` | 133.627 |
| dev-012 | 3002 | 29734 | 18414 | `strict-edit-refused:code.proposal.edit` | 85.399 |
| dev-013 | 8380 | 24356 | 6457 | `strict-edit-refused:code.proposal.format` | 36.675 |
| dev-014 | 6009 | 26727 | 26727 | `generation-refused:error:chat.response.incomplete` | 132.178 |
| dev-015 | 8517 | 24219 | 24219 | `generation-refused:error:chat.response.incomplete` | 123.284 |
| dev-016 | 8061 | 24675 | 7959 | `ungraded-proposal` | 39.342 |
| dev-017 | 4049 | 28687 | 3755 | `ungraded-proposal` | 22.175 |
| dev-018 | 3064 | 29672 | 28638 | `ungraded-proposal` | 135.383 |
| dev-019 | 8241 | 24495 | 713 | `ungraded-proposal` | 10.336 |
| dev-020 | 7424 | 25312 | 2042 | `ungraded-proposal` | 11.607 |
| dev-021 | 7672 | 25064 | 22754 | `ungraded-proposal` | 116.047 |
| dev-022 | 6827 | 25909 | 16219 | `strict-edit-refused:code.proposal.format` | 78.463 |
| dev-023 | 7668 | 25068 | 25068 | `generation-refused:error:chat.response.incomplete` | 126.954 |
| dev-024 | 5159 | 27577 | 3807 | `ungraded-proposal` | 18.704 |
| dev-025 | 4168 | 28568 | 28568 | `generation-refused:error:chat.response.incomplete` | 142.644 |
| dev-026 | 4820 | 27916 | 4707 | `ungraded-proposal` | 22.811 |
| dev-027 | 7967 | 24769 | 24769 | `generation-refused:error:chat.response.incomplete` | 126.353 |
| dev-028 | 8157 | 24579 | 11180 | `strict-edit-refused:code.proposal.edit` | 54.968 |
| dev-029 | 7969 | 24767 | 3821 | `ungraded-proposal` | 24.183 |
| dev-rust-001 | 3437 | 29299 | 29299 | `generation-refused:error:chat.response.incomplete` | 146.938 |

## Verification and retention

The source/record audit verifies frozen inputs, 30 unique development tasks,
request bodies, strict checks, candidate/base hashes, resource guards, cleanup,
token counters and costs. The original helper reproduces all 30 contexts and
available strict checks without executing candidates or hidden tests. Cleanup
passes for all 15 pairs: settings revoked, API processes reaped, model keys and
sockets absent, no remaining campaign model/API/controller processes.

The separate read/hash-only archive verifier passes **22,065
files**, including the exact source snapshot, binaries, helpers and raw records.
It extracts nothing and imports or executes no archived code. All 30 retired
API token files are excluded; no weights are packaged. Answers and candidates
remain private. The earlier E4B, 31B and 27B records and all previous seals stay intact,
including the [four-task checkpoint](t303-json-object-gpt-checkpoint-2026-11-13.md).
No completed attempt was replayed; this profile starts each task once. The first
queued controller was checkpointed before GPU entry, with zero attempts. The
second controller completed pairs 00 and 02 (four tasks), then waited 404.835
seconds for the build reservation before pair 04 and was checkpointed without
GPU entry or a new attempt. The third controller preserved both completed pairs, then waited 618.869
seconds for the build reservation before pair 04 and was checkpointed without
GPU entry or a new attempt. The fourth controller preserved both pairs and
resumed pair 04 with a fresh reservation log. Queue waits and cancellations
are excluded from GPU job time and model failures. The earlier checkpoint stays
in the scoreboard as partial history; this entry reports the completed set.

| Retained binding | SHA-256 |
| --- | --- |
| `json-object-gpt-20261113-inputs.tar.gz` | `30932ccd9813002756e3f915b22dd5faa0e3eed112da76cd740096caa0c568b3` |
| `json-object-gpt-20261113-manifest.json` | `615b7636c8b4d2be918d603321cda973e6ee3c6e23d97ef6992155d1a367a30f` |
| Frozen input manifest | `79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3` |

This reporting increment changes no source, prompt, profile, task, threshold,
permission or supply-chain output. Reporting-only fast gate and direct Markdown
lint results are recorded with the review request. The last affected-source gate
recorded 53 inherited failures; it is not rerun for this report. The other
two prepared profiles must finish their development repeats before further
tuning, except high/medium review fixes. No task-quality improvement, native
qualification, row closure, release or owner-test readiness is claimed.
Independent core review of this measurement remains required.

# Complete-response GPT-OSS development repeat — 2026-11-12

T-303/T-306 remain open. All 30 frozen development tasks ran on product source
`974d2bb55afa6be47bd58ffcf1dd30a6450f70cd` with `gpt-oss`.
**5/30 strict ungraded proposals; task grades remain unavailable.**
No candidate code, test, feedback, retry or held-out task was executed.

## Path and changed variables

Only a partial coding path is measured: existing Rust source selection,
the credential-owning local chat API and the original Rust strict edit checker.
Selection receives the statement and the editable frozen base file; full
repository analysis and retrieval are not measured. Native executable artifact
admission and the missing original grading-controls receipt still block the
native edit/test/retry path. Neither boundary is bypassed and no original
control is reconstructed. An accepted proposal is not a solved task.

The same newly built repaired API and unchanged source-built helper used by the
A4B repeat are reused after verifying all 2,283 repository and 16 private input
hashes. API SHA-256 is
`41fa8967d6d9e93102798c39ee17f71c31b20e7da2b066163730830737de626e`;
helper SHA-256 is
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
The [independent source re-review](../reviews/974d2bb55afa-complete-response-empty-answer-repair.txt)
returns PASS for the exact repaired source, resolving medium M1. Low L1–L4
remain T-309 backlog; neither T-303 nor T-306 closes.

Changed variables against the preceding streaming repeat are complete JSON
framing and the explicitly selected existing 300-second idle budget instead of
60 seconds. Total remains 300 seconds. Context remains 32,768 tokens; output is
`min(32768, 32768 - rendered input tokens - 32)`, ranging from 24,219 to
29,748. Inputs range from 2,988 to 8,517; all 30 fit and **does not fit: 0**.
These are the largest allowances in the prepared 32K profile and transport
policy, not a claim about every physically possible context. Byte ceilings,
prompts, source selection, checker, tasks and thresholds remain unchanged.
No prompt or model role is adopted from this partial measurement.

The model weight SHA-256 is
`27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901`
(12,109,566,624 bytes). Its existing profile retains
four threads, temperature 0, seed 42, Q8 key/value caches, batch 256,
microbatch 128 and `reasoning_effort: medium`. Model/runtime bytes are reverified
at every launch; no download or new preparation qualification is claimed.
The recorded GPU driver package is `xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`;
the platform tuple is marked changed since preparation. These measurements do
not qualify the changed platform for coding.

## Results and cost

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 13 |
| `strict-edit-refused:code.proposal.edit` | 8 |
| `strict-edit-refused:code.proposal.format` | 4 |
| `ungraded-proposal` | 5 |
| Does not fit | 0 |
| First-attempt task passes | Unavailable |
| Within-budget task passes | Unavailable |

Proposal outcomes are counted separately from task grades. The table above
records every observed cause; no task is marked solved. Backend termination
counts are `{'output-cap': 13, 'stopped-below-token-limits': 17}`. Input and output counters include
non-answer tokens; a transport refusal alone does not establish its root cause.

All 13 incomplete refusals reached their per-task output allowance. This
identifies the effective token ceiling, not why no completed answer was
accepted. No deadline or byte-ceiling change follows from this report.

Request/check time is **2,743.824 seconds**.
Fifteen separate two-task jobs reserve **2,926.278
seconds / 48.771 GPU work minutes**, including
verification, load and cleanup, excluding queues and wrapper overhead. This is
job wall inside the reservation, not active kernel time. Backend final counters cover
30/30 attempts: **188,409 input and
539,597 output tokens**, including non-answer output.
Final counters are available for every attempt; the totals include all 30.
API usage was not requested. Both resource wrappers are used with a 900-second
cap per pair and release both reservations between pairs. Completed-job guard and cleanup
checks pass: settings revoked, model keys and sockets removed, children reaped.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8176 | 24560 | 18894 | `strict-edit-refused:code.proposal.edit` | 104.209 |
| dev-002 | 8239 | 24497 | 24497 | `generation-refused:error:chat.response.incomplete` | 130.596 |
| dev-003 | 4349 | 28387 | 28387 | `generation-refused:error:chat.response.incomplete` | 143.387 |
| dev-004 | 2988 | 29748 | 17057 | `strict-edit-refused:code.proposal.format` | 80.636 |
| dev-005 | 6766 | 25970 | 6320 | `strict-edit-refused:code.proposal.edit` | 35.826 |
| dev-006 | 7916 | 24820 | 3524 | `ungraded-proposal` | 19.253 |
| dev-007 | 8357 | 24379 | 2003 | `strict-edit-refused:code.proposal.format` | 16.505 |
| dev-008 | 8005 | 24731 | 24731 | `generation-refused:error:chat.response.incomplete` | 123.787 |
| dev-009 | 3006 | 29730 | 2960 | `strict-edit-refused:code.proposal.edit` | 19.313 |
| dev-010 | 3983 | 28753 | 15084 | `strict-edit-refused:code.proposal.edit` | 70.740 |
| dev-011 | 6033 | 26703 | 26703 | `generation-refused:error:chat.response.incomplete` | 136.123 |
| dev-012 | 3002 | 29734 | 29500 | `strict-edit-refused:code.proposal.format` | 142.978 |
| dev-013 | 8380 | 24356 | 24356 | `generation-refused:error:chat.response.incomplete` | 128.193 |
| dev-014 | 6009 | 26727 | 26727 | `generation-refused:error:chat.response.incomplete` | 129.882 |
| dev-015 | 8517 | 24219 | 14206 | `strict-edit-refused:code.proposal.format` | 73.845 |
| dev-016 | 8061 | 24675 | 8163 | `ungraded-proposal` | 40.553 |
| dev-017 | 4049 | 28687 | 25198 | `ungraded-proposal` | 127.137 |
| dev-018 | 3064 | 29672 | 29672 | `generation-refused:error:chat.response.incomplete` | 151.348 |
| dev-019 | 8241 | 24495 | 835 | `ungraded-proposal` | 11.803 |
| dev-020 | 7424 | 25312 | 2476 | `strict-edit-refused:code.proposal.edit` | 14.007 |
| dev-021 | 7672 | 25064 | 17566 | `ungraded-proposal` | 89.814 |
| dev-022 | 6827 | 25909 | 23663 | `strict-edit-refused:code.proposal.edit` | 116.044 |
| dev-023 | 7668 | 25068 | 25068 | `generation-refused:error:chat.response.incomplete` | 129.803 |
| dev-024 | 5159 | 27577 | 27577 | `generation-refused:error:chat.response.incomplete` | 134.466 |
| dev-025 | 4168 | 28568 | 28568 | `generation-refused:error:chat.response.incomplete` | 142.257 |
| dev-026 | 4820 | 27916 | 2136 | `strict-edit-refused:code.proposal.edit` | 11.014 |
| dev-027 | 7967 | 24769 | 24769 | `generation-refused:error:chat.response.incomplete` | 125.848 |
| dev-028 | 8157 | 24579 | 24579 | `generation-refused:error:chat.response.incomplete` | 119.928 |
| dev-029 | 7969 | 24767 | 5079 | `strict-edit-refused:code.proposal.edit` | 30.340 |
| dev-rust-001 | 3437 | 29299 | 29299 | `generation-refused:error:chat.response.incomplete` | 144.191 |

## Retention and verification

Fifteen two-task jobs release both reservations between pairs. The private
scheduler keeps its existing 30-second pause outside both reservations. It
changes no guard, model request or task budget. Completed-job GPU wall excludes
queues, wrapper overhead and those pauses. Original completed records are
preserved; no completed attempt is replayed or replaced. Existing timeouts remain unchanged; observed terminal causes are reported above.

The complete source/record audit verifies all frozen bindings, 30 unique tasks,
record and candidate/base hashes, contexts and requests, settings, resource
guards, cleanup, backend counters and costs. The original helper reproduces all
30 contexts and available strict checks without executing candidates or hidden
tests. The separate read/hash-only archive verifier checks **80,649
files**, including the exact 2,283-file source snapshot and all 16 private
inputs, and reproduces counts, counters and cost totals. It extracts nothing
and imports no archived code. The archive includes the preceding A4B, 31B and 27B records
unchanged; their existing archives and verifiers remain untouched. All 120 retired
API token files are excluded and no model weight is packaged. Raw inputs,
answers, candidates and logs remain private.

| Retained binding | SHA-256 |
| --- | --- |
| `complete-gpt-20261112-inputs.tar.gz` | `0201d9b3df56eccdd70150aac6b0043cef32c8bb20a3432c9c11ca2c13b2e94b` |
| `complete-gpt-20261112-manifest.json` | `f01a6f8d1ccf901727ca6dee03e60ec3e98d5e59e1f0e1f94bc1016e63bf6b3d` |
| Frozen input manifest | `f31465aa9af682aa1aada9ad79e9433f3cb6e6f8d3af06552f2952a9d7e5fe51` |

This reporting increment changes no product source, prompt, profile, hidden
test, threshold, permission, evidence binding or supply-chain output. All
existing ledger rows and earlier scoreboard entries remain unchanged. Two
other prepared profiles still require the full repeat before further tuning,
except high/medium review fixes. Independent core review is required. No row,
role, qualification, release or owner-test readiness is claimed.

Validation here is the reporting-only fast gate. The last affected-source gate
recorded 53 inherited failures; it was not rerun for this documentation increment.

# Complete-response 27B development repeat — 2026-11-12

T-303/T-306 remain open. All 30 frozen development tasks ran on product source
`974d2bb55afa6be47bd58ffcf1dd30a6450f70cd` with `qwen35-27b`.
**29/30 strict ungraded proposals; task grades remain unavailable.**
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
`min(32768, 32768 - rendered input tokens - 32)`, ranging from 24,137 to
29,804. Inputs range from 2,932 to 8,599; all 30 fit and **does not fit: 0**.
These are the largest allowances in the prepared 32K profile and transport
policy, not a claim about every physically possible context. Byte ceilings,
prompts, source selection, checker, tasks and thresholds remain unchanged.
No prompt or model role is adopted from this partial measurement.

The model weight SHA-256 is
`5ae89d6720ba0a27f643e6fbb0d1b557870d17aeff23156e5981e863ec16985c`
(16,547,400,032 bytes). Its existing profile retains
four threads, temperature 0, seed 42, Q8 key/value caches, batch 256,
microbatch 128 and `enable_thinking: false`. Model/runtime bytes are reverified
at every launch; no download or new preparation qualification is claimed.
The recorded GPU driver package is `xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`;
the platform tuple is marked changed since preparation. These measurements do
not qualify the changed platform for coding.

## Results and cost

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 1 |
| `ungraded-proposal` | 29 |
| Does not fit | 0 |
| First-attempt task passes | Unavailable |
| Within-budget task passes | Unavailable |

Proposal outcomes are counted separately from task grades. All 29 accepted
proposals have observed backend stops below their token limits. Dev-004 reaches
the existing 300-second request deadline, with no visible text event. Its
retained backend slice has no timing counters, so its termination and token
counts remain unavailable. The terminal timeout code does not establish the
root cause. Input and output counters include non-answer tokens.

Request/check time is **1,436.046 seconds**.
Fifteen separate two-task jobs reserve **1,860.960
seconds / 31.016 GPU work minutes**, including
verification, load and cleanup, excluding queues and wrapper overhead. This is
job wall inside the reservation, not active kernel time. Backend counters cover
29/30 attempts: **186,402 input and
34,920 output tokens**, including non-answer output.
Tasks without observed backend counters: dev-004.
Token totals exclude those attempts; missing counters are not zero tokens.
API usage was not requested. Both resource wrappers are used with a 900-second
cap per pair and release both reservations between pairs. Completed-job guard
and cleanup checks pass: settings revoked, model keys and sockets removed, children reaped.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8198 | 24538 | 1014 | `ungraded-proposal` | 39.900 |
| dev-002 | 8303 | 24433 | 1193 | `ungraded-proposal` | 33.899 |
| dev-003 | 4290 | 28446 | 723 | `ungraded-proposal` | 29.701 |
| dev-004 | 2932 | 29804 | Unknown | `generation-refused:error:chat.transport.timeout` | 300.009 |
| dev-005 | 7154 | 25582 | 306 | `ungraded-proposal` | 22.679 |
| dev-006 | 8024 | 24712 | 958 | `ungraded-proposal` | 29.023 |
| dev-007 | 8449 | 24287 | 2777 | `ungraded-proposal` | 79.831 |
| dev-008 | 8061 | 24675 | 3104 | `ungraded-proposal` | 77.076 |
| dev-009 | 2939 | 29797 | 169 | `ungraded-proposal` | 16.653 |
| dev-010 | 3913 | 28823 | 1133 | `ungraded-proposal` | 28.798 |
| dev-011 | 6062 | 26674 | 4157 | `ungraded-proposal` | 107.913 |
| dev-012 | 2938 | 29798 | 1218 | `ungraded-proposal` | 29.943 |
| dev-013 | 8428 | 24308 | 1914 | `ungraded-proposal` | 62.355 |
| dev-014 | 6075 | 26661 | 2074 | `ungraded-proposal` | 53.092 |
| dev-015 | 8599 | 24137 | 1110 | `ungraded-proposal` | 43.244 |
| dev-016 | 8150 | 24586 | 1766 | `ungraded-proposal` | 47.075 |
| dev-017 | 4060 | 28676 | 533 | `ungraded-proposal` | 30.983 |
| dev-018 | 3069 | 29667 | 814 | `ungraded-proposal` | 23.428 |
| dev-019 | 8273 | 24463 | 168 | `ungraded-proposal` | 25.943 |
| dev-020 | 7461 | 25275 | 315 | `ungraded-proposal` | 15.320 |
| dev-021 | 7724 | 25012 | 1022 | `ungraded-proposal` | 46.891 |
| dev-022 | 6841 | 25895 | 1127 | `ungraded-proposal` | 34.771 |
| dev-023 | 7630 | 25106 | 834 | `ungraded-proposal` | 38.212 |
| dev-024 | 5138 | 27598 | 730 | `ungraded-proposal` | 23.083 |
| dev-025 | 4153 | 28583 | 2235 | `ungraded-proposal` | 66.281 |
| dev-026 | 4821 | 27915 | 132 | `ungraded-proposal` | 7.405 |
| dev-027 | 8036 | 24700 | 966 | `ungraded-proposal` | 38.941 |
| dev-028 | 8176 | 24560 | 690 | `ungraded-proposal` | 23.769 |
| dev-029 | 8023 | 24713 | 692 | `ungraded-proposal` | 32.581 |
| dev-rust-001 | 3414 | 29322 | 1046 | `ungraded-proposal` | 27.247 |

## Retention and verification

Fifteen two-task jobs release both reservations between pairs. The private
scheduler keeps its existing 30-second pause outside both reservations. It
changes no guard, model request or task budget. Completed-job GPU wall excludes
queues, wrapper overhead and those pauses. Original completed records are
preserved; no completed attempt is replayed or replaced. The 300-second
timeout remains an observed failure.

The complete source/record audit verifies all frozen bindings, 30 unique tasks,
record and candidate/base hashes, contexts and requests, settings, resource
guards, cleanup, backend counters and costs. The original helper reproduces all
30 contexts and available strict checks without executing candidates or hidden
tests. The separate read/hash-only archive verifier checks **61,065
files**, including the exact 2,283-file source snapshot and all 16 private
inputs, and reproduces counts, counters and cost totals. It extracts nothing
and imports no archived code. The archive includes the preceding A4B and 31B
records unchanged; their existing archives and verifiers remain untouched.
All 90 retired
API token files are excluded and no model weight is packaged. Raw inputs,
answers, candidates and logs remain private.

| Retained binding | SHA-256 |
| --- | --- |
| `complete-qwen-20261112-inputs.tar.gz` | `d476c2dbe49c94eb99a420a6424dec1c73fee008cd61f33c3ae32eb4b0c2f6c2` |
| `complete-qwen-20261112-manifest.json` | `5a5421aa737c7a15b763f2c2c6e260d42f95b2cca979805987c3fcea82e89de8` |
| Frozen input manifest | `f31465aa9af682aa1aada9ad79e9433f3cb6e6f8d3af06552f2952a9d7e5fe51` |

This reporting increment changes no product source, prompt, profile, hidden
test, threshold, permission, evidence binding or supply-chain output. All
existing ledger rows and earlier scoreboard entries remain unchanged. Three
other prepared profiles still require the full repeat before further tuning,
except high/medium review fixes. Independent core review is required. No row,
role, qualification, release or owner-test readiness is claimed.

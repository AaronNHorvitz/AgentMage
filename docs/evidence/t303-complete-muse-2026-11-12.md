# Complete-response Muse development repeat — 2026-11-12

T-303/T-306 remain open. All 30 frozen development tasks ran on product source
`974d2bb55afa6be47bd58ffcf1dd30a6450f70cd` with `muse`.
**19/30 strict ungraded proposals; task grades remain unavailable.**
No candidate code, test, feedback, retry or held-out task was executed.
Campaign date denotes its start day; attempts span 2026-11-12–13 UTC.

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
`min(32768, 32768 - rendered input tokens - 32)`, ranging from 24,395 to
29,826. Inputs range from 2,910 to 8,341; all 30 fit and **does not fit: 0**.
These are the largest allowances in the prepared 32K profile and transport
policy, not a claim about every physically possible context. Byte ceilings,
prompts, source selection, checker, tasks and thresholds remain unchanged.
No prompt or model role is adopted from this partial measurement.

The model weight SHA-256 is
`4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`
(16,756,683,904 bytes). Its existing profile retains
four threads, temperature 0, seed 42, Q8 key/value caches, batch 256,
microbatch 128 and `reasoning_strength: medium`. Model/runtime bytes are reverified
at every launch; no download or new preparation qualification is claimed.
The recorded GPU driver package is `xorg-x11-drv-nvidia-cuda-615.71.09-3.fc44.x86_64`;
the platform tuple is marked changed since preparation. These measurements do
not qualify the changed platform for coding.

## Results and cost

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.timeout` | 3 |
| `strict-edit-refused:code.proposal.edit` | 3 |
| `strict-edit-refused:code.proposal.format` | 5 |
| `ungraded-proposal` | 19 |
| Does not fit | 0 |
| First-attempt task passes | Unavailable |
| Within-budget task passes | Unavailable |

Proposal outcomes are counted separately from task grades. The table above
records every observed cause; no task is marked solved. Backend termination
counts are `{'stopped-below-token-limits': 27, 'unobserved-or-ambiguous': 3}`.
Input and output counters use only lines after the task's own slot launch,
excluding preceding-request log prefixes, and include non-answer tokens.
A transport refusal alone does not establish its root cause.

Existing deadlines and output ceilings are unchanged. Backend terminations
are reported separately from transport outcomes; no new root cause is inferred.
The three timed-out attempts reached the deadline before their output allowances.
Any deadline change remains T-309 backlog.

Dev-004 has 99 progress counters, reaching a 13,407-token lower bound at
299.919 seconds after slot launch, before its 29,805-token allowance.
Dev-011 has 95 counters, reaching 12,517 at 297.552 seconds, before 26,793.
Dev-012 has 94 counters, reaching 12,512 at 286.791 seconds, before 29,795.
Their final token counters are unavailable; these lower bounds are excluded
from aggregate totals. The four counters before dev-012's own slot launch
belong to the preceding request and are excluded from its count.

Request/check time is **4,755.558 seconds**.
Fifteen separate two-task jobs reserve **5,328.730
seconds / 88.812 GPU work minutes**, including
verification, load and cleanup, excluding queues and wrapper overhead. This is
job wall inside the reservation, not active kernel time. Backend final counters cover
27/30 attempts: **174,326 input and
163,130 output tokens**, including non-answer output.
Tasks without final backend counters: dev-004, dev-011, dev-012.
Token totals exclude those attempts; missing final counters are not zero tokens.
API usage was not requested. Both resource wrappers are used with a 900-second
cap per pair and release both reservations between pairs. Completed-job guard and cleanup
checks pass: settings revoked, model keys and sockets removed, children reaped.

| Task | Input tokens | Output allowance | Observed output tokens | Cause | Request/check s |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8120 | 24616 | 8203 | `strict-edit-refused:code.proposal.edit` | 192.585 |
| dev-002 | 8227 | 24509 | 5864 | `ungraded-proposal` | 135.484 |
| dev-003 | 4298 | 28438 | 5527 | `strict-edit-refused:code.proposal.edit` | 131.369 |
| dev-004 | 2931 | 29805 | At least 13407; final unavailable | `generation-refused:error:chat.transport.timeout` | 300.008 |
| dev-005 | 6622 | 26114 | 3792 | `ungraded-proposal` | 93.976 |
| dev-006 | 7898 | 24838 | 5183 | `ungraded-proposal` | 121.017 |
| dev-007 | 8324 | 24412 | 8073 | `ungraded-proposal` | 200.090 |
| dev-008 | 7987 | 24749 | 10539 | `strict-edit-refused:code.proposal.format` | 251.912 |
| dev-009 | 2910 | 29826 | 2274 | `ungraded-proposal` | 61.929 |
| dev-010 | 3911 | 28825 | 5122 | `ungraded-proposal` | 123.411 |
| dev-011 | 5943 | 26793 | At least 12517; final unavailable | `generation-refused:error:chat.transport.timeout` | 300.011 |
| dev-012 | 2941 | 29795 | At least 12512; final unavailable | `generation-refused:error:chat.transport.timeout` | 300.009 |
| dev-013 | 8333 | 24403 | 12578 | `strict-edit-refused:code.proposal.format` | 294.566 |
| dev-014 | 5914 | 26822 | 7975 | `ungraded-proposal` | 180.518 |
| dev-015 | 8341 | 24395 | 4413 | `ungraded-proposal` | 110.153 |
| dev-016 | 7888 | 24848 | 6489 | `ungraded-proposal` | 149.597 |
| dev-017 | 3987 | 28749 | 4125 | `ungraded-proposal` | 100.507 |
| dev-018 | 3008 | 29728 | 5387 | `ungraded-proposal` | 121.912 |
| dev-019 | 8121 | 24615 | 869 | `ungraded-proposal` | 29.850 |
| dev-020 | 7308 | 25428 | 2914 | `ungraded-proposal` | 70.336 |
| dev-021 | 7558 | 25178 | 8192 | `ungraded-proposal` | 196.950 |
| dev-022 | 6715 | 26021 | 8371 | `strict-edit-refused:code.proposal.format` | 191.232 |
| dev-023 | 7619 | 25117 | 4066 | `ungraded-proposal` | 100.847 |
| dev-024 | 5103 | 27633 | 2797 | `ungraded-proposal` | 64.859 |
| dev-025 | 4109 | 28627 | 7631 | `strict-edit-refused:code.proposal.edit` | 176.859 |
| dev-026 | 4704 | 28032 | 1967 | `ungraded-proposal` | 46.925 |
| dev-027 | 7971 | 24765 | 13044 | `strict-edit-refused:code.proposal.format` | 298.852 |
| dev-028 | 8131 | 24605 | 7426 | `strict-edit-refused:code.proposal.format` | 168.292 |
| dev-029 | 7852 | 24884 | 3281 | `ungraded-proposal` | 83.810 |
| dev-rust-001 | 3367 | 29369 | 7028 | `ungraded-proposal` | 157.692 |

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
tests. The separate read/hash-only archive verifier checks **100,248
files**, including the exact 2,283-file source snapshot and all 16 private
inputs, and reproduces counts, counters and cost totals. It extracts nothing
and imports no archived code. The archive includes the preceding A4B, 31B, 27B and GPT-OSS records
unchanged; their existing archives and verifiers remain untouched. All 150 retired
API token files are excluded and no model weight is packaged. Raw inputs,
answers, candidates and logs remain private.

| Retained binding | SHA-256 |
| --- | --- |
| `complete-muse-20261112-inputs.tar.gz` | `513201bbd6784d40ae5fb9b226bdb9862d9cc97378be4587e536124a58ca61e1` |
| `complete-muse-20261112-manifest.json` | `cce1d834b55043917a85228678c02cfa4e1e7efca914dab6144e65cb4721c7a3` |
| Frozen input manifest | `f31465aa9af682aa1aada9ad79e9433f3cb6e6f8d3af06552f2952a9d7e5fe51` |

This reporting increment changes no product source, prompt, profile, hidden
test, threshold, permission, evidence binding or supply-chain output. All
existing ledger rows and earlier scoreboard entries remain unchanged. One
other prepared profile still requires the full repeat before further tuning,
except high/medium review fixes. Independent core review is required. No row,
role, qualification, release or owner-test readiness is claimed.

Validation here is the reporting-only fast gate. The last affected-source gate
recorded 53 inherited failures; it was not rerun for this documentation increment.

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

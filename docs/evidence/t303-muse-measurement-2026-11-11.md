# Muse development measurement — 2026-11-11

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on the
prepared `muse` artifact through Rust context selection, credential-owning chat
and strict edit checking. **Partial path: task first-attempt and within-budget
passes remain unavailable, not zero.** Native Git executable admission and missing
original grading controls prevent native execution and task grading. No native
tools, worktree edits, candidate tests, test feedback, retries, approvals, apply,
commit, push or held-out tasks ran. Strict proposals are ungraded edits, never
solved tasks. Selection receives only the statement and editable base file;
full repository analysis and retrieval are not measured. Neither native trust
nor original control admission is bypassed or reconstructed.

Changed variables versus this artifact's November 10/11 measurement: complete
source lines and merged/canonical excerpts with the exact-edit instruction;
total/idle 300/60 s (was 120/30), private client/relay 310 s (was 130) and two-task
cold-load jobs 900 s (was 600). Context remains 32768 tokens and selection 32768
bytes. Output allowance is `min(32768, 32768 - rendered_input_tokens - 32)`,
including reasoning. The private effect-free adapter uses the existing chat
allowance; the product proposal route stays capped at 4096 output tokens.
All existing byte caps and strict refusals remain. Combined changes do not
isolate which variable changed an outcome. 32K is the largest prepared context
tested, not an exhaustive hardware-limit search. Runtime, read-only weights,
temperature zero, seed 42 and family template settings match the preceding
campaign. All 30 contexts match the current-source primary-model run byte for
byte; tokenization and output allowances differ by model.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-11 | muse | 32768 | 24394–29889 | unavailable / unavailable (30 tasks) | 18/30 | 4945.146 | 5674.078 | 94.568 |

Does-not-fit tasks: 0. One attempt per task, zero retries and no grade.
Transport timeouts: 2; the November 10/11 run at 120/30 s had 18.
That November 10/11 run had 10/30 strict ungraded proposals; neither count is a task
pass rate. GPU work minutes mean reserved model-job wall/60, including
verification, loading and cleanup, excluding queue and wrapper checks.
Active GPU kernel time was not measured. Request/check time excludes context
preparation and preflight. All 15 two-task jobs used both shared reservations
and released them after completion. The first two attempts predate the smaller
model's repeat and were retained unchanged; the controller resumed from task 3,
without rerunning or dropping earlier attempts. No model was downloaded.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.incomplete` | 1 |
| `generation-refused:error:chat.transport.timeout` | 2 |
| `strict-edit-refused:code.proposal.edit` | 6 |
| `strict-edit-refused:code.proposal.format` | 3 |
| `ungraded-proposal` | 18 |

Unambiguous backend counters cover 28/30 tasks:
171088 input / 184996 output tokens.
API usage was not requested; other counters remain unknown.
Largest individual refusal: `strict-edit-refused:code.proposal.edit` (6).
Three other prepared models still need complete repeats before model comparison
or unrelated source work. T-303/T-306 remain open without role selection,
new rows/sub-rows, completion ticks or native coding/release claims. Exact
bindings and per-task observations follow. These measurements do not establish
task-solving improvement.

## Verification and retained inputs

The artifact audit verifies 30 tasks and 15 completed two-task jobs, source and
private input hashes, settings, model/runtime identity, request and candidate/log
bindings, terminal events, observed counters and cleanup. The previously clean
source-built helper reproduces all 30 contexts and strict checks without running
any candidate or hidden test. A separate comparison verifies byte-identical
contexts against the current-source primary-model run. Raw prompts, answers,
candidates, authentication files and host paths stay private. Shared inert
inputs and an additive restore helper permit independent reproduction without
changing any earlier manifest-bound bytes. No product source changed; the
prior affected source gate still has 53 inherited failures. No new full source
gate is claimed. The documentation fast-gate log accompanies the review request.
A network-disabled restore into empty private state verifies all 39,268 bound
files. Restoring over the earlier 21,091-file inputs and repeating restoration
also pass. A deliberately conflicting cold-copy file is refused without being
overwritten. No model, candidate or hidden test runs during those checks.

The current manifest binds every existing regular file in this measurement root,
including the previous manifest and retained first two Muse attempts. All bound
files and all prior archives stay immutable. Later campaign outputs must use
new paths. The restore helper adds missing files and checks existing bytes;
it refuses conflicts and never overwrites existing files.

| Binding | SHA-256 |
| --- | --- |
| Model weights | `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e` |
| Runtime manifest | `f007bf5a668a88d2c00446f4233a073274cc529bde05bb9c37cd91f5a82f9ba1` |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private freeze | `972c69479e9858ad0e349c020851273980a3300761eee11e23d7f42785e202e9` |
| API binary | `72ca684879583ce1d3e9abc6ab26db7d475d74e3ae6d1fc9b946269986de4965` |
| Original Rust helper | `0e06f7b880e0b6584a6e823249187db5e275a2c20c8a1edffa661189e1759fe0` |
| Audited results | `8dcee78dbaa347d46d83ff2bd9c9c04212797e4840ebe3136f67008a0f97b7c5` |
| Fresh-source reproduction | `cacdda8730ea4713e67ce19f3789fea67f42917e2088be3605afc515f0203181` |
| Cross-model context comparison | `0b805cbb24c3a2261dd118b671b2312f831f0edeab0a263fefc1d6b0cd617c8a` |
| Current retained manifest | `9e0619aaf77ebb6b741970cd54e7b3da5da5f379e431cc9d043f5b8c620b85cf` |
| Previous unchanged manifest | `4ee396cad2abea509976b7d709c10306639ceb62fa20bd2fd9295322e540c0f1` |

## Per-task observations

Each row is one attempt, zero retries and no task grade.

| Task | Preflight input tokens | Output allowance | Request/check s | Observed input / output | Outcome |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8121 | 24615 | 145.152 | 8121 / 5737 | `strict-edit-refused:code.proposal.edit` |
| dev-002 | 8264 | 24472 | 132.310 | 8264 / 5564 | `strict-edit-refused:code.proposal.edit` |
| dev-003 | 4235 | 28501 | 164.460 | 4235 / 7031 | `strict-edit-refused:code.proposal.format` |
| dev-004 | 2868 | 29868 | 284.424 | 2868 / 12696 | `strict-edit-refused:code.proposal.format` |
| dev-005 | 6559 | 26177 | 132.179 | 6559 / 5429 | `ungraded-proposal` |
| dev-006 | 7913 | 24823 | 177.908 | 7913 / 7717 | `generation-refused:error:chat.response.incomplete` |
| dev-007 | 8312 | 24424 | 206.690 | 8312 / 8795 | `ungraded-proposal` |
| dev-008 | 8007 | 24729 | 179.503 | 8007 / 7853 | `strict-edit-refused:code.proposal.format` |
| dev-009 | 2847 | 29889 | 68.668 | 2847 / 2698 | `ungraded-proposal` |
| dev-010 | 3848 | 28888 | 197.267 | 3848 / 8740 | `ungraded-proposal` |
| dev-011 | 5880 | 26856 | 300.021 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-012 | 2878 | 29858 | 224.880 | 2878 / 9881 | `ungraded-proposal` |
| dev-013 | 8329 | 24407 | 256.755 | 8329 / 10971 | `ungraded-proposal` |
| dev-014 | 5851 | 26885 | 227.085 | 5851 / 9960 | `ungraded-proposal` |
| dev-015 | 8342 | 24394 | 131.053 | 8342 / 5302 | `ungraded-proposal` |
| dev-016 | 7877 | 24859 | 145.824 | 7877 / 6264 | `ungraded-proposal` |
| dev-017 | 3924 | 28812 | 168.691 | 3924 / 6998 | `strict-edit-refused:code.proposal.edit` |
| dev-018 | 2945 | 29791 | 108.480 | 2945 / 4755 | `ungraded-proposal` |
| dev-019 | 8091 | 24645 | 36.100 | 8091 / 1009 | `ungraded-proposal` |
| dev-020 | 7245 | 25491 | 72.194 | 7245 / 3003 | `ungraded-proposal` |
| dev-021 | 7495 | 25241 | 245.189 | 7495 / 10269 | `strict-edit-refused:code.proposal.edit` |
| dev-022 | 6652 | 26084 | 191.196 | 6652 / 8281 | `ungraded-proposal` |
| dev-023 | 7556 | 25180 | 119.507 | 7556 / 4756 | `ungraded-proposal` |
| dev-024 | 5040 | 27696 | 66.917 | 5040 / 2863 | `ungraded-proposal` |
| dev-025 | 4046 | 28690 | 214.061 | 4046 / 9231 | `strict-edit-refused:code.proposal.edit` |
| dev-026 | 4641 | 28095 | 49.182 | 4641 / 2059 | `ungraded-proposal` |
| dev-027 | 7982 | 24754 | 300.012 | unknown | `generation-refused:error:chat.transport.timeout` |
| dev-028 | 8109 | 24627 | 161.598 | 8109 / 6994 | `strict-edit-refused:code.proposal.edit` |
| dev-029 | 7789 | 24947 | 95.352 | 7789 / 3788 | `ungraded-proposal` |
| dev-rust-001 | 3304 | 29432 | 142.489 | 3304 / 6352 | `ungraded-proposal` |

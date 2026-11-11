# 26B A4B development measurement — 2026-11-11

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on the
prepared `gemma4-26b-a4b` artifact through Rust context selection, credential-owning chat
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
| 2026-11-11 | gemma4-26b-a4b | 32768 | 22998–29579 | unavailable / unavailable (30 tasks) | 0/30 | 4275.841 | 4800.543 | 80.009 |

Does-not-fit tasks: 0. One attempt per task, zero retries and no grade.
Complete answers reaching strict checking: 1/30.
Transport timeouts: 0; the November 10/11 run at 120/30 s had 29.
That November 10/11 run had 0/30 strict ungraded proposals; neither count is a task
pass rate. GPU work minutes mean reserved model-job wall/60, including
verification, loading and cleanup, excluding queue and wrapper checks.
Active GPU kernel time was not measured. Request/check time excludes context
preparation and preflight. All 15 two-task jobs used both shared reservations
and released them after completion. The first two attempts were retained
unchanged after a controller-only stop between jobs. An active pair on another model finished normally and remains
partial progress; the resumed controller completed this model from task 3
without rerunning or dropping earlier attempts. No model was downloaded.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.transport.limit` | 29 |
| `strict-edit-refused:code.proposal.format` | 1 |

Unambiguous backend counters cover 1/30 tasks:
5329 input / 10262 output tokens.
API usage was not requested; other counters remain unknown.
Largest individual refusal: `generation-refused:error:chat.transport.limit` (29).
Two other prepared models still need complete repeats before model comparison
or unrelated source work. T-303/T-306 remain open without role selection,
new rows/sub-rows, completion ticks or native coding/release claims.
Exact bindings and per-task observations follow. These measurements do not
establish task-solving improvement.

## Verification and retained inputs

The artifact audit verifies 30 tasks and 15 completed two-task jobs, source and
private input hashes, settings, model/runtime identity, request and candidate/log
bindings, terminal events, observed counters and cleanup. The previously clean
source-built helper reproduces all 30 contexts and the one available
strict check without running any candidate or hidden test. No new clean build
is claimed. A separate comparison verifies byte-identical
contexts against the current-source primary-model run. An initial comparison
refused the not-yet-written audit input; its empty output and ordering note are
retained separately. The final comparison ran after the audit passed. No
model attempt is repeated. Raw prompts, answers,
candidates, authentication files and host paths stay private. Shared inert
inputs and an additive restore helper permit independent reproduction without
changing any earlier manifest-bound bytes. No product source changed; the
prior affected source gate still has 53 inherited failures; no fresh full source
gate is claimed. The first documentation gate could not find its JavaScript
runtime on `PATH`. The final gate uses an existing hash-verified local runtime;
no tooling was downloaded. Both logs accompany the review request.
A network-disabled empty-state restore verifies all 60,276 bound files. A repeated
restore and an additive restore over the previous 39,268-file inputs also pass.
A deliberately conflicting cold-copy file is refused without overwriting it.
A planted symlinked directory is refused without changing the outside directory.
The temporary cold state is removed. No model, candidate or hidden test runs
during restoration checks.

The current manifest binds every existing regular file in this measurement root,
including previous manifests and the retained first two attempts. All bound
files and all prior archives stay immutable. Later campaign outputs must use
new paths. The restore helper adds missing files and checks existing bytes;
it refuses conflicts and never overwrites existing files.

| Binding | SHA-256 |
| --- | --- |
| Model weights | `a07f72221e8e3f77455ab0d7f7652d01a9f63c262b954aa6932a53275a0e895a` |
| Runtime manifest | `f007bf5a668a88d2c00446f4233a073274cc529bde05bb9c37cd91f5a82f9ba1` |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private freeze | `972c69479e9858ad0e349c020851273980a3300761eee11e23d7f42785e202e9` |
| API binary | `72ca684879583ce1d3e9abc6ab26db7d475d74e3ae6d1fc9b946269986de4965` |
| Original Rust helper | `0e06f7b880e0b6584a6e823249187db5e275a2c20c8a1edffa661189e1759fe0` |
| Audited results | `441afbd263887ac5aba37320ffba070d072d08c497df8284ca7c069e47701aec` |
| Source-built helper reproduction | `f1b5ee01c7a5dcd138ee0083aabc741c944a7a8575c61cfa18733761b6a95ac3` |
| Cross-model context comparison | `b85f3fe6c2cbfdcf8e5ae0bc14f6196800f6d9c0b5a3757db38f2b55b43c0133` |
| Current retained manifest | `730c2869113124d68afe369b967f2c99d76c6f5273f4aea18cb56a70952fa026` |
| Previous unchanged Muse manifest | `9e0619aaf77ebb6b741970cd54e7b3da5da5f379e431cc9d043f5b8c620b85cf` |

## Per-task observations

Each row is one attempt, zero retries and no task grade.

| Task | Preflight input tokens | Output allowance | Request/check s | Observed input / output | Outcome |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 9500 | 23236 | 159.981 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-002 | 9621 | 23115 | 121.802 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-003 | 4686 | 28050 | 146.109 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-004 | 3157 | 29579 | 120.630 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-005 | 8394 | 24342 | 152.909 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-006 | 9337 | 23399 | 123.633 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-007 | 9724 | 23012 | 152.935 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-008 | 9424 | 23312 | 125.446 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-009 | 3345 | 29391 | 156.752 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-010 | 4249 | 28487 | 124.323 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-011 | 6618 | 26118 | 157.126 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-012 | 3179 | 29557 | 163.076 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-013 | 9738 | 22998 | 153.949 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-014 | 6929 | 25807 | 136.684 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-015 | 9529 | 23207 | 156.045 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-016 | 9052 | 23684 | 124.323 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-017 | 4482 | 28254 | 148.869 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-018 | 3453 | 29283 | 124.433 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-019 | 9323 | 23413 | 166.070 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-020 | 8398 | 24338 | 133.896 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-021 | 8675 | 24061 | 169.696 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-022 | 7688 | 25048 | 131.494 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-023 | 8800 | 23936 | 177.718 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-024 | 5878 | 26858 | 129.744 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-025 | 4745 | 27991 | 167.082 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-026 | 5329 | 27407 | 83.250 | 5329 / 10262 | `strict-edit-refused:code.proposal.format` |
| dev-027 | 9386 | 23350 | 160.259 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-028 | 9446 | 23290 | 125.179 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-029 | 9008 | 23728 | 158.919 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-rust-001 | 3629 | 29107 | 123.508 | unknown | `generation-refused:error:chat.transport.limit` |

# Small-model development measurement — 2026-11-11

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on the prepared
`gemma4-e4b` artifact through Rust context selection, credential-owning chat and
strict edit checking. **Partial path: task first-attempt and within-budget passes
remain unavailable, not zero.** Native executable admission and missing original
grading controls still prevent the complete coding loop and task grading. No
candidate tests, retries, held-out tasks, native tools, worktree edits, approvals,
apply, commit or push ran. Strict proposals are ungraded edits, never solved tasks.
No role selection, coding admission or row closure follows.
Selection receives only each task statement and its editable base file; full
repository analysis and retrieval are not measured. The native Git artifact
refuses its existing root-ownership admission in this lane; original grading
control receipts remain absent. Neither boundary is bypassed or reconstructed.

Changed variables versus this artifact's November 10/11 measurement: complete
source lines and merged/canonical excerpts with the exact-edit instruction;
total/idle 300/60 s (was 120/30); private client/relay 310 s (was 130); two-task
cold-load jobs 900 s (was 600). Context remains 32768 tokens and selection 32768
bytes. Output allowance is `min(32768, 32768 - rendered_input_tokens - 32)`,
including reasoning. All existing byte caps and strict refusals remain. The same
private effect-free adapter uses the chat allowance; the product proposal route
remains capped at 4096 output tokens. Context and time changes are combined;
these results do not isolate which variable changed an outcome. 32K is the largest
prepared context tested, not an exhaustive hardware-limit search. Runtime, weights,
temperature zero, seed 42 and default family settings match the preceding campaign.
All 30 selected contexts match the preceding current-source GPT-OSS run byte for
byte; backend tokenization and output allowances differ by model.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-11 | gemma4-e4b | 32768 | 22998–29579 | unavailable / unavailable (30 tasks) | 10/30 | 1592.324 | 1733.627 | 28.894 |

Does-not-fit tasks: 0. One attempt per task, zero retries and no grade.
Transport timeouts: 0 (the November 10/11 run at 120/30 s had 1).
The November 10/11 run at 120/30 s had 10/30 strict ungraded proposals; neither count
is a task pass rate. GPU work minutes mean reserved model-job wall/60, including
verification, loading and cleanup; queue and wrapper checks are excluded. Active
GPU kernel time was not measured. Request/check time excludes context preparation
and preflight. Every pair used both shared reservations and released them on
completion. No weights were downloaded or modified.

| Outcome / cause | Tasks |
| --- | --- |
| `strict-edit-refused:code.proposal.edit` | 3 |
| `strict-edit-refused:code.proposal.format` | 17 |
| `ungraded-proposal` | 10 |

Unambiguous backend counters cover 30/30 tasks:
214722 input / 142033 output tokens.
API usage was not requested; other counters remain unknown.
Largest individual refusal: `strict-edit-refused:code.proposal.format` (17).
Four other models still need complete repeats. Muse completed only its first two
tasks before a controller-only stop between jobs; those retained attempts are
unfinished campaign progress, not a 30-task measurement, and must not be silently
repeated. Scheduling the smaller model first changed no model input or budget.
The complete six-model repeat remains unfinished; no unrelated source work is
justified by this subset. T-303/T-306 remain open. Exact bindings and observations follow.

## Verification and retained inputs

The artifact audit verifies all 30 tasks and 15 completed two-task jobs, frozen
source/private inputs, runtime/model profiles, settings, request and candidate/log
bindings, terminal events, observed counters and cleanup. A fresh offline source
build reproduces every context and strict check without executing candidate code
or hidden tests. All contexts also match the retained primary-model run. The
original source-coverage root remains unchanged. Raw prompts, answers, candidates,
authentication files and machine paths remain private. Independent review uses
a shared inert archive, manifest and safe restoration helper. No product source
changed in this measurement batch; the prior affected source gate's 53 inherited
failures remain open. The final documentation fast gate passes, including actual
Markdown lint and verbatim-review checks. A network-disabled restore into empty
private state verifies all 21,091 manifest-bound files without running any model,
candidate or hidden test.

The manifest freezes complete existing files, including common launchers, all
small-model outputs and the two-task Muse progress. Future campaign outputs may
be added under new paths; no manifest-bound file or completed attempt may be
changed or overwritten. The review archive remains immutable. Future audits and
controller logs must use new filenames so the current review inputs remain exact.

| Binding | SHA-256 |
| --- | --- |
| Model weights | `d35a3aa7a6d47de237fd488bb6f7f3efd62908d828f85a63da0ee93acdbefe9f` |
| Runtime manifest | `f007bf5a668a88d2c00446f4233a073274cc529bde05bb9c37cd91f5a82f9ba1` |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private freeze | `972c69479e9858ad0e349c020851273980a3300761eee11e23d7f42785e202e9` |
| API binary | `72ca684879583ce1d3e9abc6ab26db7d475d74e3ae6d1fc9b946269986de4965` |
| Original Rust helper | `0e06f7b880e0b6584a6e823249187db5e275a2c20c8a1edffa661189e1759fe0` |
| Audited results | `2c802d61dde72de380f77da83528b1ad5fd29bda35c0b6383c3dc5576ede84e9` |
| Fresh-source reproduction | `6c1772edfe741f34dd9f18c3d4e0a0ba4d286309aed667af05a60c27c8437151` |
| Cross-model context comparison | `bda014b1c1b4e6b52578c7f3d9bed375654ff686d0e14a1d217c8efc299f75b1` |
| Retained manifest | `4ee396cad2abea509976b7d709c10306639ceb62fa20bd2fd9295322e540c0f1` |

## Per-task observations

Every row is one attempt, zero retries and no task grade.

| Task | Preflight input tokens | Output allowance | Request/check s | Observed input / output | Outcome |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 9500 | 23236 | 57.405 | 9500 / 4666 | `ungraded-proposal` |
| dev-002 | 9621 | 23115 | 28.198 | 9621 / 3482 | `ungraded-proposal` |
| dev-003 | 4686 | 28050 | 53.879 | 4686 / 4590 | `strict-edit-refused:code.proposal.format` |
| dev-004 | 3157 | 29579 | 74.744 | 3157 / 9104 | `strict-edit-refused:code.proposal.format` |
| dev-005 | 8394 | 24342 | 54.392 | 8394 / 3138 | `ungraded-proposal` |
| dev-006 | 9337 | 23399 | 21.364 | 9337 / 2465 | `ungraded-proposal` |
| dev-007 | 9724 | 23012 | 57.297 | 9724 / 4058 | `strict-edit-refused:code.proposal.format` |
| dev-008 | 9424 | 23312 | 55.117 | 9424 / 6878 | `strict-edit-refused:code.proposal.format` |
| dev-009 | 3345 | 29391 | 47.964 | 3345 / 2274 | `strict-edit-refused:code.proposal.format` |
| dev-010 | 4249 | 28487 | 36.515 | 4249 / 4729 | `ungraded-proposal` |
| dev-011 | 6618 | 26118 | 77.661 | 6618 / 5871 | `strict-edit-refused:code.proposal.format` |
| dev-012 | 3179 | 29557 | 40.881 | 3179 / 5028 | `ungraded-proposal` |
| dev-013 | 9738 | 22998 | 96.992 | 9738 / 7360 | `strict-edit-refused:code.proposal.format` |
| dev-014 | 6929 | 25807 | 41.961 | 6929 / 5051 | `strict-edit-refused:code.proposal.edit` |
| dev-015 | 9529 | 23207 | 97.901 | 9529 / 7560 | `ungraded-proposal` |
| dev-016 | 9052 | 23684 | 63.063 | 9052 / 7231 | `strict-edit-refused:code.proposal.format` |
| dev-017 | 4482 | 28254 | 58.573 | 4482 / 2443 | `strict-edit-refused:code.proposal.edit` |
| dev-018 | 3453 | 29283 | 34.584 | 3453 / 4298 | `ungraded-proposal` |
| dev-019 | 9323 | 23413 | 34.505 | 9323 / 598 | `strict-edit-refused:code.proposal.format` |
| dev-020 | 8398 | 24338 | 36.014 | 8398 / 4327 | `strict-edit-refused:code.proposal.format` |
| dev-021 | 8675 | 24061 | 43.763 | 8675 / 2045 | `strict-edit-refused:code.proposal.format` |
| dev-022 | 7688 | 25048 | 62.225 | 7688 / 7623 | `strict-edit-refused:code.proposal.format` |
| dev-023 | 8800 | 23936 | 76.576 | 8800 / 6304 | `strict-edit-refused:code.proposal.format` |
| dev-024 | 5878 | 26858 | 32.921 | 5878 / 4059 | `strict-edit-refused:code.proposal.format` |
| dev-025 | 4745 | 27991 | 78.185 | 4745 / 6315 | `strict-edit-refused:code.proposal.format` |
| dev-026 | 5329 | 27407 | 17.894 | 5329 / 2209 | `ungraded-proposal` |
| dev-027 | 9386 | 23350 | 65.583 | 9386 / 4679 | `ungraded-proposal` |
| dev-028 | 9446 | 23290 | 33.059 | 9446 / 3916 | `strict-edit-refused:code.proposal.format` |
| dev-029 | 9008 | 23728 | 68.424 | 9008 / 4033 | `strict-edit-refused:code.proposal.format` |
| dev-rust-001 | 3629 | 29107 | 44.684 | 3629 / 5699 | `strict-edit-refused:code.proposal.edit` |

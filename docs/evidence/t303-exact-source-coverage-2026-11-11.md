# Exact source coverage — 2026-11-11

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`.

Decision 0203 addresses the preceding primary-model development measurement's
largest individual refusal cause: eight strict edit refusals. Parser-backed
excerpts now retain complete original lines, including indentation and newlines.
Overlapping or touching ranges merge when the serialized message fits the
existing ceiling; a complete file can replace smaller selected excerpts. Retained
ranges have canonical path/offset order. The fixed instruction explicitly asks
for exact whitespace, distinguishing context and effective changes.

The checker and proposal data definitions are byte-identical to the preceding
commit. Ambiguous, invisible, altered, overlapping, ineffective and stale
preimages still refuse the entire proposal. No fuzzy application, normalization,
evaluation-test change, admission relaxation or tool authority is introduced.

## Verification

The complete affected fast gate is **FAIL**: 1,020 passed, 53 failed, 11 ignored,
zero measured or filtered, across 29 unfiltered targets. Every failure and ignore
identity in a shared target matches the retained candidate-build baseline.
Five Linux targets from that broader baseline are outside this change's affected
scope; their previous results are not new verification. Formatting, Clippy with
warnings denied, Markdown lint and untracked whitespace checks pass. The new
three-method integration target passes in the full run, alongside the existing
strict proposal tests. No timeout or additional skip was introduced.

The regressions exercise a complete file replacing smaller definitions,
cross-definition edits, exact indentation/CRLF/Unicode boundaries under a tight
budget, deterministic canonical ranges, omitted-source refusal and refusal of
newline normalization. These are component tests, not a native coding workflow.

An effect-free diagnostic rechecks the preceding run's 21 completed answers
against the new context. Two previously refused proposals become strict proposals;
the other refusals remain. This is saved-answer reuse, with no inference, test
execution or task grade. It is not a scoreboard measurement or a solve-rate gain.

The complete gate log has SHA-256
`2be8c7f6da0f8784d61963b6b31db051e2a99dd90bd07f7d6b4e000cb5aded81`.
The private review inputs retain the exact log and target/failure comparison.
The development remeasurement below is a partial path. T-303/T-306 stay open;
an independent core verdict is required.

## Development remeasurement

Source: `42612ca9787f07e4df79402e6d92f39f578e5c67`. All 30 frozen development tasks ran once on GPT-OSS through
Rust retrieval, the credential-owning chat API and the unchanged Rust strict
checker. **Partial path: first-attempt and within-budget task passes remain
unavailable, not zero.** No candidate tests, retries, held-out tasks, native
worktree/tool execution, apply/commit/push approvals or durable coding workflow
ran. Native admission and original grading controls remain blockers. The other
five prepared models have not been measured on this source; their resumable
campaign is pending. No model role or coding admission is selected.

Changed variables from the preceding 300/60-second primary-model repeat: complete
original lines and merged/canonical excerpts, plus the clarified exact-edit
instruction. Rendered input and output allowances change with that context.
Unchanged: 32768 context tokens, 32768 serialized selection bytes, output formula
`min(32768, 32768 - rendered_input_tokens - 32)`, total/idle 300/60 seconds,
private client/relay 310 seconds, two-task cold-load jobs 900 seconds, all existing
byte caps, temperature zero, configured seed 42 and medium reasoning. The product
proposal route remains capped at 4096 output tokens; the same private effect-free
adapter uses the larger existing chat allowance. No output ceiling was raised.
These observations do not isolate selection from instruction changes, and fixed
settings do not establish deterministic model output or task-solving improvement.

| Date (UTC) | Model | Context tokens | Output-token allowance | First / within budget task passes | Strict ungraded proposals | Request/check s | Reserved job wall s | GPU work min |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-11 | gpt-oss | 32768 | 24223–29812 | unavailable / unavailable (30 tasks) | 11/30 | 1515.472 | 1692.982 | 28.216 |

Does-not-fit tasks: 0. One attempt per task, zero retries.
Strict proposals are ungraded edits, never solved tasks. This run shows no
increase in strict proposal acceptance: 11/30 versus 12/30.
Task-solving quality remains unmeasured. Request/check time excludes
context preparation/preflight. GPU work minutes mean reserved model-job wall/60,
including verification/loading/cleanup and excluding queue/wrapper guards;
active GPU kernel time was not measured. Unambiguous backend token counters
cover 22/30 tasks: 139703 input / 146413 output.
Other counters are unknown; API usage was not requested.

| Outcome / cause | Tasks |
| --- | --- |
| `generation-refused:error:chat.response.invalid` | 1 |
| `generation-refused:error:chat.transport.limit` | 8 |
| `strict-edit-refused:code.proposal.edit` | 7 |
| `strict-edit-refused:code.proposal.format` | 3 |
| `ungraded-proposal` | 11 |

Largest individual refusal: `generation-refused:error:chat.transport.limit` (8).
Use these results for the next development investigation, after any arriving
high/medium review findings. They do not authorize an unrelated limit or
bookkeeping increment. No held-out evidence or row closure is claimed.
Exact bindings and task-level observations follow. T-303/T-306 stay open.

## Measurement verification

The artifact audit verifies all 30 tasks and 15 completed two-task jobs, source/private
hashes, exact model/runtime/profile identity, context/request and candidate/log
bindings, event terminals, counters and cleanup. Independent Rust reproduction
recreates all contexts and strict checks without executing candidates or hidden
tests. The original helper was built cleanly offline before inference. An
additional builder recompile did not obtain a shared reservation and never
started; its queue log is retained. Independent review commands require a fresh
source build and replay.
Raw prompts, answers, candidates and local credentials remain private. The exact
review request supplies shared retained inputs and fresh-build commands. The API
and original inference helper were separately built successfully before freezing.
Only reporting/verification tooling was added after inference began.

| Binding | SHA-256 |
| --- | --- |
| Development set | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Source/private freeze | `ed84819660877e688d3532bf986940b4ab2be0aceef8697741f98ac3c83d13ed` |
| API binary | `72ca684879583ce1d3e9abc6ab26db7d475d74e3ae6d1fc9b946269986de4965` |
| Rust helper | `0e06f7b880e0b6584a6e823249187db5e275a2c20c8a1edffa661189e1759fe0` |
| Audited results | `e24a3b29192eeade6e7bcb9857534631c4a8b6b192d4979949eeeace2bd3de42` |
| Rust reproduction | `7b4b1007ed9b8fd955e5e439eb50da5b9dfd12cff0d5377aa01c4ce7b3190f25` |
| Retained manifest | `4a883ee51f6f1f3664ef1eaf1eb997368bc4d3d54d627e917f88853abd1b4eb3` |

## Per-task observations

Every task has one attempt, zero retries and no task grade.

| Task | Preflight input tokens | Output allowance | Request/check s | Observed input / output | Outcome |
| --- | --- | --- | --- | --- | --- |
| dev-001 | 8175 | 24561 | 52.308 | 8175 / 9331 | `strict-edit-refused:code.proposal.edit` |
| dev-002 | 8272 | 24464 | 38.865 | 8272 / 7352 | `strict-edit-refused:code.proposal.edit` |
| dev-003 | 4285 | 28451 | 35.557 | 4285 / 6233 | `strict-edit-refused:code.proposal.edit` |
| dev-004 | 2924 | 29812 | 83.368 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-005 | 6702 | 26034 | 70.411 | 6702 / 12887 | `ungraded-proposal` |
| dev-006 | 7930 | 24806 | 87.107 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-007 | 8344 | 24392 | 20.953 | 8344 / 2662 | `strict-edit-refused:code.proposal.edit` |
| dev-008 | 8028 | 24708 | 91.402 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-009 | 2942 | 29794 | 25.962 | 2942 / 3960 | `ungraded-proposal` |
| dev-010 | 3919 | 28817 | 48.343 | 3919 / 9262 | `ungraded-proposal` |
| dev-011 | 5969 | 26767 | 86.261 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-012 | 2938 | 29798 | 80.589 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-013 | 8375 | 24361 | 73.350 | 8375 / 13433 | `strict-edit-refused:code.proposal.edit` |
| dev-014 | 5945 | 26791 | 56.468 | 5945 / 11313 | `ungraded-proposal` |
| dev-015 | 8513 | 24223 | 41.193 | 8513 / 7114 | `ungraded-proposal` |
| dev-016 | 8050 | 24686 | 32.105 | 8050 / 5929 | `strict-edit-refused:code.proposal.format` |
| dev-017 | 3985 | 28751 | 16.558 | 3985 / 2282 | `ungraded-proposal` |
| dev-018 | 3000 | 29736 | 31.390 | 3000 / 6470 | `strict-edit-refused:code.proposal.edit` |
| dev-019 | 8209 | 24527 | 12.116 | 8209 / 760 | `ungraded-proposal` |
| dev-020 | 7360 | 25376 | 17.458 | 7360 / 3187 | `ungraded-proposal` |
| dev-021 | 7608 | 25128 | 88.026 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-022 | 6763 | 25973 | 52.737 | 6763 / 10490 | `strict-edit-refused:code.proposal.format` |
| dev-023 | 7604 | 25132 | 39.513 | 7604 / 6920 | `ungraded-proposal` |
| dev-024 | 5095 | 27641 | 15.410 | 5095 / 2940 | `generation-refused:error:chat.response.invalid` |
| dev-025 | 4104 | 28632 | 85.544 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-026 | 4756 | 27980 | 6.497 | 4756 / 1054 | `ungraded-proposal` |
| dev-027 | 7977 | 24759 | 103.941 | unknown | `generation-refused:error:chat.transport.limit` |
| dev-028 | 8131 | 24605 | 37.960 | 8131 / 6831 | `strict-edit-refused:code.proposal.format` |
| dev-029 | 7905 | 24831 | 28.943 | 7905 / 4436 | `ungraded-proposal` |
| dev-rust-001 | 3373 | 29363 | 55.136 | 3373 / 11567 | `strict-edit-refused:code.proposal.edit` |

The shared archive was restored in empty private state with networking disabled;
all 19,633 manifest-bound files verified. No model, candidate or hidden test ran
during restoration. The final documentation fast gate passes; this does not
change the inherited source-gate failures above.

A final retention check found that the full test build refreshed a legacy API
cache alias used by earlier evidence. Its exact archived bytes were restored.
Both earlier manifests (117,972 and 19,634 files) and the new 19,633-file manifest
verify unchanged. The new measurement used isolated frozen binaries throughout.
The legacy target is retired from future builds; the restoration record and
manifest checks are supplied privately with the review.

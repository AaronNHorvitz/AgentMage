# Retained development timeout diagnosis — 2026-11-12

This audits the largest observed failure in the completed six-model
[development measurement](coding-scoreboard.md#observed-causes-across-the-completed-six-model-campaign):
45 transport timeouts among 180 attempts at product source
`42612ca9787f07e4df79402e6d92f39f578e5c67`. It runs no model, candidate,
test, retry or held-out task, changes no product source or evaluation input,
and reports no new task score. T-303 and T-306 remain open.

The audit verifies the original reviewed manifest and checks 260 retained input
files against its hashes and byte counts. Those inputs include the three
relevant model result files, each timeout's record, request, API events and
backend log, and its enclosing job report. It checks all 45 timeout records,
rather than inferring their behavior from selected examples.

| Model artifact label | Timeouts | No final text | Some final text | Backend stop context-position range |
| --- | --- | --- | --- | --- |
| muse | 2 | 2 | 0 | 18727–20888 |
| gemma4-31b | 22 | 16 | 6 | 13441–19628 |
| qwen35-27b | 21 | 20 | 1 | 15346–20975 |

All 45 API records end with `chat.transport.timeout` at 300.006–300.032 seconds.
Every corresponding backend slice records one cancellation and one stop with
`truncated = 0`. These positions are backend context positions, not measured
output-token counts. The stopped attempts have no unambiguous prompt/output
counter pair; their token usage remains unknown. Each job used the existing
300000/60000 ms total/idle settings and recorded no resource-guard error.

Of the 45 attempts, 38 emit no final text and seven emit some text before the
timeout. This establishes a generation problem with an active backend; it does
not establish reasoning as the cause. The two model profiles responsible for
43 timeouts supply empty template options. The other two timeouts use the
existing medium-strength Muse profile. The next controlled experiment compares
those two empty-option profiles with `enable_thinking=false`, retaining the
same prompts, model/runtime artifacts, decoding, strict checker, 32K context,
output-reservation rule, time/byte limits and safety boundaries. Template
rendering and the resulting reserved output may change and must be measured.
No profile is adopted and no improvement is claimed from this audit.

The shared-resource experiment is prepared separately from sealed evidence.
Its execution status and any new observations belong in a subsequent measurement
entry; preparation and queueing are not evidence of model execution. Run the
complete 30-task development set for each changed profile in resumable two-task
GPU reservations. Native executable trust and missing original grading controls
remain blockers: strict proposals are ungraded, never solved tasks. Do not
reconstruct admission controls or run held-out tasks to investigate this cause.

| Binding | SHA-256 |
| --- | --- |
| Original reviewed retained manifest | `e759f7d7127700b37a68c757efa8935a5027f543025c81ea7f32604a7831269d` |
| Content-free 45-timeout audit result | `7cbd8a74d70e686ed5fb1c5d80608991a52c6027eaa0bf4fe35b0421a9f622b9` |
| Audit script | `49271b5c527e2449677c3c716fa9989e54156c21e4c7de746a42f4afc7b0e073` |
| Inert reproduction archive | `985f6bb5357b419ac0a028d6d96c64af1590675d2c6b2adbffa05e96429cbf50` |
| Network-disabled cold-check log | `e593340d8c2cb85667d3170c8ce62705121218d34f255d672d768f0cdd341f8d` |

The private inert reproduction package retains the audit and every input it
reads, including the unchanged original manifest. The initial cold check exposed
missing context files in the first package; the new package includes them and
reproduces the audit from fresh temporary state with networking disabled. All
260 original inputs verify, and temporary state is removed. Both private package
versions remain intact. Raw prompts, outputs and host
identities stay private; no weights or credentials are needed. Documentation
checks and reproducible audit results do not demonstrate runtime qualification,
functional task improvement, native execution or row closure. The earlier
53 inherited Rust-gate failures are unchanged; this batch makes no fresh Rust
build, full-gate or supply-chain-regeneration claim. Independent review is required.

# T-303/T-306: diagnose the completed original-address repeat

Reporting base: `4422aa7ba4c2797bbf13c82f8d4618c8412d167b`.
Frozen measured source: `7821274ab5caa56573d8b68aa7a2121c71fc4b2b`.

The [completed repeat](t303-original-address-checkpoint180-2026-11-21.md)
records all 30 development tasks on six profiles. The largest broad refusal
category is `code.proposal.edit` (62 attempts). This read-only diagnosis
separates those refusals before further tuning. No new inference, source,
prompt, schema, task, context, model profile, budget or acceptance-rule change
occurs. T-303/T-306 stay open; no row closes. The preceding checkpoint review
was in flight at preparation. Its subsequently received exact-commit PASS is
retained unchanged below; this diagnosis requires its own review.

## First failing condition per attempt

The authenticated frozen native helper reproduces **167 complete-reply checks**,
including three format refusals. The diagnostic classifier follows the Rust
check order on 164 typed replies: resolve the complete batch against the
original file, then check preimages, visibility, overlap, size and net change.
Its acceptance/refusal agrees with the native checker on every typed reply.
These finer labels are diagnostic conditions, not native error codes or grades.

| Diagnostic condition or retained outcome | Attempts |
| --- | ---: |
| Repeated original text | 31 |
| Invalid line order | 20 |
| Incomplete response terminal | 7 |
| Original not fully visible in one excerpt | 5 |
| Transport timeout | 4 |
| Overlapping edits | 3 |
| Format refusal | 3 |
| Length termination | 2 |
| Line range outside the source | 2 |
| Unknown source path | 1 |
| Strict ungraded proposal | 102 |

The six edit-condition categories total the original 62 edit refusals. Three
format refusals and thirteen generation refusals remain separate. Terminal
event labels do not infer output-cap attribution; the completed checkpoint
records backend counters and their coverage separately. All 102 accepted
proposals remain ungraded, including the 69 with a joined-line boundary
reported by that checkpoint. No task success follows from strict acceptance.

**Repeated original text is the largest individual refusal condition: 31/180.**
Of the 31 first-failing repeated fragments, 30 use line ranges and one
uses the preserved exact-text form. Among the line ranges, 29 select one
line and 16 select only whitespace. 27 addressed ranges are fully visible
within one excerpt. These groups overlap. A visible line address does not waive
whole-file uniqueness. No range is widened, normalized, repaired or resubmitted.

Invalid order accounts for 20 attempts: 20 reverse the range and
0 start at zero. 2 are adjacent reversed ranges; 9 starts exceed the
complete original line count. These observations concern submitted numbers,
not intended edits. Whole-batch resolution checks order before bounds, so a
reversed out-of-bounds range is classified as invalid order.

| Profile | Repeated original | Invalid line order | Strict ungraded proposals | Accepted with joined-line boundary |
| --- | ---: | ---: | ---: | ---: |
| gemma4-26b-a4b | 2 | 7 | 18/30 | 11 |
| gemma4-31b | 3 | 0 | 26/30 | 23 |
| gemma4-e4b | 6 | 11 | 11/30 | 4 |
| gpt-oss | 9 | 1 | 8/30 | 4 |
| muse | 5 | 0 | 17/30 | 10 |
| qwen35-27b | 6 | 1 | 22/30 | 17 |

The boundary column is an overlapping subset of accepted proposals: **69/102**,
reported beside refusals as requested by the checkpoint-180 review's low finding.
It comes from the authenticated preceding diagnostic and that independent
review's recount; it is not recomputed by this diagnosis. It counts proposals
with at least one edit removing LF before a nonblank following original line
and replacing it with nonempty text lacking final LF. Adjacent edits are
included without assuming the following original line survives. It is a textual
observation, not a syntax or task grade, and is not added to the refusal totals.
This larger ungraded pattern must inform the next edit-contract experiment
alongside repeated originals; refusal counts alone cannot rank all edit defects.

First-failure counts are not a list of every defect or independent task
outcomes. They do not establish how many tasks an improvement would solve.
No model admission, role adoption or profile comparison follows.

## Reproduction and limits

The capsule authenticates **1090 consumed files** against the unchanged
originating manifest (119284 entries), with byte lengths and SHA-256 values.
It retains all records, available answers/checks, contexts, terminal events,
batch indexes and resource reports, 30 original source payloads, preflight,
frozen input index and original audit. Every source payload also matches its
exact base Git blob. All 30 contexts reproduce through the authenticated
helper; excerpts match exact original UTF-8 byte ranges. Record identities,
available log digests, zero retries and absence of grading are checked.
All 90 retained job reports record completion without a resource-guard error
and a sampled GPU peak within the unchanged 22,528 MiB ceiling. This does not
re-execute jobs or reverify the complete predecessor inventory; its review
remains separate. The original manifest and postcommit binding are retained.

The unchanged 38 synthetic controls cover LF-only line counting, CRLF,
Unicode, no phantom final line, invalid/out-of-bounds ranges, mixed edit forms,
complete batch resolution, repeated/overlapping originals, visibility,
unchanged edits, replacement NUL, size and net change. Four controls cover
repeated-range observations. Custom-excerpt controls and the oversized-answer
size branch are classifier-only; other edit-condition controls compare with
the native helper. The previously reviewed analyzer and exact adaptation diff
are retained. Adaptation changes campaign identities, expected reply counts
and resource-report checks, and retains the prior joined-line diagnostic;
its classifier and controls remain unchanged. Initial and final attempt rows
and classifications are byte-identical.
Reproduction requires byte-identical analysis JSON.

No task loader, candidate tree, project test, hidden test or held-out input is
read or executed. Proposed code remains data. Only the authenticated effect-free
helper runs; no model, API server or GPU process starts. The analyzer emits
counts, identifiers, line numbers, lengths and digests, not replacement source.
Native executable admission and original execution/grading controls remain
unavailable and are not reconstructed. First-attempt and within-budget task
passes, test feedback and retries remain unavailable.

Originating manifest SHA-256:
`6c9c32983ef23487cf0d425db9c29a85000f6412ee1f5121c1683e47c4d140d9`.
Analysis SHA-256:
`715ae41effc8b997c98199b7fa188643c78ee0da0607aae3b3a1dddc9102e6dd`.
Analyzer SHA-256:
`47748bd4c11a08c5b9c2e8179de5193fa9fbaa75b7ab8532a2343d1b073084cb`.
Consumed-input origin index SHA-256:
`11ada1f93af15198d092d6597c8ea312a1886078eb40af2422dda1bca5fc043c`.

Reporting fast gate, direct Markdown lint/render, privacy, licence,
preservation and portable diagnosis reproduction must pass before requesting
independent core review. All 512 ledger table lines, 49 historical path blobs,
README, historical inventory, Decision 0051, status model and supply-chain
outputs remain unchanged; earlier scoreboard bytes remain an exact prefix.
The retained preceding full source gate remains **FAIL: 1,045 passed,
53 inherited failures, 11 ignored, zero filtered, 31 targets**. It is not rerun
or waived by this reporting change. No SBOM regeneration, long gate, acceptance,
release, owner-test-ready or Git push occurs.

## Next work selected by MEASURE FIRST

Improve selection of unique visible original ranges. Use the repeated-range
observations to choose one experiment; a whole-file unique preimage, exact
source identity, visibility and all-or-nothing refusal remain mandatory.
The earlier insertion guidance has not removed reversed ranges; those remain
counted. This diagnosis establishes a direction, not a measured improvement.
A model-visible change needs tests, frozen contexts and a complete development
repeat before further tuning or capability work. Do not add a limit, deadline
or bookkeeping increment for these failures. Native execution and grading
blockers remain explicit; accepted-edit counts cannot replace task outcomes.

Existing T-309 lows remain backlog: timeout backend cancellation, per-pair
wrapper provenance, campaign-policy/request wording and frozen helper build
source. The preceding checkpoint includes the portable joined-line counts;
this diagnosis does not change candidate bytes or their checker.
Independent core review is required for this increment; no row closes.

The [preceding independent core review](../reviews/4422aa7ba4c2-original-address-development-checkpoint180.txt)
returns **Verdict: PASS** for exact
`4422aa7ba4c2797bbf13c82f8d4618c8412d167b`, with no open high or medium
finding. It is retained unchanged. It accepts the preceding measurement;
this diagnosis needs its own review and closes no row.

Inherited joined-line diagnostic SHA-256:
`59c3c009fde633825a79855e82e9fb0d7282bea6ef347b1907bead160ef41f64`.

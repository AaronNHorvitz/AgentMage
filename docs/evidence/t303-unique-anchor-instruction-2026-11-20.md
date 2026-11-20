# T-303/T-306: explain unique anchors before the next repeat

Base: `1906b95ddbdf359eff5fc21ffb05f72c051dbd30`.
[Decision 0215](../decisions/0215-unique-anchor-generation-guidance.md) targets
21 repeated-original refusals, the largest individual condition in the completed
six-profile line-range repeat. This is preparation for a new experiment, with
**no model inference or demonstrated improvement**. T-303 and T-306 remain open;
independent core review is required.

## Changed variable

Only the fixed generation instruction changes in production source: **1,061 to
1,039 UTF-8 bytes**. It explains why blank or brace-only lines may need neighboring
visible code for a unique preimage, and tells the model to preserve the added
context and insertion anchor in the replacement. It explicitly orders inclusive
line bounds and explains the final LF before following source. The same
instruction applies to the existing API and effect-free helper.

The entire production source file outside that string is byte-identical to the
base. Parser, strict checker, retrieval algorithm, version 2 source framing,
output schema, permissions and every limit remain unchanged. No ambiguous range
is widened and no replacement newline is repaired. Existing exact-text clients
remain valid. The new regression exercises insertion beside repeated blank lines
under LF and CRLF, with exact retained context, refusal of the blank-only anchor
and literal handling of a missing replacement line ending. Existing tests and
minimum budgets are unchanged.

## Development preflight

All **30** development payloads match their frozen historical source Git blobs.
The preflight authenticates **987 complete consumed files** against the preceding
diagnosis manifest. It reproduces all **162** recorded complete-reply checks with
the frozen helper. Checking the same answers under the new contexts produces
**zero differences**. This comparison is a context regression, not a new model
run, repaired answer or task grade. All 109 prior strict proposals stay ungraded.
The prior complete million-file campaign inventory is not rerun or claimed
verified in full by this preparation.

Selected excerpts change on **10** tasks: dev-002, dev-006, dev-007, dev-008, dev-013, dev-015, dev-016, dev-019, dev-027, dev-028.
The algorithm and allowances are unchanged, but the shorter instruction changes
the available allowance. Every delivered body is checked against its complete
original UTF-8 source range; byte lengths, framing and original line bounds
are independently counted. Message content sizes are **11,000–32,627 bytes**.
These are not rendered model tokens; token fit remains unmeasured.
The schema and six profile files are retained unchanged with their original
manifest bindings. The changed experimental variable is the fixed wording and
its resulting budget-dependent excerpts.

Preflight SHA-256: `4009b824c7953fbf30e7110e47546cc71e1562e612ef47cb39e1e9ab853b0272`.
Method SHA-256: `4ec35a3f240cd076b4abe80e8741394c1277cd88f5cec2ea64d1f74c7a924b5a`.
Checker preservation SHA-256: `dd3befa6dbcd71239c44a62eb9132777315a1f9b2d1f66ee548eccfa632eb026`.

## Prior replacement line endings: diagnostic observation

The checkpoint180 review requested this observation without changing its frozen
checker or results. Among the **109** prior strict ungraded proposals, **64**
contain at least one nonempty line-range replacement before the final source
line whose `new` does not end in LF: **130** edits in total.
All 11 exact-text edits are excluded from this line-range observation. Counts
are over proposals and edits, not independent tasks or syntax failures; the
observation does not establish whether joining or removing a line was intended.
No proposed source is executed and no prior result is reclassified.

| Profile | Proposals with this condition | Line-range edits with this condition |
| --- | ---: | ---: |
| gemma4-26b-a4b | 14 | 21 |
| gemma4-31b | 19 | 41 |
| gemma4-e4b | 6 | 6 |
| gpt-oss | 5 | 14 |
| muse | 6 | 15 |
| qwen35-27b | 14 | 33 |

The first diagnostic draft incorrectly assumed every accepted edit used line
ranges; the corrected reporter handles the already-supported exact-text form.
The initial script and failure log are retained. No product source or answer
was changed in response to that diagnostic failure.

The preceding diagnosis review's low about output-cap attribution is retained:
its cap explanation comes from the checkpoint180 report's backend counters,
not from the incomplete-response event alone. This preparation does not consume
or remeasure those backend counters. Earlier accepted lows remain T-309 backlog.

## Verification and next measurement

Formatting and Clippy pass. The full affected-crate gate remains **FAIL**:
**1,044 passed, 53 inherited failures, 11 ignored, zero filtered, 31 targets**.
Every failing test identity matches the previous source gate; there are no new
or absent failures. No test is skipped or allowlisted to obtain a pass.
Direct Markdown lint/render, review-format, privacy, licence and preservation
checks must pass before the request. Both newly available independent PASS
reports are retained unchanged; neither closes a parent row.

All 512 ledger table lines, 49 historical path blobs, README, historical planning
inventory, Decision 0051, status model and supply-chain outputs remain unchanged.
Earlier scoreboard bytes remain an exact prefix. There is no SBOM regeneration,
long acceptance gate, release, owner-test-ready or push.

Freeze this source and all 30 contexts, then repeat the full development set on
all six profiles in resumable GPU reservations before any more tuning or
capability work. Keep 32K context, the per-task output formula, thinking request,
profile settings and deadlines. Report actual token fit, refusals, strict
ungraded proposals, time and tokens. No held-out input is used here or for tuning.

Native executable admission and original grading controls remain unavailable;
they are not reconstructed. No candidate, project test, test feedback or retry
runs. First-attempt and within-budget task passes remain unavailable. No model
is admitted, qualified or selected for a role, and no row closes.

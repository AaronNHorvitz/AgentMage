# Exact-edit refusal diagnosis — 2026-11-17 UTC

The completed six-profile inline-schema repeat contains **180 development
attempts: 106 strict ungraded proposals, 56 edit refusals, four format refusals
and 14 generation refusals**. Edit refusal is the largest recorded refusal
cause. First-attempt and within-budget task passes remain **unavailable**.
This is read-only diagnosis of existing runs, not another inference campaign
or a task-quality result. T-303/T-306 remain open; independent core review
is required.

The preceding [independent Muse completion review](../reviews/5333ea1b3581-inline-schema-muse-development-completion.txt)
has **Verdict: PASS** for exact
`5333ea1b3581a0e89eb67e18bd4b33bc28606148`, with no findings at any severity.
It is retained byte for byte, accepts that measurement only and closes no row.
It does not review this diagnosis.

All runs measured frozen source
`72c002a42957340de0b6fa0eb4d48385db2c7e62`. Their existing scoreboard entries
retain their actual budgets, tokens, time and resource accounting. This
diagnosis changes no measured variable and uses no additional GPU reservation.
The available path still ends at the strict proposal checker. Native executable
admission and original grading controls remain unavailable; full repository
retrieval, native effects, project tests, feedback and retries cannot run here.

## First failed edit condition

Classify each refused answer once, following the unchanged Rust checker's
order: process edits in array order; check the path, empty original fragment,
unchanged replacement, NUL, absent original fragment, ambiguity and visibility;
only after all edits pass those checks examine overlap and final size.
UTF-8 byte offsets and overlapping occurrences are respected. A later problem
does not replace an earlier reason.

| Model profile | Attempts | Ungraded proposals | Edit refusals | Format refusals | Generation refusals |
| --- | --- | --- | --- | --- | --- |
| gemma4-26b-a4b | 30 | 19 | 10 | 0 | 1 |
| gemma4-e4b | 30 | 17 | 13 | 0 | 0 |
| gemma4-31b | 30 | 17 | 13 | 0 | 0 |
| qwen35-27b | 30 | 28 | 2 | 0 | 0 |
| gpt-oss | 30 | 6 | 15 | 0 | 9 |
| muse | 30 | 19 | 3 | 4 | 4 |
| Total | 180 | 106 | 56 | 4 | 14 |

The 14 generation refusals comprise two length terminations, eight incomplete
responses and four transport timeouts, as recorded in their terminal events.

| First failed condition | Answers |
| --- | --- |
| Original fragment is absent from the complete original file | 25 |
| Original-source edit ranges overlap | 11 |
| Original fragment occurs more than once, including overlapping matches | 8 |
| Original fragment is empty | 5 |
| Replacement equals its original fragment | 5 |
| Original fragment exists but is outside a supplied excerpt | 2 |
| Total | 56 |

These are mechanical refusals, not diagnoses of whether the intended changes
would solve the tasks. The checker correctly refuses them. No answer is
repaired, an absent fragment is never accepted through normalization, and
no generated program or candidate test executes.

Seven of the 25 absent-fragment answers have a match only after removing
whitespace from both the fragment and original file. One absent fragment
occurs verbatim in an earlier proposed replacement. These are diagnostic
observations, not accepted matching rules or proof of the model's intent.
They are counted only at the first failed edit; later defects are not added
as extra failed answers.

## What the next change must address

Copying exact original source is the largest identified edit problem. The
model must preserve the decoded text, including whitespace, quotation marks
and newlines, and propose disjoint changes against that original text. Some
answers instead supply different text, overlapping changes, an empty insertion
anchor, an unchanged edit or too little context to distinguish repeated text.

The next T-303/T-306 generation work should address original-fragment fidelity
and then repeat the complete development set on all available profiles. It
must preserve exact preimage matching, unique visible ranges, all-or-nothing
refusal and the unchanged checker limits. This report selects the problem;
it does not establish that a particular prompt or representation solves it.
Another timeout, cancellation or bookkeeping increment is not selected by
these numbers. No Git or service-action tool starts while task-completion
reliability remains unavailable. The held-out set is not used for diagnosis
or choosing a change.

## Reproduction and preservation

The private review capsule retains the diagnostic method, authenticated input
bindings, each development record/context/answer/check result, complete
editable-source preimages and content-free per-answer classifications. Its
method authenticates every file in the six original completed capsules, their
pinned manifests and postcommit receipts, and checks the entire committed diff
against each receipt. Earlier capsules and review reports are unchanged.
Whole-capsule authentication hashes retained files; only development records,
contexts, answers and editable source are interpreted for the diagnosis. No
hidden or held-out test content is used as diagnostic input.

It reads development-only source blobs at their frozen base commits and checks
each original-source digest and excerpt byte range. All six profiles have the
same 30 context bindings. It executes only the authenticated original Rust
context/check helper on saved text, reproducing the original context and
checker result for all **166 saved answers**; the other 14 attempts have no
answer to check. All **16 synthetic diagnostic checks pass**, covering
ordering, absent/ambiguous/invisible text,
UTF-8 ranges, overlap, adjacency, size, net unchanged edits and flags that
cannot change a refusal. It imports no archived Python or generated candidate.
These are diagnostic checks, not task grades.

The earlier scoreboard remains an exact byte prefix. Existing ledger table
lines, historical private-path files, inventory text, Decision 0051, status
model, runtime source and frozen evaluation inputs are preserved. The only new
ledger rows record the owner's previously requested later service-action plan;
they start unchecked and unreleased. No row closes or status is promoted.

The reporting fast gate, Markdown checks, privacy/licence checks and exact
preservation checks cover this documentation batch. The full source/acceptance
gate is not rerun and supply-chain outputs are not regenerated. The inherited
source result remains **FAIL: 1,169 passed / 53 failed / 19 ignored**, as recorded
in the [source review](../reviews/ac1c30201171-trusted-inline-output-schema.txt).
The licence batch's two inherited Python failures and supply-chain limits
remain in its [evidence](readme-and-dual-licence-2026-11-16.md). No model-role,
native runtime, platform, release or owner-readiness qualification is claimed.

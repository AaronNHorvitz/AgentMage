# T-303/T-306: diagnosis of the original-fragment repeat

Reporting date: 2026-11-18 local / 2026-11-19 UTC.
Base: `fda187da1b42d26405c4dc0ce50f8353b30c2f72`.
Frozen measured source: `35217c60a56a5d3e065148eb9a6aea898b5e1b16`.

The completed [six-profile repeat](t303-copy-original-fragments-e4b-complete-2026-11-19.md)
contains 180 attempts: 179 completed and one preserved resource interruption.
Its largest failure cause is `strict-edit-refused:code.proposal.edit`, with
50 refused proposals. This read-only diagnosis compares those proposals with
their original supplied source and visible excerpts. It performs no inference,
candidate execution, project tests, retries or grading. The 107 strict ungraded
proposals remain ungraded; first-attempt and within-budget task passes remain
unavailable. No model role qualifies and no row closes.

## First strict-edit failure

Each proposal counts once, at its first failing condition in the unchanged
Rust checker's order. Subsequent errors may also exist.

| Profile | Absent original | Overlap | Repeated original | Unchanged edit | Outside visible excerpt | Empty original | Total |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A4B | 3 | 4 | 0 | 3 | 2 | 0 | 12 |
| 31B | 2 | 1 | 2 | 1 | 1 | 1 | 8 |
| E4B | 11 | 0 | 0 | 1 | 0 | 0 | 12 |
| gpt-oss | 12 | 0 | 2 | 0 | 0 | 0 | 14 |
| Muse | 0 | 1 | 1 | 0 | 0 | 0 | 2 |
| 27B | 0 | 1 | 0 | 0 | 1 | 0 | 2 |
| Total | 28 | 7 | 5 | 5 | 4 | 1 | 50 |

An absent original means the decoded `old` string does not occur in the
complete frozen source. Overlap means individually valid original ranges
share source characters. A repeated original has multiple occurrences,
including overlapping occurrences. Visibility requires the entire original
range to lie within one supplied excerpt; source outside it grants no context.

Among the 28 absent originals, seven match somewhere in the complete source
after removing whitespace for observation only; one occurs in an earlier
proposed `new`; and 16 contain a literal newline or quote escape spelling.
These flags overlap and do not establish the cause of every refusal.

In seven absent-original cases, diagnostically substituting literal `\n`
with a newline and `\"` with a quote yields text found in a supplied excerpt.
Six are gpt-oss cases and one is E4B. This is a narrow representation
observation, not a general JSON decoder, uniqueness check or accepted repair.
No transformed string is supplied to the checker, emitted as a candidate or
executed. Other edits may still fail, and task correctness is unknown.

## What the instruction repeat establishes

[Decision 0212](../decisions/0212-copy-original-fragments-before-replacement.md)
changed the fixed instruction to ask for copying the original before deriving
the replacement. The preceding repeat had 106 strict ungraded proposals in
180 attempts; this repeat has 107 in 179 completed attempts plus one resource
interruption. The interrupted task previously produced an ungraded proposal.
Restricting comparison to the same 179 completed profile/task pairs gives:

| Profile | Paired attempts | Previous ungraded proposals | Current ungraded proposals | Became a proposal | Ceased to be a proposal |
| --- | --- | --- | --- | --- | --- |
| A4B | 30 | 19 | 17 | 3 | 5 |
| 31B | 30 | 17 | 22 | 9 | 4 |
| E4B | 29 | 16 | 11 | 3 | 8 |
| gpt-oss | 30 | 6 | 8 | 5 | 3 |
| Muse | 30 | 19 | 22 | 6 | 3 |
| 27B | 30 | 28 | 27 | 1 | 2 |
| Total | 179 | 105 | 107 | 27 | 25 |

These are mechanical proposal transitions, not solved tasks or role-selection
scores. A single repeat with changes in both directions does not establish a
reliable improvement from the instruction. Absent-original failures rose from
25 to 28, despite total strict-edit refusals falling from 56 to 50.

Under MEASURE FIRST, the next model-facing work targets source-copy fidelity,
starting with the source text's presentation and escape layer. Further prose
instructions alone are not supported as an established remedy by this repeat.
A presentation change must preserve exact original bytes, visible ranges,
untrusted-data boundaries and all-or-nothing checking, then freeze and measure
all 30 development tasks on all six profiles. No such change ships here.
Git remains behind measured reliability; services, flows and team mode stay
later work. No new row, limit, cancellation mechanism or bookkeeping feature
is introduced.

## Reproduction and limits

The private review capsule retains the diagnostic method, 30 Git-bound source
snapshots, the 180 recorded contexts, requests, events and available answers
and checks, their original manifest entries, and all 25 provenance indexes.
No task loader, hidden-test module, archived Python module or candidate code
is imported or executed. The interrupted record remains separate from completed
inference; its failed job and partial next task remain unchanged.

The original effect-free Rust helper is authenticated to
`1992c3c40cd00f5b460585fe3934e661bf8f6004f1c77ea7c27635309c3bd6de`.
Reproduction checks all 180 contexts and the 161 available strict-check results,
including format refusals, against that helper. Twenty-two synthetic diagnostic
checks exercise classification and representation flags. These checks assess
the diagnostic method, not task success or native execution admission.

The frozen model settings, prompts, schema, context and output budgets, raw
attempts and counters remain unchanged. This diagnosis adds zero inference
attempts and zero GPU minutes. Native executable admission and the original
execution/grading controls remain unavailable; effects, project tests, feedback
and retries cannot run. Hidden and held-out tasks are neither loaded nor used.

Reporting fast gate, Markdown lint/render, privacy, licence, input authentication
and preservation checks cover this documentation batch. Source tests are not
repeated because source is unchanged. The last source gate remains FAIL:
1030 passed, 53 inherited failures and 11 ignored, with two Python failures and
supply-chain limits disclosed in the preceding evidence. All 512 ledger table
lines, 49 historical path blobs, inventory, Decision 0051, status fields and
previous scoreboard bytes are preserved. No SBOM regeneration or acceptance
gate runs.

At session start the preceding exact-commit review had no available verdict.
Its queued request is retained privately; no approval is inferred. This
diagnosis and the preceding pending completion require independent core review.
T-303 and T-306 remain open.

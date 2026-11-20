# T-303/T-306: diagnose the completed raw-source repeat

Reporting base: `26334be8cd5b176d8435112db2fcce8e614facaf`.
Frozen measured source: `eef17dbdc8e2e3aeaa95b5a2684cb2c47cc0767f`.

MEASURE FIRST selects the 36 first-failing absent-original refusals from the
[completed repeat](t303-raw-source-checkpoint180-2026-11-20.md).
This read-only investigation changes no product source, prompt, task, model
profile, checker, budget or scoreboard result. No attempt is rerun. T-303 and
T-306 remain open; independent core review is required.

## Observations

The retained classifier reproduces all **161 typed edit-condition results**:
103 valid ungraded proposals and 58 edit-condition refusals, including the 36
absent originals. The other 19 classifications (15 generation refusals and four
format refusals) are retained, rather than independently rederived here.
The unit of the following analysis is the **first failing original fragment**
in each of those 36 attempts, not every edit in every answer.

| Observation on the decoded original fragment | Attempts |
| --- | ---: |
| Erasing all Unicode whitespace from both the fragment and one visible excerpt gives a substring match | 12 |
| Replacing literal newline and quote escapes in the fragment gives a visible substring match | 1 |
| Collapsing pairs of backslashes in the fragment gives a visible substring match | 1 |
| None of those, backslash doubling or spelling nonprinting controls gives a visible match | 22 |

The three matching groups happen to be disjoint in this repeat. Each comparison
uses a single original visible excerpt; separate excerpts are never joined.
These are observations, **not valid edits or task passes**. Whitespace erasure
can change string values, substring matches need not be unique, and a match
says nothing about the remaining edits. The original checker still refuses
every one of these 36 answers. No transformed fragment is sent to it.

Of the 22 without a visible transformation match, 19 have at least one nonblank
line that is absent from the full source even after stripping outer whitespace.
In the other three, every nonblank line individually occurs after that stripping;
this does not establish a contiguous original block or correct line order.
These line observations do not prove why the model emitted its answer.

| Profile | Absent-original attempts | Any specified visible transformation match | First failing fragment at least 1,024 bytes |
| --- | ---: | ---: | ---: |
| gemma4-31b | 3 | 1 | 0 |
| qwen35-27b | 1 | 1 | 0 |
| gpt-oss | 11 | 6 | 2 |
| muse | 1 | 0 | 0 |
| gemma4-e4b | 15 | 3 | 10 |
| gemma4-26b-a4b | 5 | 3 | 0 |
| Total | 36 | 14 | 12 |

The absent fragments range from 24 to 13,854 bytes, with median 453.5 bytes.
Two exceed the byte length of every individual visible excerpt. One is contained
in an earlier edit's proposed replacement; it remains absent from the original.
For comparison, the maximum original-fragment length within each of the 103
valid proposals ranges from 11 to 3,365 bytes, with median 297 bytes; 22 valid
proposals contain a fragment of at least 1,024 bytes. These are different
statistics (first failing fragment versus per-proposal maximum), not a causal
comparison. The 1,024-byte reporting bin is not an execution or admission limit.

## Method and verification

The analysis consumes complete original records, contexts, answers and checks
from all 180 attempts, the full 30 development source files at their frozen Git
commits, the entire previous diagnosis and preflight, and the retained classifier.
Every consumed file is checked for its full SHA-256 and length against the
unchanged original manifest or the frozen source binding. Source excerpts are
checked against exact UTF-8 byte ranges. Record log digests, task/model identities,
zero retries and the absence of grading/native execution are checked again.

The exact original manifest is retained whole, with SHA-256
`fb58e55cd32fa533e2f9f4ab55de83cd2b418572fcf9b3c0680ef0c1953d86e3`.
The original 1,995,948-file capsule and its correction supplement stay unchanged.
This new analysis authenticates its 723 consumed input files; it does not replace
or claim to rerun the complete predecessor inventory verification. Its independent
review remains in flight at reporting preparation.

Only the four explicitly named classifier/control functions are compiled from
the authenticated diagnostic source. Its archived main is not imported or run.
Proposed code is treated as text; no candidate, task loader, hidden test or
held-out test executes. No candidate tree is read. The analyzer emits counts,
identifiers, lengths and digests, without reconstructed source or repaired answers.

The 22 retained classifier controls and 16 new diagnostic controls pass. New
controls cover whitespace within strings, escaping, Unicode byte lengths,
invisible text, separate excerpt boundaries, earlier replacement containment,
line reordering, missing lines, refusal of empty/exact originals and the
content-free output fields. Reproduction yields the same complete analysis JSON.

Retained analysis SHA-256:
`0e0b0237a4ded526bdbcb2243e1a51c7b27fc28a650fc81a2260bb6278270ba1`.
Analyzer SHA-256:
`3472ba070942664b92c03e5dbb51ee31ddd31b81c1c15e5ca8d93e2c3e56ff16`.
Input-origin index SHA-256:
`12bd2883daafe640aab263ef58ce67958bcf8b20397f0bf153258cefadb6615d`.
The independent-review bundle retains exact inputs, provenance, scripts,
controls and verification logs, with its own complete inventory binding.

The reporting-only fast gate, Markdown lint/render, licence, privacy and
preservation checks pass. All 512 ledger table lines, 49 historical path blobs,
the original inventory, Decision 0051, status model, README and scoreboard remain
unchanged. The inherited source-gate failure remains disclosed in the completed
repeat; no source suite, SBOM, long acceptance gate or release is rerun here.

## Consequence for the next work unit

Original-source identification remains the largest measured refusal cause.
The next experiment should reduce source recopying while retaining exact,
unique, visible preimages and all-or-nothing refusal. This diagnosis supplies
no evidence for whitespace repair, larger budgets or a new fragment-size cap.
Any change to model-visible input or edit interpretation needs its own tests,
freeze and full development repeat under MEASURE FIRST before further tuning.

Native executable admission and the original execution/grading controls remain
unavailable. Task passes, test feedback and retries remain unavailable; the
103 strict proposals are still ungraded. No model role is selected, no capability
is admitted and no row closes.

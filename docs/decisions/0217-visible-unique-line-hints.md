# Decision 0217: Identify unique visible anchor lines before generation

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-21 |
| Authority | MEASURE FIRST; T-303/T-306 |
| Scope | Model-visible source metadata and instruction; no execution authority |

The completed original-address development repeat records 31 repeated-original
refusals, the largest individual refusal condition. Thirty use line ranges;
29 select one line and 16 select whitespace. The preceding instruction already
explains uniqueness. Supply concrete anchor information instead of asking the
model to infer whole-file uniqueness from incomplete excerpts.

Version 3 source headers add `unique_lines`: compressed inclusive intervals
of original line numbers. Every line within an interval is individually unique
as an exact substring of the complete bound source and contains non-whitespace
text. Occurrences inside another line and overlapping occurrences count.
LF defines lines; CR remains part of the text. An unterminated final line is
checked with its exact bytes. Only visible lines appear in a header. Adjacent
unique lines share one interval; an empty list makes no claim about multi-line
ranges. Compute uniqueness once per source with repeated-line results shared.

The fixed instruction explains these hints and retaining an anchor when editing
repeated blank or brace lines. It also gives an explicit escaped-LF replacement
example: the completed repeat has 69 accepted proposals with joined-line
boundaries, an overlapping textual observation rather than a correctness grade.
No newline is added by the runtime. This supersedes Decision 0216's generation
wording and extends Decision 0214's header metadata; both remain unchanged.

Hints are advisory source metadata, never grants or additional accepted edit
forms. Other unique visible ranges remain valid. Every submitted range still
passes the unchanged strict checker, including complete-source identity,
whole-file uniqueness, visibility, size, overlap and all-or-nothing refusal.
No answer, range or replacement is repaired. Source bodies, output schema,
retrieval ranking, selection allowance, permissions and limits stay unchanged.
Charge every header byte against the existing actual-message budget; added
metadata can change selected excerpts. Record those changes in the preflight.

Test hint accuracy against an independent byte-window oracle and test anchored
insertions, missing LF, omitted repeats, Unicode, CRLF and tight budgets. Freeze
all 30 development contexts and compare prior replies without executing their
code. Then repeat all 30 tasks on all six profiles in resumable GPU jobs before
further tuning or capability work. Keep 32K context, the per-task output formula,
profiles, thinking requests and deadlines. This is an unmeasured hypothesis.

Native executable admission and original grading controls remain unavailable;
do not reconstruct authority or claim task passes, feedback or retries. No
hidden or held-out input is used. Independent core review is required;
T-303/T-306 stay open and no row closes.

# Decision 0216: Explain nonempty original addresses for insertions

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-21 |
| Authority | MEASURE FIRST; T-303/T-306 |
| Scope | Fixed generation instruction; no new edit or execution authority |

The completed unique-anchor repeat records 17 invalid-line-order refusals,
the largest individual refusal condition. All 17 reverse the range; seven
are adjacent reversed ranges and nine start beyond the original source.
These observations describe submitted numbers, without inferring intended edits.

Change the fixed generation instruction to emphasize nonempty original ranges.
Explain insertion before and after an existing line using the same selected
anchor, with a concrete line-number example. Explicitly prohibit reversed
insertion ranges and addresses for newly generated lines or beyond the source.
Continue counting from the excerpt's original start line. Preserve guidance on
whole-file uniqueness, retained visible context and replacement line endings.

The instruction remains within its previous byte allowance. The parser, strict
checker, retrieval algorithm, framing, output schema, permissions and limits
stay unchanged. No range is swapped, extended or repaired by the runtime.
Existing exact-text clients remain supported. This supersedes only Decision
0215's generation wording; the original-line contract of Decision 0214 and all
prior frozen results remain unchanged. This is an unmeasured hypothesis.

Test insertion anchors at the beginning, middle and end of a synthetic source,
including LF, CRLF and an unterminated final line. Reversed or nonexistent
anchors still refuse the complete batch. Freeze all 30 development contexts
and record excerpt changes and prior-answer check differences. Then measure
every development task on all six existing profiles in resumable GPU jobs
before more tuning or capability work, keeping 32K context, per-task output
budgets, profiles, thinking requests and deadlines unchanged.

Native executable admission and original grading controls remain unavailable;
do not reconstruct authority or claim task passes, feedback or retries. No
candidate, project test, hidden or held-out input is used in preparation.
Independent core review is required; T-303 and T-306 stay open.

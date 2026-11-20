# Decision 0214: Resolve edit preimages from original line ranges

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-20 |
| Authority | MEASURE FIRST; T-303/T-306 |
| Scope | Coding proposal representation; no execution authority |

The completed six-profile raw-source repeat has 36 absent-original refusals,
the largest measured refusal cause. Its read-only diagnosis finds 14 visible
matches after specified diagnostic transformations and 22 without one. These
observations select original-source identification for the next experiment;
they do not justify normalization, repaired output or a new fragment-size cap.

Add the strict edit form `path/start_line/end_line/new`. Numbers are positive,
1-based original-file line numbers, inclusive at both ends. LF ends a line;
CR before LF remains part of that line. A final line without LF is selectable;
a final LF does not create an empty selectable line. Insertions retain an
existing anchor line; deletions use an empty replacement. The runtime obtains
the original bytes from the already-bound source snapshot. The model supplies
the replacement, never an execution grant or new source authority.

Derived preimages go through the same exact-text checker as existing
`path/old/new` clients: unique in the complete original file, fully visible in
one excerpt, nonempty, effective and nonoverlapping. An address does not make
a repeated preimage acceptable. Mixed representations within one edit, unknown
or duplicate keys, positional arrays, nulls and noninteger line numbers are
refused. A batch may combine independently valid edits of both forms, but all
edits still address the original snapshot and all succeed or all are refused.
Whole-source identity, answer/edit/file/aggregate size bounds and later exact
permissions remain in force. There is no fuzzy matching or answer repair.

Raw source bodies remain unchanged. Version 2 excerpt headers add original
`start_line` and `end_line`; framing still follows UTF-8 byte counts rather
than searches for delimiters. The fixed instruction prefers line ranges and
permits exact-text edits when a configured schema requires them. Existing
exact-text clients remain supported. The retrieval algorithm and existing
JSON-sized selection allowance remain, with both that allowance and the actual
framed message charged for the current instruction. Changed overhead may
change excerpts; record every change in the development preflight.

This supersedes Decision 0212's output-construction instruction and extends
Decision 0213's source headers. Both records and their measured results remain
unchanged. The changed experimental variable is the complete line-reference
protocol: instruction, headers, strict parser and trusted inline output schema.
This is a generation hypothesis, with no measured improvement yet.

Freeze the source, schema and all 30 development inputs before inference. Run
each task on all six existing profiles in resumable GPU jobs before further
tuning or capability work. Keep the 32K context, per-task output formula,
profiles, thinking request and deadlines. Report changed excerpts, does-not-fit
cases, strict ungraded proposals, refusals, tokens and time. Hidden and held-out
tasks are not used. Native executable admission and original execution/grading
controls remain unavailable: do not reconstruct them or claim task passes,
feedback or retries. Independent core review is required; T-303/T-306 stay open.

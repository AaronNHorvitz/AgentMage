# Decision 0213: Present exact source as raw UTF-8

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-19 |
| Authority | MEASURE FIRST; T-303/T-306 |
| Scope | Coding proposal source presentation |

The completed original-fragment repeat has 50 strict-edit refusals. Its
read-only diagnosis finds 28 absent originals; seven have a visible-source
match only after diagnostic newline/quote substitution. The previous fixed
instruction changed paired strict ungraded proposals from 105 to 107, with
27 gains and 25 losses. That does not establish a reliable improvement.

Present selected source directly as UTF-8 in the coding proposal's user
message. A single JSON metadata line carries the task, whole-file outlines,
encoding version and excerpt count. For each excerpt, a JSON header carries
its path, original byte range and body byte count, followed by a newline,
exact source bytes and one framing newline. The framing newline is not source.
Newlines, quotes, tabs, backslashes and CRLF in source are not JSON-escaped
inside that message. The transport still encodes the entire message normally.
Body boundaries follow byte counts, never sentinel searches; header-like text
inside source remains source. All task, source and feedback content stays in
the user role as untrusted data and grants no instruction or authority.

The fixed instruction, strict output schema, exact unique visible original
checks, all-or-nothing refusal, source validation and permissions are unchanged.
No normalization, output repair, execution grant or grading control is added.
The selector retains its candidate order, range rules and existing JSON-sized
selection allowance. Actual raw framed bytes must independently fit the same
2–32 KiB ceiling. Both checks include the fixed instruction. Record any excerpt
changes in development preflight. Repair turns prepend JSON-labelled untrusted
feedback to the same raw source representation; they do not escape source
again or treat diagnostics as a successful check.

This is a generation hypothesis. Freeze the source and repeat all 30 development
tasks on all six available profiles in resumable resource jobs before further
tuning or capability work. Keep 32K served context, the per-task output formula,
profiles, inline schema, thinking request and deadlines unchanged. Publish
proposals, refusal causes, does-not-fit cases, tokens and timing as observed.
Native execution and original grading controls remain unavailable; do not
reconstruct them. Task passes and retries remain unavailable. Hidden and
held-out tasks are not used. Independent core review is required; T-303/T-306
remain open.

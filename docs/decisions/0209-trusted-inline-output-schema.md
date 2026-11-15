# Decision 0209: Trusted inline output schema

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-15 |
| Authority | MEASURE FIRST; T-303/T-306 |
| Scope | Credential-owning model transport and trusted API settings |

The six fresh development measurements at source
`a896f055256e96baec09966c71c5d0a7f1714d96` contain 27 strict format refusals
in 180 attempts, the largest measured refusal cause. Read-only inspection of
those development answers finds 11 with duplicate fields, 11 with the wrong
structure, four malformed JSON answers and one answer exceeding the unchanged
64 KiB checker ceiling. Classification checks duplicate fields before structure
and size. No answer is repaired or executed, and no hidden or held-out test is
used. These observations select the next generation change; task quality is
still unknown.

Trusted endpoint settings may request a named inline object schema with
`response_format: {"json_schema":{"name":...,"schema":...}}`. The transport
encodes it as an upstream `json_schema` request with `strict: true`. The schema
is generic: no repository, folder, coding task or tool is required. Existing
text and JSON-object choices preserve their encoding. The engine resolves no
schema resources and refuses reference/resolution keywords. Schema bytes count
toward the existing request ceiling. Unsupported schema semantics are an
endpoint error, with no fallback, repair or automatic retry.

Only explicit trusted configuration selects the schema. Chat and proposal
bodies cannot select it. The exact schema is included in the immutable request
body binding and visible in trusted settings. A fresh endpoint choice without
the option restores text. Invalid choices preserve the previous settings.
This does not validate a model answer or confer any authority on it. The strict
coding checker continues to refuse duplicate keys, wrong shapes, excess bytes,
unsafe paths, invisible or ambiguous preimages and overlapping edits. No
prompt, task, threshold, admission, sandbox or permission changes.

The development repeat will use an object with exactly `edits`: an array of
one to 32 objects, each with exactly the required string fields `path`, `old`
and `new`. It will retain the previous context, output budget formula, complete
framing, thinking request and timeouts. Run all 30 tasks on each available
profile in resumable resource jobs and report changed variables, strict
proposals and refusals separately from task passes. No further tuning precedes
that repeat. Native admission and original grading controls remain required;
they are never reconstructed or bypassed. Independent core review is required.

## Placement of the released Git work

Repository reads (clone, fetch, log, diff, status) and approved pushes belong
inside the existing T-306 coding-help row, at its first capability boundary
after this measured refusal work and its development repeat, once completion
reliability can be measured. This precedes model-role selection at T-312 and
adds no row or sub-row. It preserves MEASURE FIRST: unknown task-completion
reliability does not justify adding a consequential tool now. No Git tool is
started by this increment. Every eventual push must show the exact branch,
commits and remote and receive a fresh yes; default-branch and force pushes
remain prohibited. Code-host issues/pull requests and other service actions
remain planning only.

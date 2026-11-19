# T-303/T-306: raw source presentation

Base: `c7dc0ebcea16dfebd19ef21d28805f9dbd2f405f`.
[Decision 0213](../decisions/0213-raw-source-presentation.md) selects one
model-visible hypothesis: remove the source-level JSON escape layer while
keeping the fixed instruction, selected source and strict checks unchanged.
This source batch performs no inference and adds no task score. The frozen
six-profile development repeat starts after this source commit.

## Behavior

Coding proposals now receive a JSON metadata line with the task, file outlines,
encoding version and excerpt count. Each following JSON header gives an
untrusted excerpt's path, original byte range and UTF-8 body byte count. The
body follows directly, byte for byte, with a separate framing newline. Quotes,
backslashes, tabs, newlines and CRLF remain source characters. Transport JSON
encoding still applies once to the complete message.

Framing follows byte counts, so source that resembles a header cannot terminate
its body. All content stays in the user role. Task text, source and diagnostics
remain untrusted data and confer no permission. Repair turns prepend labelled
JSON diagnostics without JSON-escaping the source again. Source getters and
API response excerpt records keep their original exact-byte contents.

Retrieval keeps its previous JSON-sized selection allowance, candidate ordering
and range rules. Actual raw message bytes must independently fit the same byte
ceiling. Both size checks include the unchanged fixed instruction. The strict
checker and source validator are byte-identical to the base: no normalization,
fuzzy match, repair, admission change or partial acceptance is introduced.

## Development preflight

All 30 authenticated development contexts retain the same task, file outlines,
excerpt ranges and exact source bytes. Their model messages change only in
presentation. All 161 available checks of previous answers reproduce identically
with the rebuilt effect-free library helper. This executes only context and
strict-check code, never candidate source, project tests or hidden tests.

An earlier preflight draft sized retrieval directly by raw bytes. It changed
11 tasks' selected excerpts and changed one previous check from proposal to
visibility refusal. That draft was discarded before inference. The final
implementation retains the prior selection allowance; all 30 excerpts and
161 checker outputs match. Both preflights and the interrupted intermediate
gate are retained privately. No discarded draft produced a model attempt.

The four new synthetic presentation tests cover byte-count round trips,
Unicode, quotes, literal escapes, CRLF, tabs, framing-like hostile source,
multiple files, omitted bytes, exact byte ceilings and refusal of newline,
escape or indentation repair. API tests continue to inspect the source actually
delivered upstream and require fresh native approval. Their old JSON-only
assertions were updated for the new framing after the first source gate exposed
four representation assumptions; no acceptance condition was removed.

## Verification and limits

Final formatting, Clippy, Markdown lint/render, privacy and licence checks
pass. The complete affected-crate test run remains **FAIL**: 1,034 passed,
53 inherited failures, 11 ignored and zero filtered across 30 targets.
All 53 failure identities match the retained preceding source run; no new
or missing failure remains. All four new presentation tests and all API
contracts pass. Development model-message sizes are 11,003–32,064 bytes,
including the fixed instruction, versus 11,300–32,767 previously. These
are bytes, not rendered token counts. No failed check is called green. No supply-chain regeneration or long acceptance gate runs
mid-batch. The inherited native-manifest failures, unavailable original grading
controls, and the licence batch's two Python failures and supply-chain limits
remain open. They are not replaced with new admission or grading controls.

Both preceding exact-commit independent reviews returned PASS and are retained
unchanged. The completion review's two low findings remain T-309 backlog:
use frozen task IDs in future scheduling guards, and persist and count token
retirement receipts before unlinking tokens. No row closes.

Next, freeze this source and measure all 30 development tasks on each of the
six existing profiles, in resumable GPU jobs with reservations released between
jobs. Keep the 32K context, per-task output formula, profiles, schema, thinking
request and deadlines. Report first-attempt and within-budget task passes as
unavailable while native effects, project tests, feedback and retries cannot
run. No candidate, hidden or held-out code is executed. No further tuning,
Git capability, service tool, flow or team work precedes the repeat.

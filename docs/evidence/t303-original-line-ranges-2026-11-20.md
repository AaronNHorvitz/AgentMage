# T-303/T-306: original line-range proposals

Base: `abab73ff91f63fdfc8c6cfe3d0691122fbbf9030`.
[Decision 0214](../decisions/0214-original-line-range-proposals.md) targets the
largest measured refusal cause: 36 absent originals in the completed repeat.
This batch implements and verifies a generation hypothesis. It runs no model
and establishes no improvement in coding quality. Both parent rows remain open;
independent core review is required.

## Behavior

The model can identify complete original lines using
`path/start_line/end_line/new`, with positive 1-based inclusive line numbers.
The runtime derives the original text from the bound snapshot and passes it
through the same strict preimage checks. LF ends a line; CRLF and a missing
final newline retain their exact bytes. There is no extra selectable empty
line after a final LF. Insertions retain an anchor; deletion permits empty new
text. Legacy `path/old/new` clients remain supported. An edit cannot mix forms;
separate valid edits of either form may share a batch.

An address does not waive whole-file uniqueness, visibility in one excerpt,
nonoverlap, effective changes, stale-source refusal or size bounds. A repeated
line must be extended with visible context until its text is unique. All edits
address the original snapshot and the whole batch succeeds or refuses. No
normalization, fuzzy matching, model-output repair or execution grant is added.
The source validator is byte-identical to the base. The shared checker tail is
byte-identical after substituting only the resolved-edit binding in its loop.

Version 2 headers add original line bounds to the existing byte-counted raw
source framing. Body bytes remain unchanged. The fixed instruction prefers
line references and permits exact-text edits if a configured schema requires
that form. The API does not replace a person's chosen schema. A new socket
contract explicitly configures the line schema and streams a checked unapplied,
untested proposal through the existing authenticated HTTP route. It is a stub,
not a real-model or native-workflow result. See the
[proposal guide](../guides/coding-proposals.md) for both strict forms.

## Development preflight

All **30** frozen development sources are authenticated against the complete
consumed files and their historical Git objects; none is imported or executed.
All **165** recorded answer-check results reproduce identically, including
format refusals. **185** line-range controls match their exact-text equivalents.
These controls replace visible excerpts in memory; they are not task solutions.
Six additional duplicate-null-key controls refuse with the format error.

Selected excerpts change on nine tasks: dev-002, dev-006, dev-007, dev-008,
dev-013, dev-015, dev-016, dev-019 and dev-027. The same selector, candidate order,
range rules and JSON-sized allowance remain. Changed instruction and header
bytes affect the available source allowance. Every delivered body matches its
original UTF-8 range, and the original line bounds are independently counted.
Message content sizes are **11,022–32,631 bytes**, including the instruction;
these are not rendered model token counts. Token fit is not measured here.

Preflight SHA-256: `bd067960ee81c0b8526ef7d360ba5a1bf23a4d1e8066b99431826a5b8d6cb84d`.
Method SHA-256: `23c9fc3bfa62bd1cdce75ae478afefd0a8842fff05eaaa0cbd70cba496d162ad`.

The preflight reuses one prepared context per task to check a batch of answers,
and independently compares its complete context with the unchanged single-call
helper entry. Both helpers call the same production library. Full sources,
answers, checks, contexts and records remain bound as complete files; batching
does not narrow any evidence input. The new capsule authenticates its **721**
consumed predecessor inputs through the pinned complete predecessor manifest.
It does not rerun or replace the predecessor's complete inventory verification.
Earlier capsules remain unchanged.

## Verification and limits

Eight new Rust tests cover all ranges in small UTF-8/LF/CRLF fixtures, insertion,
deletion, mixed edits, repeated preimages, stale snapshots, omitted lines,
overlap, malformed shapes, duplicate keys, nulls, integer bounds, batch refusal,
edit counts and aggregate size limits. Existing presentation tests additionally
verify the new header line bounds; the new HTTP socket contract passes.

Final formatting and Clippy pass. The complete affected-crate source gate stays
**FAIL**: **1,043 passed, 53 inherited failures, 11 ignored,
zero filtered**, across **31 targets**. Every failure identity matches
the previous source gate. No failed test is allowlisted or called green.

The first draft's 1,467-byte instruction caused one additional failure in an
existing three-file fixture at the unchanged 2,048-byte minimum. Its full gate,
source and preflight are retained. The final instruction is 1,061 bytes, versus
1,072 previously. The original fixture and budget stay unchanged and pass in
the final full gate. No inference used the discarded draft. Its initial helper
was subsequently rebuilt; only the final helper binaries are retained, alongside
the complete initial source and logs. The initial preflight is not rerun by the
new verifier and supports no model result.

Markdown lint/render, privacy, licence and preservation checks pass. All 512
ledger table lines, 49 historical path blobs, the planning inventory, Decision
0051, status model and README remain unchanged. Earlier scoreboard bytes remain
an exact prefix; only a preparation note is appended. Both preceding independent
PASS reports are retained unchanged. Their three low findings stay T-309 backlog.
No supply-chain regeneration, long acceptance gate, release or Git push occurs.
Existing native-admission, grading-control and earlier disclosed licence-batch
Python/supply-chain limitations remain open.

## Next measurement

Freeze this source and the strict line schema, then run all 30 development tasks
on all six existing profiles in resumable GPU reservations before more tuning
or capability work. Keep 32K context, the per-task output formula, profile
settings, thinking request and deadlines. Record source changes and actual token
fit rather than assuming that byte fit establishes token fit. The changed
variable is the complete line-reference protocol, not an isolated prompt edit.

No model inference, candidate execution, project tests, test feedback, retries,
hidden or held-out work occurs here. Native executable admission and original
execution/grading controls remain unavailable and are not reconstructed. First
attempt and within-budget task passes remain unavailable. No role or model is
qualified and no row closes.

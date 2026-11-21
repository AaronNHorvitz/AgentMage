# T-303/T-306: identify unique visible original anchor lines

Base: `18c1f7889709a4dcabc0fa789e6b07a66fb9ea98`.
[Decision 0217](../decisions/0217-visible-unique-line-hints.md) follows the
completed development repeat's 31 repeated-original refusals: 30 use line ranges,
29 select one line and 16 select whitespace. It also considers the overlapping
69/102 accepted proposals with joined-line boundaries. Those observations are
not task grades or an estimate of improvement.

This is **unmeasured preparation**, with no new inference. T-303/T-306 remain
open and this increment requires independent core review. The preceding
[exact-commit diagnosis PASS](../reviews/18c1f7889709-original-address-refusal-diagnosis.txt)
is retained unchanged. It has no high or medium finding; existing low findings
remain T-309 backlog. No row closes.

## Changed variable and component checks

Version 3 raw-source headers add `unique_lines`, compressed inclusive intervals
of individually unique visible original lines containing non-whitespace text.
Uniqueness uses exact substrings of the complete bound file, including matches
inside another line and overlapping matches. LF defines lines; CRLF and an
unterminated final line retain exact bytes. Results are computed once per file,
with repeated lines sharing their scans. Binary searches restrict header work
to visible anchor numbers. Adjacent unique lines share one interval.

The fixed instruction changes from **1030 to 1029 UTF-8 bytes**. It explains
using these hints when selecting repeated blank or brace lines, preserving the
anchor in the replacement, and explicitly shows the difference between a
replacement with and without final LF. Hints are advisory metadata. Other unique
visible ranges remain valid; an empty list says nothing about multi-line
uniqueness. Every edit still passes the strict checker. No source body, answer,
range or newline is repaired.

The complete parser/resolver/checker portion is byte-identical to the preceding
source. Output schema, retrieval ranking and selection allowance, file and
message limits, snapshot binding, visibility, uniqueness, overlap, permissions
and all-or-nothing refusal remain unchanged. The actual message limit charges
every new header byte, so excerpt coverage can change.

All **17 targeted tests pass** (10 line-edit and 7 source-presentation tests).
Three new tests use an independent overlapping byte-window oracle for hint
accuracy, complete-file versus excerpt-only uniqueness, Unicode, CRLF,
unterminated lines, omitted repeats, compressed intervals, deterministic output
and tight budgets. They check advertised anchors through the strict checker,
repeated blank-line refusal, retained insertion context, exact missing-LF
behavior and invalid replacement refusal. Existing source and HTTP fixture framing-version assertions
advance to v3; previous tests and budget thresholds remain unchanged. These
are effect-free component tests, not measured coding tasks.

## Development preflight and disclosed coverage cost

The capsule authenticates **1090 consumed files** against the unchanged
originating 119284-entry manifest and the preceding diagnosis manifest. Complete
predecessor inventory is not reverified. All 30 development source payloads match
exact historical Git blobs. The campaign-bound original helper reproduces all
30 prior contexts and **167 complete-reply checks**. All 102 original strict
proposals remain ungraded in their original measurement.

The new helper freezes all **30** development contexts, checks exact raw-body
bytes, LF line bounds, framing and hint accuracy, and compares the same prior
answers under those contexts. Selected excerpts change on **11 tasks**:
dev-001, dev-002, dev-006, dev-007, dev-008, dev-013, dev-015, dev-016, dev-019, dev-027, dev-028.
Message content sizes are **11,246–32,768 bytes**, not rendered tokens.
Actual model token fit remains unmeasured. Schema and six profile files retain
their original bindings.

There are **3 prior-answer check differences**, all accepted-to-refused on
dev-008: gemma4-26b-a4b, gemma4-31b and qwen35-27b. Each answer selects original
lines 1083–1120 (UTF-8 bytes 46354–48128), previously visible and now omitted.
The added metadata displaces that source under the unchanged message budget.
The visibility checker correctly refuses these unchanged answers. This is a
coverage regression of the new context, not a reclassification of the old run
or a new model outcome. The other 164 checks retain their exact results.
No answer is repaired, no result is removed and no improvement is claimed.
The new development repeat must measure the complete changed input.

Preflight SHA-256: `1ea2a48eea8ba5db9cf81132c69f08e7166b815616592abc53031b844ad21244`.
Method SHA-256: `3811266d460dbadf972e43ad061619ef6c3bed5ce66be3651964e60629423dfb`.
Checker preservation SHA-256: `44a47f1754ea10e8861dd0ffb5b5b97f6eb7714cff8a89c8223ac08518ad1e78`.
Visibility diagnosis SHA-256: `2b7523b607df0a6d12186d0c282f1cb0ce307965a2a3cb0997e658f39eea906f`.

No task loader, candidate tree, project test, hidden test or held-out input is
used. These exclusions concern evaluation tasks; the source gate runs
AgentMage's own tests and their synthetic fixtures. Proposed code remains data.
Native executable admission and original
execution/grading controls remain unavailable and are not reconstructed.
First-attempt and within-budget task passes, feedback and retries remain
unavailable. No model is admitted, qualified or selected for a role.

## Verification and next work

Formatting and Clippy pass. The full affected-crate gate remains **FAIL**:
**1,048 passed, 53 inherited failures, 11 ignored, zero filtered, 31 targets**.
All failing test identities match the preceding source gate; none is added or
removed. The three new component tests account for the extra passing tests.
No test is skipped, allowlisted or weakened. The retained final source-gate log
covers the final production source. An earlier invocation was interrupted while
waiting for a nested reservation, before its Cargo checks; its log is retained.
The final invocation uses the gate's own per-command build reservations.
The first completed run has 1,043 passes, 58 failures and 11 ignored tests.
Four extra failures came from the HTTP fixture's stale v2 assertion, advanced
to the emitted v3 framing without weakening its equality check. One unrelated
terminal-transfer transcript assertion also failed and then passed in isolation.
The corrected HTTP suite passes all 53 tests; both rerun logs and the final
full-gate outcome are retained separately. The initial completed run is
preserved and is not relabelled as the final result.
Direct Markdown lint/render, review format, privacy, licence, preservation and
sealed preflight reproduction must pass before requesting independent review.

All 512 TASKS table lines, 49 historical path blobs, README, historical inventory,
Decision 0051, status model and supply-chain outputs remain unchanged. Earlier
scoreboard bytes remain an exact prefix. No SBOM regeneration, long acceptance
gate, release, owner-test-ready or Git push occurs.

Freeze this exact source and all 30 contexts, then run every development task on
all six profiles in resumable GPU reservations before further tuning or
capability work. Keep 32K context, per-task output formula, profiles, thinking
request and deadlines. Publish actual fit, refusals, strict ungraded proposals,
coverage, tokens and time, without treating accepted edits as solved tasks.
Existing T-309 lows remain backlog. Independent core review is required;
T-303/T-306 remain open.

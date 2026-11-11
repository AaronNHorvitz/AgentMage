# Exact source coverage — 2026-11-11

Decision 0203 addresses the latest primary-model development measurement's
largest individual refusal cause: eight strict edit refusals. Parser-backed
excerpts now retain complete original lines, including indentation and newlines.
Overlapping or touching ranges merge when the serialized message fits the
existing ceiling; a complete file can replace smaller selected excerpts. Retained
ranges have canonical path/offset order. The fixed instruction explicitly asks
for exact whitespace, distinguishing context and effective changes.

The checker and proposal data definitions are byte-identical to the preceding
commit. Ambiguous, invisible, altered, overlapping, ineffective and stale
preimages still refuse the entire proposal. No fuzzy application, normalization,
evaluation-test change, admission relaxation or tool authority is introduced.

## Verification

The complete affected fast gate is **FAIL**: 1,020 passed, 53 failed, 11 ignored,
zero measured or filtered, across 29 unfiltered targets. Every failure and ignore
identity in a shared target matches the retained candidate-build baseline.
Five Linux targets from that broader baseline are outside this change's affected
scope; their previous results are not new verification. Formatting, Clippy with
warnings denied, Markdown lint and untracked whitespace checks pass. The new
three-method integration target passes in the full run, alongside the existing
strict proposal tests. No timeout or additional skip was introduced.

The regressions exercise a complete file replacing smaller definitions,
cross-definition edits, exact indentation/CRLF/Unicode boundaries under a tight
budget, deterministic canonical ranges, omitted-source refusal and refusal of
newline normalization. These are component tests, not a native coding workflow.

An effect-free diagnostic rechecks the preceding run's 21 completed answers
against the new context. Two previously refused proposals become strict proposals;
the other refusals remain. This is saved-answer reuse, with no inference, test
execution or task grade. It is not a scoreboard measurement or a solve-rate gain.

The complete gate log has SHA-256
`2be8c7f6da0f8784d61963b6b31db051e2a99dd90bd07f7d6b4e000cb5aded81`.
The private review inputs retain the exact log and target/failure comparison.
Development inference and its scoreboard entry remain pending at this source
checkpoint. T-303/T-306 stay open; an independent core verdict is required.

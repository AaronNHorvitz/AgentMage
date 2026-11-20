# Decision 0215: Explain unique anchors when generating line edits

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-20 |
| Authority | MEASURE FIRST; T-303/T-306 |
| Scope | Fixed generation instruction; no new edit or execution authority |

The completed six-profile line-range repeat has 21 repeated-original refusals,
the largest individual refusal condition. Nineteen select one line and ten
select only whitespace. All addressed ranges are visible, but an address does
not waive whole-file uniqueness. The next experiment explains that requirement
more concretely, especially for insertions beside repeated blank lines.

Change only the fixed proposal instruction. Tell the model to extend repeated
blank or brace-only ranges with visible neighboring code, retain that context
in the replacement, and include an insertion's unique anchor unchanged. State
the ordered inclusive line bounds explicitly. Explain that the replacement
includes the whole selected range and its line endings, including the LF before
following source. These are generation instructions, not additional acceptance
rules. The runtime never extends a submitted range or adds a missing newline.

The parser, strict checker, retrieval algorithm, raw framing, schema, limits,
permissions and execution boundaries stay unchanged. Rewording keeps the
instruction within its previous byte allowance; existing minimum-budget tests
remain unchanged. The selector still charges the actual instruction, so freeze
and report any resulting excerpt changes. Exact-text clients remain supported.
This supersedes only Decision 0214's generation wording; its edit contract and
the prior frozen repeat remain unchanged. No earlier answer is repaired or
reclassified. The reviewer observation about missing replacement line endings
is diagnostic development data, not a reason to change the checker.

Before more tuning or capability work, freeze this source and all 30 development
contexts and repeat every task on all six existing profiles in resumable GPU
reservations. Keep 32K context, the per-task output formula, profile settings,
thinking request and deadlines. Publish actual token fit, strict ungraded
proposals, refusal causes, tokens and time. A smaller instruction is not evidence
that the rendered input fits or that coding quality improves.

No candidate, project test, hidden or held-out input is executed in preparation.
Native executable admission and original grading controls remain unavailable;
do not reconstruct authority or claim task passes, feedback or retries.
Independent core review is required. T-303 and T-306 remain open.

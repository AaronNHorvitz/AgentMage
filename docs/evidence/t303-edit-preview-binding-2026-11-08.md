# T-303/T-306: retain the original prepared edit

Date: 2026-11-08. Tier: core. Immutable predecessor:
`d78e31e5734fede78cf0e81f9068871d20e42aaa`.
Both parent rows remain open. Independent exact-commit review is required;
no row is added or closed.

## Behavior

The guided command previously discarded the edit-prepare reply and bound its
approval helper to the first subsequent pending-edit inspection. Another control
token holder could replace the edit in that interval, keep the original root
and creation receipt, and have the replacement accepted as this attempt's edit.
The replacement still needed exact human consent, but it no longer belonged to
the proposal the workflow prepared.

The shared proposal and preparation helpers now return the original preview as
well as displaying it. The guided command checks that reply's operation, held
worktree root and creation receipt, then requires both pending-edit inspections
to equal the complete original reply. The approval helper receives that same
original value. Changing the digest or any preview content stops the attempt
before edit consent or dispatch. The original native owner still decides the
exact approved digest and strict preimages when writing. Standalone commands
retain their displayed output and separate approvals.

This addresses low L1 from the guided command's independent review. The
actual-service consumer case (L2) and earlier lows remain same-row backlog.
See the [guided workflow](../guides/coding-guided-worktree.md).

## Verification

The complete ordinary pre-edit API baseline passed: **160 tests across ten
targets**, zero failed, ignored or filtered, plus an empty passing doctest
target. The three relevant source hashes matched the predecessor's final gate
before execution and after completion.
The new first-inspection regression then failed original production: a stable
replacement in both inspections was accepted and the synthetic workflow ended
successfully. The other new regression passed original production, exercising
the already implemented second-inspection comparison. The raw expected failure
is retained, with one passing and one failing method; assertions are unchanged
in the production fix.

Two executable regression methods cover both replacement windows, each on an
initial proposal and a failure-informed repair, using changed preview digests
and changed diff content with the digest field retained. They require exact
stopping positions, no new edit or test dispatch, and no edit approval prompt
for the replaced preview. The existing positive and denial cases remain.
The recording HTTP peer supplies synthetic model, edit and command replies;
fixture source remains unchanged. No native worker, local model or project-test
process is executed by these cases.

The final ordinary affected-package gate is **PASS: ten targets, 162 passed,
zero failed, ignored or filtered**. Every one of the 160 prior passing test
identities remains, plus exactly the two named regression methods. Formatting,
strict Clippy, Markdown and whitespace checks pass. Six gate commands took
**70.64 seconds** including their individual build reservations. All three
Rust hashes stayed frozen through that gate; no source edit followed it.
Raw baseline, expected regression failure, final gate, source hashes and the
identity audit remain private. The private audit parser explicitly checks the
baseline's additional empty doctest target; its initial parsing failure did not
change a raw log, test or gate. Final lightweight checks cover the current
documents, scope and privacy. Deferred host/Linux T-301.1 is not rerun or waived;
this scoped API pass cannot establish whole-workspace acceptance.

## Limits

Independent core review is required. T-303/T-306 remain open. Native execution
is still blocked by fixed system-executable ownership admission in this lane;
no trust or sandbox boundary is relaxed. Autonomous endpoint edit/test
composition, durable recovery, approved Git and parent measurements remain open.
No evaluation inputs, hidden or held-out tests, scorer, thresholds, model/GPU
work, dependencies, SBOM, status model, scoreboard, release or push changes.

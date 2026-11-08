# T-303/T-306: bind guided proposal admission

Date: 2026-11-08. Tier: core. Immutable predecessor:
`ec1f3ee1d934d40a633e8aaa14c9ab3de7f3c000`.
Both parent rows remain open. Independent exact-commit core review is required;
no row is added or closed.

## Behavior

The guided terminal checked its original selection and settings before calling
the shared proposal helper. That helper fetched selection again and, for a
repair, fetched current settings again. Those newer observations could replace
the binding checked by the workflow. Its initial request also lacked an API
pin to the original settings and held worktree root.

The helper now compares its fresh observations to the original binding. Every
guided proposal carries the original selection identity, settings revision,
exact endpoint and held worktree root digest. The original API checks pins
under its existing operation lease, before reading source or dispatching the
model. A settings pair must be complete. The repair path retains its original
settings check and additionally checks selection and root pins before failure
admission, then reuses the existing scoped source reader. Correct pins cannot
create ownership, approve disclosure or fabricate an eligible failure.

The initial root pin also refuses an already selected directory other than the
held worktree. A root digest identifies a directory, not its source content.
Existing complete preimages, strict checker, diff and test approvals, grants,
sandbox and retry limits remain authoritative. The older API contract still
works when optional pins are omitted; the guided command always supplies them.
Shared standalone proposal commands now forward a supplied selection identity.
See the [guided guide](../guides/coding-guided-worktree.md),
[workspace contract](../guides/coding-workspace.md) and
[repair contract](../guides/coding-model-repair.md).

## Verification

The complete ordinary pre-edit API baseline passed: **155 tests across ten
targets**, zero failed, ignored or filtered. A new executable regression then
failed the original implementation: after replacing the selection during the
helper's fresh read, the old client completed both synthetic attempts and
reported success. The original failing assertion and raw log are retained
privately. Its assertion is unchanged in the fix.

The seven guided executable methods pass after the fix. Two new methods check
the complete binding on both proposals and exact early refusal for selection,
repair settings and repair root replacement. The recording peer supplies
synthetic model, write and command replies; fixture source never changes.
Two additional real API/socket methods check precise stale/partial-pin
refusals before source access or any upstream connection, and a complete bound
proposal through the original transport and strict checker. The stale tests
replace the selected source with a symlink to distinguish early binding
refusal from a later file-boundary refusal, and cover reapproving the same
endpoint. A fifth new method checks repair pins through the actual API and
confirms that correct pins still cannot invent worktree ownership or a failure.
The upstream remains synthetic; no native command or local model is executed.

The first full gate ran all **160 tests across ten targets** successfully but
failed strict Clippy on the new selection-identity presence check's Boolean
spelling. The raw failed gate and original source freeze are retained. Only
that expression was simplified; assertions, test inputs and thresholds stayed
unchanged. The complete gate was then rerun with the final source batch frozen.

The final ordinary affected-package gate is **PASS: ten targets, 160 passed,
zero failed, ignored or filtered**. Every one of the 155 baseline passing
identities remains, plus exactly the five named additions. Formatting, strict
Clippy, Markdown, whitespace and scope/privacy checks pass. Six gate commands
took **24.78 seconds** including individual build reservations. All seven Rust
inputs stayed frozen throughout the final gate; no source edit followed it.
Raw baseline, regression, initial failure, final gate, source freezes and the
identity audit remain private. Current documents and the retained review receive
separate final lightweight checks. Deferred host/Linux T-301.1 gates were not
rerun or waived; this is a scoped API gate, not a whole-workspace pass.

## Independent predecessor acceptance

The guided command's [independent core review](../reviews/ec1f3ee1d934-local-coding-guided-worktree.txt)
arrived during verification: **Verdict: PASS** for its exact immutable commit,
with no high or medium finding. It is retained unchanged. Low L1 (the prepare
reply is not bound to the first edit inspection, and neither replacement window
has discriminating coverage) and L2 (no guided consumer test against the actual
service) remain backlog in these same rows. The current guide now describes
the narrower preview comparison precisely. The full displayed diff still needs
its exact approval; this increment does not fix that low finding. Earlier lows
also remain backlog. The reviewer's missing-Node exit 127 remains recorded as
a documentation-lint failure, without a waiver. Its acceptance closes neither
parent and supplies no verdict for this new binding increment.

## Limits

This increment requires independent review and supplies no parent acceptance.
Native positive execution remains
unobserved: the lane's system executable ownership still fails original
root-owned admission. No boundary is relaxed. Autonomous endpoint edit/test
composition, durable execution recovery, approved Git and parent measurements
remain open. No evaluation inputs, hidden tests, held-out tasks, scorer,
thresholds, model/GPU work, dependency, SBOM, status model, scoreboard, release
or push changes.

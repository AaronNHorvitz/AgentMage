# Decision 0076: Native Git Schema Fidelity

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Disclosure of existing native Git inspection argument constraints |
| Preserves | Native validators, exact tool/schema hashes, grants, confinement, correction budgets and verifier ownership |

## Evidence

Muse campaign9 stable1 at `22e5c44e` ran complete passing validation, then twice
proposed Git status with `pathspecs:[["."]]`. Both frames were valid ATEM and
complete EOS, with ample context/output headroom. The second proposal changed
only output/record limits after the one allowed correction notice. Both requests
were correctly refused before permission or effects, and the run exhausted.

The actual argument hashes are
`139004858d8a258c64870090adc33df6208f0a3c7f775945d726258b4ab28953`
and `9039f96ca7972cef22ef259d7bde07ba5204ad01b7db2bb18fa6324c296ef6a0`.
A read-only AJV reproduction confirmed the published input schema accepts both,
although the native owner rejects both. This is an integration disclosure defect,
not evidence that the owner should accept dot paths or broaden Git operations.

## Decision

Align the model-visible Git schema with existing `validate_request` rules: only
diff/staged_diff/show may have nonempty path selections; all other operations
require `[]`. Show/ref require a revision, log permits one, and others require
null. Only object accepts and requires an exact object ID. Publish existing
revision grammar and ordinary canonical component refusals, including dot,
traversal, separators, encoded path syntax, controls and ambiguous suffixes.
Describe the native UTF-8 byte and NFC/NFKC checks explicitly; JSON Schema's
character limit is not a substitute for those unchanged native checks.

Add typed, native-validated status/diff examples to the existing context. Explain
that `[]` selects the held worktree, not `[["."]]`, and that untracked creations
are evidenced by their creation receipt/postimage plus status, not an empty
ordinary Git diff. No new tool, wrapper, executor or coordinator path is introduced.

Do not normalize rejected arguments or relax native parsing. The exact published
schema hash and resulting tool/catalog bindings change normally; stale schema or
profile bindings remain inadmissible. No model, generation, context, resource or
parser budget increases. This narrows disclosure to the already admitted native
contract, not the scope of effect authority.

## Verification and disposition

Retain both exact failed requests as native-owner and model-schema regressions.
Check all thirteen operations' required/null revision/object fields and path
selection rules, safe examples, unsafe paths/revisions and unknown fields. Keep
raw native failures and the before-fix schema mismatch report. Exercise actual
CLI/host regressions, then a new separately pinned native campaign; never relabel
the failed campaign or combine earlier favorable tuples. Independent review,
model admission, daily-use, platform and release gates remain open.

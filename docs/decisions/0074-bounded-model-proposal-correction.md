# Decision 0074: Bounded Model Proposal Correction

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Pre-effect protocol/schema correction in the existing coordinator |
| Preserves | Strict decoding, schemas, identities, exact grants, confinement, verifier and existing budgets |

## Evidence

At `bc2d53db`, Muse's multi-file attempt selected the correct validation tool but
omitted the opening ATEM wrapper (raw SHA-256
`84d1777465aba476dec9197b202007aa7bacab510877218d104d1e82d858ae4e`).
The pinned upstream template requires that wrapper. Its stable-fixture attempt
used flat Git pathspecs instead of component arrays (raw SHA-256
`7b13b9b5e6e47857a1c68ba393c4227bcf29b39dd857a6f65147746a6f013ed5`).
The unchanged registry correctly rejected the arguments, but the coordinator
reported a generic boundary failure instead of an actionable pre-effect rejection.

GPT's distinct new-file attempt recovered from an unavailable read, created the
authorized file, then emitted an invalid duplicate Harmony channel delimiter
(raw SHA-256 `638fd1d7f9f93be065b668f0c97a7bd47ed8c65716ce82c2c324974905906ee0`).
No malformed frame is reinterpreted as a valid call. These failed attempts remain
failed; new source requires a new campaign. The same source also demonstrated a
complete real Muse failed-test repair, so neither these rejections nor the older
integration defects justify a blanket model-incapability claim.

## Decision

Use the existing coordinator's next turn and canonical continuation to expose a
bounded, explicitly non-authoritative correction observation. There is no hidden
retry, new execution loop, automatic argument repair or alternate store.

Only complete, identity/stream/digest/usage-verified output rejected by an explicit
native syntax decoder allowlist may become a recoverable protocol rejection.
Preserve the exact rejected bytes and validated resource report before recovery;
retention failure remains terminal. Unknown codec errors, runtime/transport errors,
truncation, context/output exhaustion, stale identity and cancellation remain
terminal. Rejected bytes never become a proposal, tool result or verifier evidence.

For tool arguments, first validate the closed envelope, digest and exact frozen
tool/version/schema binding. Only a subsequent registered argument-validator
rejection with an explicit syntax-only opt-in by that tool owner may return
correction feedback, before permission evaluation or worker
launch. Unknown identity, bad envelope, mismatched digest/schema, authority denial,
preimage drift and effect failures retain terminal behavior. Decision 0072's
separate unavailable-read condition remains unchanged. This narrowly supersedes
its terminal-schema-error rule, without relaxing any schema.

The default opt-in is false. Initially only the stateless Git inspection parser
opts in: it has no policy, inventory, intent/preimage or grant inputs. In particular,
coding-write validators also enforce owned-path scope, so their refusals remain
terminal. Future tools cannot inherit correction eligibility accidentally.

Both conditions consume the existing parser-failure allowance of **one per run**,
plus ordinary model/turn/tool/input/time/repetition/no-progress budgets as
applicable. A second parser rejection exhausts the run. No budget increases.
One additional Observation-to-Checkpoint edge closes a rejected model turn before
any proposal or tool exists; it cannot authorize an effect. Existing ModelFailed,
ToolRejected and TurnCompleted records bind the exact rejection to the canonical
continuation. Resume restores accounting and checks those event bindings without
replaying failed output or consumed grants. Missing new continuation fields fail
closed; no silent historical migration.

Context labels rejection feedback separately from receipts and retained evidence,
keeps the latest rejection essential, and does not inject raw private reasoning as
instructions. The model must submit a new strictly valid proposal. Completion
still requires the existing verifier and fresh complete validation/diff evidence.

## Verification required

Retained ATEM/Harmony and flat-pathspec failures remain negative decoder/schema
fixtures. Prove one rejected proposal followed by valid correction, no permission
or effect on rejection, unchanged strict envelope/identity failures, bounded
repeated rejection, truthful resource accounting, exact continuation/event binding,
tamper/drift refusal and non-replay resume. Exercise the actual CLI/host fixture,
then repeat separate native campaigns. Scripted correction cannot qualify a model.
Independent review and all admission/platform/release gates remain open.

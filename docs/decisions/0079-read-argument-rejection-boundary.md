# Decision 0079: Read Argument Rejection Boundary

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Registered read-only argument rejection in the existing coordinator |
| Preserves | Strict native schemas, exact grants, confinement, verifier ownership and all resource/retry limits |

## Evidence

Muse campaign12 at `d8557c993838ba54a85dba7a364a9495b6d92496` completed
five distinct/repeated native cases, then new-file2 failed with no terminal
outcome. After observing a genuine failed validation, the model proposed
`agentmage.workspace.list-directory` with `paths:["src"]` instead of nested
component lists. Its ATEM frame and generic JSON were valid; the native read
validator correctly refused the shape. The coordinator then returned
`InvalidBoundaryResult` because this owner did not opt into correction.

The exact raw response SHA-256 is
`67383cce1d575c78d9597090e981fbd0c1c076e2f0b4265cd0e81018979e993c`;
the argument bytes, retained as a fixture, hash to
`aa960a422a134655e7cc9eeebc922e74d9d10883d3fdc1e5721693c9544b5028`.
This is neither truncation nor a context/generation/resource-pressure failure.
All attempts remain retained, and campaign12 is not qualified.

## Decision

Explicitly opt the registered workspace read-only owner into Decision 0074's
existing single parser-correction allowance **only** when its unchanged parser
returns `ReadOnlyRequestError::Malformed`. That parser has no policy, inventory,
intent/preimage or grant inputs. No argument bytes are repaired or normalized.
The model must propose fresh strictly valid arguments on the next ordinary turn.

`ReadOnlyRequestError::Denied`, including path traversal, incompatible operation,
schema version and hard bounds, remains non-correctable. Artifact, write,
validation-template and generic-command owners do not gain this opt-in. Live
path admission, permission denial, grant consumption and effect failures retain
their existing behavior. Exact envelope, digest and tool/version/schema binding
must still pass before any argument-rejection handling.

A non-correctable registered argument rejection is a controlled terminal
refusal, not an internal coordinator failure. Emit the existing `ToolRequested`
and `ToolRejected` records, count its ordinary attempt/input/parser usage, close
the turn and return `FAILED` with the existing `runtime.proposal.invalid` code.
If the existing parser allowance is already consumed, return `EXHAUSTED`.
Neither path requests permission, creates a grant, starts a worker or fabricates
a receipt. Only the existing owner-opted-in path can continue after rejection.

No new loop, store, retry budget, model limit or completion exception is added.
Existing correction continuation/resume accounting and verifier checks apply.
Actual-process scripted regressions remain distinct from subsequent native
campaigns, model admission and independent review.

## Verification required

Reproduce the retained shape refusal before the fix. Exercise the exact bytes,
corrected valid arguments, path/limit/version/depth denials, forged digest/schema,
non-opted-in terminal closure and already-consumed correction budget. Run the
actual CLI/host with both a malformed read followed by bounded genuine-test
repair and a denied read followed by no effects. Preserve failed and successful
logs. Rerun the executable matrix and separate exact-model campaign dispositions.

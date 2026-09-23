# Decision 0072: Bounded Pre-effect Read Rejection

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Existing coordinator feedback for an unselectable frozen read projection |
| Preserves | Frozen inventory, schemas, approvals, exact grants, confinement, verifier, budgets and canonical stores |

## Evidence

Muse's corrected new-file attempt at `b7fbf289` observed the genuine failing
validation and emitted a structurally valid read of absent `src/calc.py`.
Raw SHA-256: `a9a7e9b2110117856029230a65dab7bae3a722ddbe9f09a93bf2e7c0879a1161`.
The native planner rejected the path against its frozen inventory before path
resolution, approval or worker launch. The host erased that distinction into
`Dependency(Invalid)` and aborted. This is a local error-reporting/continuation
defect, not evidence of a codec or model-capability failure.

## Decision

Preserve this exact read rejection. Carry only the closed
`ReadProjectionUnavailable` condition from native projection through the Linux
boundary. Other schema, drift, identity, resource, policy, grant and effect
failures retain their existing terminal behavior. Do not reveal whether an
unselectable path is missing, excluded or the wrong kind.

The existing coordinator records `ToolRejected` before any permission event,
retains the exact call/reason in its existing hash-bound continuation, and
publishes bounded host observation text in the next context. This is not a tool
result, read receipt, filesystem absence proof, grant or verifier evidence.
The event verifier rejects this transition after approval or launch. Recovery
is a new model turn, subject to unchanged tool/turn/repetition/no-progress limits;
there is no automatic retry or second execution loop.

Add the narrow Approval-to-Checkpoint state edge for that pre-effect outcome.
Continuation validation binds the complete disjoint ordered sets of completed
and rejected calls to the original attempt ledger. Resume also checks every
rejection against the exact canonical event. Old continuations without the new
required field fail closed; they are not silently migrated or replayed.

Regression covers the actual raw Muse frame and decoded arguments, no authority
or effect on rejection, no conversion of write failures into read recovery,
bounded repeated rejection, context/continuation integrity and an actual-process
scripted rejected-read followed by controlled creation. Native qualification
still requires a separate real-model campaign; the fixture cannot qualify it.

GPT native-7's extra Harmony channel separator is retained as another negative
fixture. Its header is not accepted by relaxing the decoder. No generation or
resource limits change under this decision.

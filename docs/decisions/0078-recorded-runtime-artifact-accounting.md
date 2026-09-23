# Decision 0078: Recorded Runtime Artifact Accounting

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Required record accounting and terminal pressure handling in the existing coordinator |
| Preserves | Fixed resource ceilings, exact grants, confinement, native tools, verifier, complete evidence and independent review |

## Evidence

Muse campaign11 multi-file1 at `99cc4e16` performed nine native effects: failed
validation of two modules, hash/read/read, two exact patches, fresh complete
passing validation, status and diff. Its final response was a valid completion
candidate, not a decoder failure. The host instead returned
`coding.coordinator.failed:InvalidBoundaryResult`; exit 5 remains a failed run.

The ledger derived only `max_tool_calls + max_turns + 1` artifacts: 33 for the
sealed 16-turn/16-tool request. Consented recording requires context, model
result and safe-boundary continuation records, as well as tool payloads and the
initial request/final answer. The final model result required record 34.
Actual recorded payload sizes reproduce `ArtifactExhausted` in a ledger test.
The nine-tool scripted path through the actual CLI/host reproduces exit 5 at
the same boundary. Its failing validation and successful repair remain retained;
neither attempt is relabeled successful.

The native run used less than 3 MiB of artifact payload, at most 28225 input
tokens and 2038 generated tokens. The final request used 27289 input tokens
after exact context reflow. No output-token, context, memory or GPU increase is
indicated. Its valid final raw response SHA-256 is
`99888e8451369986d29fca4926e16bb815f3fa30da3d04b0ffd457448103db2a`.

## Decision

Derive the record allowance as `3 * max_turns + max_tool_calls + 2`, using
checked arithmetic and the unchanged fixed 1024-record cap. The three per-turn
slots account for context, model result and continuation, including bounded
rejected-proposal turns. Preserve the existing one-per-tool payload allowance
and add the initial request and final answer. Additional tool streams share
this bounded allowance; it is not a guarantee that every possible payload fits.

The sealed disk budget, fixed 64 MiB artifact cap, event budget, output limit,
turn/tool/model budgets and CPU/RAM/GPU limits remain independently enforced.
No mandatory record is omitted to fit. Arithmetic overflow and count/byte
overage remain fail-closed and non-mutating.

When context or completed model-result retention exhausts its allowance,
produce a canonical `EXHAUSTED` terminal outcome without processing that
proposal, requesting tool authority or claiming completion. Account completed
inference usage before attempting result retention. Preserve canonical terminal
events and prior records. Failure to publish an admitted artifact remains an
actual boundary failure; it is not converted into a success or silent omission.
Initial request/disk admission and existing safe-checkpoint pressure semantics
are unchanged.

Strengthen the actual-process multi-file case with the observed inspection
sequence and require verified records beyond the obsolete count estimate.
Keep scripted regression evidence distinct from subsequent native model runs.
No coding qualification, daily-use gate, independent review, platform or release
completion follows from this correction alone.

# Decision 0097: Research Completion Normalization

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-28 |
| Authority | Decisions 0054, 0081 and the current owner restart |
| Scope | AMR-03.1.2 completion and durable-start observation prerequisites |

## Decision

Prepare the existing six-member public-GET artifact bundle through the borrowed
artifact builder introduced by Decision 0096. This is closed, pure normalization
of complete descriptions after an attempted effect, not an authority issuer,
producer attestation, native admission or a new publication owner.

Reuse the original call validator and preserve the exact approved argument bytes.
Require consistent registered tool, packet, reservation, terminal transaction,
successful receipt,
native-result material and parent interval before constructing output. Keep the
existing call, packet, material, response-frame, result and bundle formats. Append
four source members, then the canonical ToolResult, then the bundle referencing
that result. All six members remain Report artifacts with their existing media
types. Obtain every reference from the same borrowed builder; introduce no second
identifier allocator or guessed timestamp.

Seal the complete immutable execution through that same builder exactly once.
Check its terminal event against the original start, receipt, full result digest
and native parent interval before returning the description for canonical commit.
A successful native disclosure retains StateChange::Changed. Nothing may rewrite
the ToolResult after terminal sealing, publish before the existing terminal commit,
or retry the effect after a preparation or publication failure. Partial bundles
remain unusable under the unchanged canonical reader.

Expose the existing canonical research begin operation with a durable-start
observer. Route both the compatibility entry and the observed entry through the
same private preflight/checkpoint owner. Notify only after durable consumption and
start publication, before driver invocation. Observer failure prevents invocation,
keeps spent accounting and requires reconciliation without automatic replay.
Preserve the existing locked plan, reservation, grant and dual-proof checks.

## Verification boundary

Batch the observer, normalizer, shared validation changes and their tests before
one supply-chain and evidence regeneration. Test full bytes and dependent references,
call/receipt/reservation/native-result substitutions, malformed or reordered builder
references, terminal timing and identity, exhausted preparation, ignored or partial
completion, observer refusal and canonical reopen/no replay. Use the existing
canonical store and payload owner to exercise the consumer with clearly synthetic
producer and artifact-builder fixtures.

All ordinary runtime modes continue rejecting NetworkAccess. Their existing state
change validation is unchanged. Do not substitute a read or write operation to
obtain a positive coordinator case. Separate synthetic bundle/store checks from
real coordinator integration, native transport, provider and model qualification.
Decision 0084's native adversarial prerequisite remains mandatory before host or
provider activation. This increment changes no frozen wire schema, enables no
outbound tool and closes no task, independent-review, human-only or release gate.

# Decision 0090: Runtime Correctness Callback Consistency

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0054, 0061 and 0081; current owner restart |
| Scope | Existing runtime coordinator correctness callbacks |

## Finding

A component regression changed a valid tool result after the trusted fixture called
the terminal-event builder. The coordinator accepted the original event alongside
the substituted result and reached success. The event bound different result bytes.
This was a deterministic fake-boundary reproduction, not an exploited native host
or a model campaign. The same callback/return pattern exists for permission
evaluation, protected resolution and checkpoint publication.

## Decision

Bind each returned value to the exact input observed by its existing event builder,
in addition to the current exact event, contract and context checks. Reject an
omitted, repeated or inconsistent callback before accepting returned correctness
events, publishing output artifacts, advancing continuation state or verifying
completion. An adapter cannot erase a refused duplicate callback by ignoring its
error and returning the first event.

For tool execution, bind the complete canonical ToolResult, receipt identity and
digest, output discriminator and ordered artifact candidates. Retain only bounded
metadata and full-content digests during the callback; do not duplicate potentially
large payload buffers. For permissions and checkpoint metadata, compare the complete
typed values. Preserve the existing event and public wire schemas, host transaction
owner, authority checks, resource limits and immutable evidence inputs.

The coordinator cannot undo an effect or durable event already committed by the
trusted host. A mismatch is a boundary failure requiring canonical reconciliation;
it must not be reported as successful cancellation, rollback or verified completion.
Latch the detected inconsistency in this coordinator instance so another call cannot
advance it; recovery must reopen the existing canonical state through its current owner.
The existing start observer remains visible after its durable commit. No second
journal, execution loop, grant issuer or publication owner is added.

## Verification and remaining scope

Exercise result, receipt, output discriminator and artifact substitution across
successful and non-successful terminal outcomes. Test changed permission results,
protected decisions and checkpoint publications, omitted/repeated callbacks, and
the unchanged valid paths. Check that refused data does not enter artifact or
continuation output and the effect is never retried. Retain the original failing
reproduction. Batch the applicable evidence renewal after all source changes.

This fixes a coding and research integration prerequisite. It does not activate a
research provider or complete native/model, independent, human or release gates.
Research publication still needs an explicit integration design: its existing
reference-bearing result must be bound before the terminal event, while actual
artifact publication and its events remain with the current coordinator afterward.
Do not bypass that ordering with synthetic receipts or another artifact owner.

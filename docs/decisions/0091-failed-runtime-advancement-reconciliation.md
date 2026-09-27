# Decision 0091: Reconcile Failed Runtime Advancement

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0054, 0061 and 0081; current owner restart |
| Scope | Failure continuation in the existing runtime coordinator |

## Finding

After a synthetic tool completed, the existing terminal-publication fault fixture
returned `Uncertain`. A second call with a valid cancellation signal returned a
cancelled outcome without the tool receipt. The first failure did not establish
whether canonical publication completed. Decision 0090's callback mismatch latch
did not cover a boundary which returned an error instead of a mismatched value.
The reproduced failure is a component observation, not native or model acceptance.

## Decision

Extend the existing in-memory reconciliation latch to every error returned while
advancing an invocation. Preserve the first exact error. Further advancement,
including cancellation, must refuse before touching events, resources, models or
tools. Reopen and reconcile through the existing canonical owners; do not retry
an uncertain transaction, invent a receipt or relabel its outcome as cancellation.
This includes artifact, journal, flush, checkpoint and callback errors, and the
development checkpoint interruption that explicitly exercises process restart.

Validate an incoming protected approval against the current pending challenge and
trusted clock before advancement. A malformed or stale client response must not
consume the pending challenge or poison otherwise valid state. A later valid
response still passes the native boundary's independent current-state checks.
Ordinary approval waits and canonical terminal outcomes remain successful API
results. Handled model/tool failures still retain their existing terminal semantics.

Use the existing coordinator and its private flag, not another transaction owner,
event family, journal, public status or recovery API. Read-only diagnostics remain
available. This conservative stop does not claim rollback, undo external disclosure,
or establish the final disposition of a failed publication. Source campaigns and
the actual Linux, model, independent, human and release gates remain separate.

## Verification

Retain the original failing uncertain-publication/cancellation reproduction. Cover
first-error preservation, repeated advancement and cancellation after faults at
permission, tool start/terminal, artifact, checkpoint, terminal journal and flush
boundaries, including errors after publication. Verify unchanged canonical history,
receipt/artifact references, resource accounting and model/effect counts after the
failure. Verify malformed approval preservation, ordinary waits, denial, successful
completion and canonical checkpoint reopen. Batch evidence after all source edits.

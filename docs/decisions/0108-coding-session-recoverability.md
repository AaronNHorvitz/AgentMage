# Decision 0108: Coding Session Recoverability

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0081, 0088 and 0107; the implementation amendment; current owner restart |
| Scope | AMR-04.3 and CAP-13 honest rollback |

## Findings

The coding runtime retains a sealed change record for each successful structured
write, and a separate tool proposes a fresh inverse write for one record. Nothing
states which of a session's effects can be reversed. Created files have no admitted
inverse. Commands, validation runs and uncertain effects are outside the runtime's
knowledge. CAP-13 requires declaring what is recoverable and surfacing the rest for
reconciliation, never claiming that a reset undid them.

## Decision

Add a host component that classifies a session's effects from records the trusted
runtime supplies. For each file, writes are examined newest first against the file's
current digest. A write is recoverable when the bytes expected after inverting every
newer write equal its postimage; inverting it then expects its preimage. A write whose
preimage is already present counts as already reverted. Any other state, including a
missing file or a human edit, is a conflict. Every older write on that file is then
a conflict too, because an inverse write would overwrite other edits.

Created files are not recoverable by an admitted operation, because deletion is not an
inverse the runtime offers. Commands and uncertain effects are external or uncertain
and need user reconciliation. Foreign, tampered or duplicate records are refused. The
sealed report lists recoverable writes newest first. It says the session is fully
recoverable only when every effect is recoverable or already reverted.

As amended by [Decision 0109](0109-review-fixes-for-component-batches.md), every
identity follows the change-record identifier rule, created-file paths follow the
record path rule, and the rendered text is capped with a distinct empty-session
summary. A record's seal is an unkeyed digest that proves consistency, not origin.
The integration must build the effect list from the canonical artifact store and
receipts it owns.

The component grants and performs nothing. Restoration still uses the existing
rollback tool with fresh authority and approval, one record at a time, in the reported
order. User-facing text describes how each effect could be reversed, if at all, and
never calls an effect undone.

## Verification boundary

Host unit tests cover write chains, human edits, missing files, already-reverted and
broken chains, created files, commands, uncertain effects, independent files, foreign
and tampered records, duplicates, bounds and deterministic digests. No runtime path
builds the effect list from actual receipts yet, and no CLI shows the report. That
integration, actual-process evidence and independent review remain open.

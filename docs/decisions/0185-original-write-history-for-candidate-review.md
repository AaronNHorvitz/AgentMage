# Decision 0185: Original write history for candidate review

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decisions 0183 and 0184 |
| Scope | Existing-file native retry history in the Linux coding owner |

Retain an ordered, bounded sequence of the original checked native write proofs
through prepared edits, runtime handoff and published runtime completions. Each
proof keeps its original shadow change set, approval binding and checked final
application-chain digest. Take previous history out of the owner before native
dispatch. Only successful checked completion may append and return it. Uncertain,
failed or unpublished dispatch loses that history; saved receipts cannot restore
it. Denial and expiry before dispatch preserve it, as before. Creation, original
transfer and poisoned recovery still retire it.

Project complete existing-file history into one inert net diff for the same
candidate-review planner. Require consecutive preimage/postimage continuity on
each path, consistent artifact and purpose labels, complete current edit lineage,
fresh native source bytes and unchanged held roots. Preserve every original
transaction in order. The composite has no singular approval or application
receipt: its change-set digest identifies the complete history, and cannot be
used as an approval for a new write. Original transaction digests remain separate.

Limit retained history to sixty-four transactions and eight MiB of complete diff
text, within the owner's existing sixteen-path bound. Never truncate or summarize
missing proof. Duplicate transactions, nested histories, inconsistent breadth,
net-zero paths, new-file lineage and incomplete chains refuse review. Final
purpose identities are path-ordered `history-N`; single-transaction identities
and serialized fields retain their existing shape.

Review packets use schema version 2 for the new history representation. Existing
in-memory packet builders and verifiers agree on that version; version 1 packets
must be rebuilt from original live proof. No saved packet restores ownership.

This extends original write provenance only. Independent reviewer and artifact
provenance, candidate execution, separately approved Git effects, native/model
measurement and durable recovery remain open. No new route, terminal command,
model request, grant, trust exception, row or sub-row is introduced or closed.

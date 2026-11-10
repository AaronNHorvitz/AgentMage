# Decision 0191: Confine non-signing local-commit I/O

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decisions 0189 and 0190 |
| Scope | Non-signing operations in the existing dormant signed-commit adapter |

Route branch lookup, raw commit reads and the final task-branch compare-and-swap
through the existing closed offline Git supervisor. Capture the selected roots
once and keep one immutable kernel-verified plan and thirty-second session
deadline. Only that branch's expected parent, one returned commit object and
the exact compare-and-swap are admitted, in order. Failed, incomplete, cancelled,
oversized, stale or malformed observations end the session; no direct retry.
Keep source read-only and the existing exact metadata/owned-root projections.

Verify raw commits against exact singleton tree, parent, author and committer
headers, one signature block and the exact approved message bytes on stdin.
Reject additional parents or headers. Verify exactly one GnuPG `VALIDSIG` status
record with the complete pinned fingerprint and no failure/revocation/expiry
record; prose and fingerprint prefixes cannot satisfy this check. Keep signed
Git stdout and stderr separate, disable replacement objects, pin both signer
configuration keys and supply message bytes through a sealed memory descriptor.
After worker dispatch or signing begins, failures report failed execution and
unverified cleanup; retained last-observation fields cannot become a verified
postflight receipt. A failed raw read or content check stops before verification.

This does not confine the external signer. Signer inspection, signed commit
creation and signature verification still use the dormant direct subprocess
boundary. They require separate confinement, whole-unit descendant cleanup and
exact authority composition before activation. The shared session deadline and
sealed input do not solve inherited descendant-pipe cleanup. No product route,
candidate authority, signer authority, approval or publication capability is
introduced. Original kernel approval and final preservation reconciliation stay
required. Reviewer/artifact provenance, durable recovery, native admission and
model measurements remain open. T-303/T-306 remain open pending independent
review and the executable workflow.

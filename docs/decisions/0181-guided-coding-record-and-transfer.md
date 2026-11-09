# Decision 0181: Guided coding through retained records and original diff review

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decisions 0178–0180 |
| Scope | Terminal composition over existing authenticated routes |

Add `endpoint-run-apply-saved` to the existing terminal client. Reuse the
original guided endpoint steps, record writer and original-transfer commands
under one elapsed-time limit and signal latch. Select only a controlled run.
Retain separate confirmations for each tool, every model disclosure, the record
save, terminal release, two-root source reading and exact original diff apply.
A declined confirmation or uncertain response stops further requests. Bind each
guided disclosure token to its local step number, preventing a queued previous
token from approving later disclosure. Single-shot commands keep their token.

Save and synchronize the complete terminal record before offering release.
The existing private archive directory must be canonical and outside both
selected checkouts. Apply this restriction to the existing saved-transfer
command too. Recheck placement before saving and before release, including
after the release confirmation. These metadata samples do not lease ancestry
against concurrent directory moves. Standalone record saving retains its
explicitly chosen destination policy.

Show the observed terminal outcome alongside the transfer. A failed, declined
or cancelled run may contain useful separately approved edits; transferring
those edits certifies neither its task nor tests. Preserve the saved failure
record. No command restarts a service, restores execution from a record, grants
blanket permission, commits or pushes. Native trust and parent acceptance remain
open; synthetic model/creation fixtures do not qualify a coding model.

# Decision 0160: Transfer verified new files to the original checkout

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-07 |
| Authority | Decisions 0153, 0154, 0156 and 0158; T-303/T-306 |
| Scope | Live native creation lineage and separately approved absent-file transfer |

Extend the existing process-held worktree owner to retain creation lineage from
its native controlled-filesystem driver. Hold the same owner lock used for
existing-file edits through creation, reconciliation and terminal publication.
Only an actual successful native application and its matching verified,
committed transaction can establish absent-before provenance. Bind operation,
path, postimage, mode, transaction and consumed grant. Never infer creation from
a zero digest, an untracked file, a caller receipt or a successful model reply.
Failed publication, restoration and an unverified result establish no lineage.
Keep the existing combined sixteen-path bound.

Later approved edits preserve a creation's absent-before provenance and original
creation mode until transfer. On explicit original-read consent, prepare an inert
complete creation preview using the existing filesystem-plan machinery. Observe
bounded parent-directory names and bind the observed parent identities; the
destination must be absent. The native transaction resolves and holds parents
again and refuses a changed parent or sibling snapshot. After separate exact
approval, use the existing single-use grants and descriptor-relative native
transaction. Refuse occupied destinations,
case aliases, unsafe paths, drift and links. Preserve unrelated files and the
Git index. Do not create missing parent directories.

Use one transaction family per original preview: existing-file edits retain the
existing shadow-write transaction, while creations use the controlled-filesystem
transaction. Refuse a mixed selection rather than performing two independently
committing transactions under one approval. Inspection identifies each eligible
path as an edit or creation so callers can choose separate previews. Both kinds
retain the current root binding, expiry, denial, uncertainty and no-replay rules.
Successful transfer consumes that path's lineage; subsequent edits start an
ordinary existing-file chain. Restart restores no ownership or lineage.

This adds no model admission, prompt, command, commit, push or publication
permission. Component tests with synthetic worktree-creation provenance and real
native file effects are distinct from native Git creation, connected model runs
and coding scores. Configured-endpoint composition, durable ownership, parent
measurements and independent parent acceptance remain open. No task, hidden
test, held-out input, threshold, evidence binding or release gate changes.

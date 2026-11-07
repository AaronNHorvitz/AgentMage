# Decision 0156: Apply approved worktree edits to the original checkout

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-07 |
| Authority | Decisions 0153 and 0154; T-303/T-306 |
| Scope | Separate exact original-checkout write approval through the existing owner |

Keep both checkout roots held by the existing local coding repository owner after
successful native worktree creation. Only successful, separately approved native
worktree edits establish transferable source paths. Track each path's first
preimage and latest postimage, with at most sixteen paths. Intermediate edits
must match the prior postimage. A dispatch without a committed result removes
those paths' transfer eligibility. No caller-supplied root or receipt can adopt
ownership or create edit provenance.

After explicit read consent, selected eligible paths can produce an inert,
complete diff for the original checkout. Worktree contents must match the last
successful postimage; original contents must match the first preimage. The
existing native writer supplies preimage checks, exact shadow changes, separate
single-use grants, atomic replacement and partial-failure reconciliation. Apply
requires a new exact approval binding both roots, creation provenance and the
whole diff. Reject drift rather than merging or overwriting intervening work.

Keep approvals, provenance and results process-only, as in the existing owned
edit path. Restart cannot adopt a tree or replay a write. Record uncertainty
before dispatch and require disk inspection after a lost reply. Later worktree
writes invalidate pending original approvals; original transfer invalidates an
older pending worktree edit. Successful transfer consumes the selected lineage;
a later successful edit starts a new chain.

This is source application, not a Git index change, commit, push, test result or
native-workflow qualification. It does not replace the canonical coding
coordinator or compose that coordinator into the manual worktree owner. Those
integration and durable-run obligations remain open. No permission, evidence,
evaluation or release threshold changes.

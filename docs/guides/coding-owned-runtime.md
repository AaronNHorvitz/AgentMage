# Live created-worktree ownership in the coding coordinator

The Linux repository owner and existing development coding coordinator now have
an in-process ownership handoff. This is an embeddable library path; the HTTP
native-host launcher still uses a separate process and does not call it. A
complete manual-create → native-run → original-apply API workflow remains open.
See [Decision 0158](../decisions/0158-live-worktree-authority-in-the-native-coordinator.md).

After successful native worktree creation, show the owner's held worktree root
identity and obtain explicit handoff consent. Consume that same owner through
`LinuxCodingRepository::into_runtime_worktree(expected_root_sha256)`. The returned
`LinuxCodingWorktreeLease` is process-only, cannot be decoded from input and
confers no read, model, command or write approval. Pending manual decisions are
discarded. Keep one lease for explicit recovery; attach it to the existing
`CodingDevelopmentRuntimeFactory` before preparing its first session with
`attach_owned_worktree`. Its independently validated activation must select the
exact created path. It still needs the ordinary private roots and explicit
[linked metadata activation](coding-linked-worktrees.md).

The factory carries a lease into each native boundary. Existing-file structured
edits, selected hunks and rollback use the unchanged native atomic writer under
the same kernel grants. Its wrapper binds successful native writes to the held
creation owner. The owner is locked through transaction reconciliation and
terminal publication. Concurrent writers refuse. A lost publication or uncertain
write removes those attempted paths' eligibility for original transfer.

After ending and releasing every coordinator and dropping the factory, call
`try_into_repository` on the retained lease. If references remain, it returns the
lease intact; do not create a new owner or adopt a path to bypass that refusal.
The recovered owner keeps its original held roots, creation receipt and
cumulative successful edit lineage. It can prepare a new original-checkout diff
after read consent, followed by a separate single-use apply approval. Previous
manual approvals cannot be reused. Restart cannot recover the lease or lineage.
Poisoning clears the created ownership and pending decisions before recovery.

New-file transfer is not supported by this bridge. Tests use synthetic creation
provenance with real held directories, kernel grants and native atomic writes;
they do not prove native Git creation, coordinator execution or inference.
The complete native host still encounters the deferred executable-ownership
blocker in this lane. No new coding score or model admission is established.

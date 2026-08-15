# Repository Worktree Handoff and Recovery

## Stop Conditions

Stop the operation and retain the worktree when any of these conditions exists:

- manifest identity changes after preview or grant approval;
- a user file, index, branch, ref, note, stash, tag, reflog, config, hook, filter, lock, or active checkout changes unexpectedly;
- the worktree is dirty, inaccessible, missing, moved, detached unexpectedly, or process-active;
- a source preimage, rename source, destination absence, task owner, branch, base object, or grant no longer matches;
- Git exits ambiguously, the parent or a descendant survives cancellation, postflight collection fails, or an effect is uncertain.

Never run reset, clean, checkout/restore discard, automatic stash, force removal,
manual lock deletion, or an unreviewed ref update as recovery.

## Recovery Record

Retain the closed worktree ownership record together with:

- the last verified preservation-manifest digest;
- operation-plan digest and invocation identities;
- repository transaction, authority transaction, and attempt identities;
- pre-effect and best-known terminal manifest digests;
- stable failure code and cleanup status;
- content-minimized owned-path, process, resource, and retention observations;
- an explicit `recovery_required` disposition.

Do not retain ignored-file content, credentials, private-key paths, raw config values,
command output, or unrelated user content merely to support recovery.

## Resume

1. Reopen the repository and owned worktree through the platform path boundary.
2. Recollect a complete current preservation manifest.
3. Reconcile every field against the last verified manifest and recovery record.
4. Verify task, owner, source object, branch, grants, owned files, and process state.
5. Create a fresh plan and preview. Prior approval and grants are stale.
6. Continue only after a new exact grant is issued and consumed.

## Handoff

A user handoff is explicit. Change the ownership record to `handed_off`, retain its
last exact identity, and remove all automatic cleanup eligibility. AgentMage may
observe the handed-off worktree only under a fresh read grant. It cannot reclaim,
remove, reset, or transfer that worktree implicitly.

## Cleanup

Cleanup can begin only from `cleanup_eligible`. Reverify clean status, zero live
processes, retained recovery, exact path identity, and unchanged manifest immediately
before grant consumption. Run non-forced `git worktree remove`, recollect the manifest,
verify only the owned worktree registry changed, then move the record to `removed`.

Any failed or uncertain cleanup returns to `recovery_required`; the worktree and all
unique task data remain in place.

# Decision 0158: Live worktree authority in the native coordinator

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-07 |
| Authority | Decisions 0153 and 0154; T-303/T-306 |
| Scope | In-process ownership handoff and native existing-file write lineage |

Allow the existing manual repository owner to transfer its successfully created
worktree into a nonserializable shared lease. Compare the displayed root identity
before revalidation and clear pending manual decisions. A marker, path, digest or
caller-supplied receipt cannot construct this authority. Every clone refers to
the same owner; recover the manual owner only after all runtime references end.
Do not reconstruct ownership after restart. Poisoned ownership loses the created
worktree and pending decisions before manual recovery.

The existing development coordinator factory can attach this lease before its
first session, only for its exact independently activated workspace. Check it
before preparation, repository refresh and composition. Keep explicit activation,
metadata read consent, the existing development profiles, model admission and
ordinary tool approvals. The lease is additional creation provenance, never a
replacement for those boundaries or a new source of grants.

Wrap the existing descriptor-relative atomic writer for structured existing-file
edits, selected hunks and rollback. Hold the owner lock through the native
transaction and its terminal publication; a concurrent writer refuses. Keep the
kernel's opaque consumed-grant authorization and native revalidation unchanged.
Before an actual application, invalidate pending manual edits, commands and
original application and remove transfer eligibility for attempted paths.

Only the actual native writer's successful application followed by its matching
committed transaction, verified receipt chain and successful terminal publication
can restore eligibility. Match transaction, consumed grant, operation identities
and preimage/postimage digests. Track each path's first preimage and latest
postimage, with the existing sixteen-path limit. Failed or unpublished writes
leave their attempted paths ineligible; restoration does not silently restore
older transfer authority. Original application still checks current bytes and
requires a new complete diff and separate exact approval under Decision 0156.

This is a library composition boundary. The current HTTP native-host launcher
uses a separate process and cannot transfer this in-process lease. Wiring its
lifecycle to manual worktree creation, durable ownership recovery, new-file
transfer, commit/push and positive native/model measurements remain open.
Do not serialize a lease or adopt provenance to bridge that gap. No new agent
loop, endpoint, row, evaluation input, score or release gate is introduced.

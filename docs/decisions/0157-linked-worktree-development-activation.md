# Decision 0157: Linked worktrees in the native development host

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-07 |
| Authority | Decisions 0153 and 0154; T-303/T-306 |
| Scope | Explicit linked-worktree activation, inventory and read-only Git inspection |

Extend the existing native coordinator's disposable development activation to a
linked Git worktree. Keep ordinary-worktree marker bytes unchanged. A linked
activation requires a new explicit marker naming the per-worktree metadata and
shared metadata roots and binding their path/object identities and exact pointer
files. An old workspace-only marker cannot authorize these additional reads.
Marker generation displays setup bytes and writes nothing; installation remains
an explicit operator action.
Only the exact independently validated linked activation marker may be untracked
or ignored during inventory. Keep it denied by the source/edit map; never require
private activation paths to enter Git history. All other inventory refusals and
the existing task-branch/profile requirements stay unchanged.

Accept only the canonical absolute gitdir pointer, `worktrees/<name>` metadata
layout, exact `../..` common-directory pointer and matching workspace backlink.
Require owned, non-writable-by-others directories and bounded single-link regular
pointer files. Hold directory and pointer descriptors, revalidate before reads
and after worker completion, and refuse substitution or drift. This does not
reconstruct manual service-created ownership or edit lineage from a path.

Run linked startup inventory and later tool inspection through the existing
offline Git worker. Mount only the selected workspace and the two bound metadata
roots read-only; select the guest Git/common directories explicitly. Do not run
linked inventory through the ordinary host-side collector, where untrusted local
config includes or object alternates could reach other host files. Retain pinned
executable trust, namespaces, seccomp, resource bounds, cancellation and descendant
cleanup. The private shared worker implementation accepts startup inventory's
existing bounded internal commands; model tools retain their kernel-prepared
plans and nonforgeable launch permits.

Linked inventory scope is deliberately read-only. Refuse it before full
preservation-manifest collection or mutating Git dispatch. Worktree creation,
branch operations, original apply and push retain their existing authority owners.
This batch adds no grant, auto-approval, frontend loop, model adapter or production
activation. It changes no evidence binding, hidden input, threshold or release
gate. Positive native workflow, manual-owner/coordinator composition, configured
endpoint admission, durable HTTP recovery and parent measurements remain open.

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
offline Git worker. Mount the selected workspace and the bound common metadata
root read-only. Keep the held per-worktree metadata descriptor for revalidation;
resolve its verified name under `/metadata/common/worktrees/<name>` so the
`../..` common-directory pointer retains its meaning. The linked Git launch
owner carries that binding and admits exactly these two projections; ordinary
Git inspection still admits exactly one. Cap each linked inventory query at the
shared supervisor's existing 4 MiB bound and refuse overflow. Do not run
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

## Review correction, 2026-11-07

The [independent core review](../reviews/0f560d5f8d40-local-coding-linked-worktree.txt)
found the initial implementation unreachable: three projections were refused by
ordinary Git admission, 32 MiB inventory limits exceeded the supervisor bound,
and the detached metadata mount could not resolve shared refs. The same-row
repair above corrects those paths without raising limits or changing trust.
Activation and admission controls are component evidence. A successful linked
inventory through the complete native worker remains unverified in this lane
under deferred T-301.1; setup instructions do not establish a usable workflow.

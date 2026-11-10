# Decision 0189: Confine approved local worktree Git

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decision 0188 |
| Scope | Existing original-owner worktree and local compare-and-swap execution |

Replace the approved local repository executor's direct Git child with the
existing offline worker supervisor. Keep its kernel-validated plan, single-use
approval, preservation comparison and post-execution reconciliation. No new
operation, permission, route or Git publication authority is introduced. Linked
read-only scopes, network operations and unsupported invocation kinds refuse.

Hold the exact original checkout, selected Git metadata and owned scratch root
as descriptors. Expose the checkout read-only and only the selected metadata
and owned root writable, with no host ancestor mounts. Preserve their canonical
path spellings inside the isolated filesystem so worktree pointers remain valid
outside the worker. Refuse collisions with the worker's runtime mounts and
private namespaces. Recheck all three held directory identities at admission,
before releasing the withheld seccomp filter and after supervision. A changed
projection or incomplete output cannot establish successful cleanup.

The worker uses the existing trusted launcher and Git verification, offline
namespaces, seccomp, bounded output and whole-unit supervision. Keep the existing
thirty-second operation deadline; the enclosing unit has a thirty-one-second
failsafe. There is no direct-process retry. The closed supervisor owner has its
own nonce prefix and exactly three projections; inspection owners retain their
existing shapes and read-only behavior. Disable replacement objects for these
local effects. Preserve exact selected metadata rather than discovering `.git`.
Mount the held Git executable read-only at `/app/git` and create a guest
`/usr/bin/git` symlink to it: installed helper symlinks used by worktree creation
can point back to `/usr/bin/git`. Bubblewrap consumes each bind-fd source, so
reuse of the same descriptor for two mounts refuses. No host executable
directory is added to the guest mounts.

The existing concurrency regression observes the admitted worker while its
seccomp filter is withheld. It stops that worker, mutates the original checkout
and resumes it before Git can execute. This updates the observation seam for
supervised execution without removing the original preservation assertions.
Synthetic results and inert plans do not establish positive native admission.
The existing native executable-ownership refusal stays unchanged.

Candidate construction, reviewer/artifact provenance, independently approved
local commit/push execution, durable recovery and native/model measurement
remain open. Dormant candidate and local-commit adapters are not activated.
T-303/T-306 remain open pending the executable workflow and independent review.

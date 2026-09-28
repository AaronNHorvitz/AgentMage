# Decision 0095: Retain Exact Native Inference Cleanup Ownership

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0054, 0061, 0081 and 0082; current owner restart |
| Scope | AMR-01/AMR-03 native model cleanup and participating-host admission |

## Findings

The native driver waits without a deadline in `Child::wait`, then checks a numeric
runtime PID's absence and removes socket/key paths without retaining the objects
created for this load. The descriptor-held inference lease ends when the owner
process exits, which does not prove its descendants stopped. These are source
findings; CPU component observations cannot qualify native inference or GPU cleanup.

## Decision

Keep the existing native driver, fixed shared lease, exact runtime/model/resource
admission and sandbox. No scheduler, new canonical store, authority grant or model
activation is introduced. Operator build/GPU reservations remain independently
mandatory. Do not change a qualified profile to fit the current lane.

Before model spawn, synchronize a bounded opaque reservation marker to the existing
fixed lease inode and its held private parent. Acquisition continues to require an
empty, private, unaliased lock; nonempty, malformed or uncertain state refuses.
Never unlink a lease or recover admission from a PID, elapsed time, process exit,
missing socket or a successful signal. Keep the same inode and flock protocol.
An older participating binary also refuses a nonempty lock. An older binary's
empty lock after an unobserved crash cannot acquire new cleanup evidence retroactively.

Bind the marker to this held file and parent; validate the complete marker again
before clearing it. Arm before the first possible process effect. A spawn failure,
partial write, synchronization error, identity drift or crash preserves uncertainty.
Only successful cleanup by this same live owner can clear its marker. The clear
occurs after cleanup evidence, never in a destructor merely because it is exiting.
If clearing fails, retain the failure; a truncation observed after proved cleanup
is not evidence of a still-running model. This marker is resource quarantine, not
an effect receipt, transaction journal or automatic recovery protocol.

Observe the directly owned launcher with bounded `try_wait` polling. Retain process
descriptors for the established exact runtime and its owned isolated PID-namespace
init, with process-generation, ancestry and namespace checks around capture. Numeric
PIDs remain observations, never signal targets or cleanup identities. Confirm both
held process descriptors report exit as well as reaping the owned launcher before
claiming cleanup. Missing namespace ownership or a failed observation remains
uncertain, including failed startup that never established the runtime. Termination
of only the launcher or one runtime process is insufficient to prove the namespace
stopped. Keep all existing live sandbox and serving-identity checks.

Use one original three-second cleanup deadline across launcher and namespace
observations; retries cannot renew it. Observe expiry before accepting success.
After an uncertain cleanup, later exit cannot turn the failed attempt into a
successful unload or free resource slot. Reject further serving operations once
cleanup starts. Destruction may attempt bounded exact-child cleanup but retains
uncertain handles and reservation rather than publishing successful cleanup.
These are cooperative bounds; kernel-stalled syscalls are not preempted.

Hold the private socket parent and created key descriptor, and capture the socket
object before changing its mode. Use fixed relative leaf names and descriptor-
relative operations. Validate owner, mode, type, link count, held/named identity
and key contents. Revalidate the original parent and every object before cleanup;
refuse and preserve substituted files, symlinks, hard links or changed contents.
An unobserved socket is never adopted during cleanup. Never remove a key by path
on a failed creation or spawn. Complete object checks precede any deletion.
An already absent leaf is acceptable only when its originally held inode is now
unlinked and otherwise unchanged, with no replacement at the original name. This
is file disposition after separate process-exit proof, not proof of process death.

These identity checks coordinate cooperative same-user owners. They do not promise
atomic check-and-unlink against an arbitrary concurrent same-UID adversary, who
already controls that private directory. Revalidate around path-based socket mode
publication, refuse drift, and make no hostile same-UID isolation claim. No generic
caller-selected deletion or process-control API is exported.

Reservation loss from logout, runtime-directory recreation, reboot or manual changes
is not proved cleanup. Preserve uncertain records and require external reconciliation
of the exact isolated processes and resource state before operator recovery. This
batch provides no reset command or authorization to delete an uncertain record.
Native qualification must cover actual namespace lifetime and recovery behavior;
CPU fixtures remain component evidence. Independent and human-only gates stay open.

Explicitly enable the existing pinned Rust I/O library's readiness feature in the
inference crate so standalone builds do not depend on workspace feature unification.
No package or version changes. Retain complete manifest, feature-inventory and
SBOM bindings; do not create a source-audit exception.

## Verification

Use actual CPU children and held process descriptors, fixed private lease inodes,
and test-owned files. Retain a failing preservation regression with complete source
bytes before correction. Test unarmed release, armed contention, owner death with
an armed marker, exact successful release, malformed/changed markers, parent/file
replacement, links and permissions. Test original cleanup deadline, late exit,
missing descendant/namespace proof, and refusal of serving after cleanup begins.
Passing observations must precede any test lifeline or fallback intervention.

Run source tests and required checks under the shared build reservation, retaining
all failures. No model process or GPU fixture is permitted outside the explicit
GPU reservation. Complete this source work unit before one full SBOM/evidence pass.
Retain actual executable startup refusals separately from positive native coding,
real-model, manual-user, independent-review and release acceptance.

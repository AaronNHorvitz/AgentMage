# Decision 0089: Development Harness Process Ownership

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0054, 0061, 0063 and 0070; current owner restart |
| Scope | Existing disposable coding developer wrapper |

## Finding

A rejected start can unlink another invocation's run record without launching a
child. The wrapper also checks a recorded PID, executable and loose root argument
before sending a signal by PID. It does not bind the process's kernel start time;
the recorded wall clock is diagnostic only. These are wrapper lifecycle defects,
independent of the lane's unavailable native confinement prerequisites.

## Decision

Keep the existing wrapper and run-record owner. Reserve its file exclusively before
launching a child. Hold the original directory and file descriptors, read bounded
bytes without following a final symlink, and check private ownership, file type,
link count and stable metadata. Cooperative record reads and updates use the file
lock. Cleanup checks both the held file identity and its expected contents. A
missing, replaced or changed record is preserved and reported as uncertain cleanup;
it cannot yield a successful wrapper result. Attempt all owned-resource cleanup
even when an earlier cleanup step fails. Publish a terminal result only after
cleanup succeeds.

Use a closed versioned record with a reserved phase and an identified-child phase.
Bind the latter to the Linux boot ID, UID and process start ticks, executable and
each exact disposable root argument. Preserve incompatible legacy records and
stale reservations; do not adopt, rewrite or automatically delete them. Diagnosis
distinguishes a reservation or stale record from a root ready for a new run. Use a
fresh disposable root when the previous invocation's ownership cannot be established.

Cancellation opens a Linux process descriptor before revalidating the recorded
identity and current record. Signal through that descriptor only. Missing support,
process exit, identity mismatch or uncertain observation refuses cancellation;
there is no PID-only fallback. Keep the native-state-key prerequisite. The parent
still cleans up its actual child through the existing subprocess handle.

This is cooperative lifecycle ownership inside the existing private developer
boundary. It is not protection from an adversarial process with the same UID that
can rewrite private state or replace paths between filesystem calls. Native Rust
admission, peer authentication, grants, tool supervision and confinement remain
the authority boundaries. No runtime owner, model profile, production activation,
schema migration or release permission is introduced.

## Verification and limits

Exercise rejected starts, simultaneous reservations, launch/publication failures,
replacement and mutation before cleanup, malformed and aliased records, changed
process identity, process exit and descriptor-only signalling. An owned synthetic
child can verify Linux descriptor mechanics; it cannot establish an AgentMage
CLI workflow, model qualification or independent acceptance. Retain earlier failed
reproductions and report these categories separately. The actual Linux coding
workflow remains subject to its native prerequisites and complete campaign.

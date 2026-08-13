# Decision 0035: Bind the Docker Runtime Peer Identity

| Field | Value |
|---|---|
| Status | Accepted corrective implementation decision |
| Date | 2026-08-13 |
| Scope | Exact kernel-side peer identity in production Docker topology collection |
| Implements | Completes the process-identity prerequisite for Sprint 9 Sub-task `9.2.2.1` |
| Amends | Decision 0033 and Docker collector protocol version 2 |
| Preserves | Explicit targets, held PIDs, exact guard identity, authenticated IPC, one-use observations, zero egress, and no fallback |
| Does not implement | Inference, model admission, release support, or Story 9.2 completion |

## Context

The production collector held and revalidated the declared runtime PID and
start time, UID, and primary GID. Its administrator baseline did not include
the runtime executable or cgroup digests, although Decision 0033 requires exact
runtime process identity. A same-user process could therefore satisfy the
collector-side runtime fields without matching the process selected by the
guard bootstrap.

## Decision

1. Collector protocol version 2 adds exact nonzero runtime executable and
   cgroup SHA-256 identities to the administrator baseline.
2. The collector hashes the held runtime process executable and complete
   `/proc/<pid>/cgroup` record and compares both before deriving an observation.
3. Missing, malformed, substituted, stale, or mismatched runtime identities
   return the content-free collector input refusal before admission.
4. The live KVM harness must retain the runtime PID, start time, executable,
   cgroup, UID, and GID identities and show they match the guard bootstrap and
   collector baseline.

## Verification

- Exact fixtures include nonzero runtime executable and cgroup identities.
- Omission and mutation of either field fail closed.
- Collector self-check and package descriptors identify protocol version 2 and
  preflight contract version 3.
- Existing no-live-Docker evidence is regenerated before reuse.

## Consequences

- A same-user process is no longer interchangeable with the declared runtime
  peer during topology admission.
- The production observation schema changes incompatibly and is deliberately
  versioned rather than accepted through fallback parsing.
- Story 9.2 remains open pending live topology, hostile reachability,
  independent control disablement, product-security evidence, and review.

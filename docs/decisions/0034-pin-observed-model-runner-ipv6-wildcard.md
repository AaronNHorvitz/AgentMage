# Decision 0034: Pin the Observed Model Runner IPv6 Wildcard

| Field | Value |
|---|---|
| Status | Accepted corrective implementation decision |
| Date | 2026-08-13 |
| Scope | Exact raw-listener family for the pinned Docker Model Runner image |
| Implements | Corrects the prerequisite for Sprint 9 Sub-task `9.2.2.1` |
| Amends | Decisions 0030 through 0033, Docker runtime and guard profiles, and preflight contract version 3 |
| Preserves | Private route-free namespace, loopback-only reachability, zero host listener, exact image identity, zero egress, guarded kernel path, and fail-closed drift refusal |
| Does not implement | Model admission, inference quality, workstation Docker installation, release support, or Story 9.2 completion |

## Context

A disposable execution of the exact pinned Model Runner image exposed one raw
listener in `/proc/net/tcp6`: the all-zero IPv6 address on port `12434`. There
was no corresponding `/proc/net/tcp` listener, non-loopback interface, or
route. The socket is dual-stack, so the dedicated guard can still connect to
`127.0.0.1:12434` inside the same namespace.

Earlier decisions described this immutable listener as IPv4 `0.0.0.0:12434`.
Admitting that description would make the production collector reject the
actual pinned image or require evidence to misstate the observed socket.

## Decision

1. The one admitted raw listener is exactly IPv6 wildcard `[::]:12434` inside
   the runner-and-guard private network namespace.
2. The collector reads both `/proc/net/tcp` and `/proc/net/tcp6`, requires one
   IPv6 wildcard listener, and requires zero IPv4 wildcard listeners and zero
   additional raw or management listeners.
3. The guard retains `127.0.0.1:12434` as its exact connection target. This is
   admissible only because the pinned listener is dual-stack and `lo` is the
   namespace's sole active interface.
4. IPv4 wildcard substitution, IPv6 loopback-only substitution, a second
   listener, a host listener, or any non-loopback interface or route is drift
   and refuses Docker mode without fallback.
5. Preflight contract version 3 and package process-boundary version 7 identify
   this corrected listener contract. Existing evidence remains historical and
   must be regenerated before making a current Docker prerequisite claim.

## Verification

- Parser tests distinguish IPv4 and IPv6 wildcard records.
- Production-derivation tests admit exactly one IPv6 wildcard record and reject
  IPv4 substitution, no listener, duplicate listeners, and exposed routes.
- Profile and descriptor identities are recalculated from the corrected files.
- Clean Fedora and Ubuntu KVM evidence must retain the raw `/proc` socket family
  together with the collector's admitted result.

## Consequences

- The contract now matches the immutable pinned runner instead of an assumed
  address family.
- The raw wildcard remains unreachable from the host, LAN, ordinary
  containers, and every process outside the private namespace.
- Story 9.2 remains open until all live topology, hostile-reachability,
  control-disablement, product-security, and independent-review gates pass.

# Decision 0032: Fail-Closed Docker Topology Preflight

| Field | Value |
|---|---|
| Status | Accepted implementation boundary |
| Date | 2026-08-13 |
| Scope | Complete pre-activation Docker topology observation, exact drift classification, and terminal refusal contract |
| Implements | Sprint 9 Sub-task `9.2.1.4` |
| Amends | Decisions 0030 and 0031 by requiring one fresh complete observation before Docker mode can become representable; Decision 0033 advances the contract for the immutable wildcard bind and production executables |
| Preserves | Native `llama.cpp` as the Linux reference, Docker unavailable by default, no automatic fallback, exact identities, no Docker authority for the runtime user, and no support claim without live evidence |
| Does not implement | A privileged observation collector, Docker installation or launch, a live daemon inspection, model activation, inference, hostile reachability execution, or release approval |

## Context

Static profile validation cannot prove that a running daemon, socket, container,
namespace, image, cgroup, or firewall still matches the approved topology. A
partial inspection is also unsafe: validating an image digest while omitting
socket ownership or egress state must not admit Docker mode.

The preflight must have one deterministic decision point before a raw-endpoint
permit or inference request exists. It must retain only content-free identities
and refusal classes, expose no observed paths or credentials, and make no
attempt to select a weaker Docker or native configuration after a mismatch.

## Decision

1. Docker preflight contract version 2 is compiled into package process-boundary version 6. Version 2 distinguishes the immutable runner wildcard bind from the guard's loopback connection and incorporates complete namespace-interface, route, guard-socket, and collector-protocol observations. The packaged adapter remains self-check-only with Docker unavailable, zero enabled models, and no inference operation.
2. A separately configured baseline binds exact runtime and guard profile hashes, trusted collector executable, daemon executable, Docker socket object and group, ordinary runtime UID, dedicated guard UID and executable, guard cgroup, and private network namespace.
3. One observation is admissible only when it is complete, fresh, non-replayed, bound to a nonzero session identity, and produced by the exact configured collector executable.
4. Daemon privilege requires the exact daemon executable at UID `0`, rootless mode disabled, the exact ordinary runtime UID, and no runtime-user membership in the Docker socket group.
5. Socket ownership requires the exact socket object, Unix-stream type, owner UID `0`, exact nonzero Docker group, and mode `0660`.
6. API binding requires one raw `0.0.0.0:12434` listener only in the exact private namespace, a guard connection target of `127.0.0.1:12434`, exactly one active interface named `lo`, zero non-local routes, and zero host, non-loopback-interface, foreign, or management listeners.
7. Container reachability requires exactly one runner and one exact-identity guard in the same private namespace, with zero host routes, bridge routes, or foreign reachable peers.
8. Image identity requires the exact runner and model OCI manifest digests. Mutable-tag admission and runtime repull are prohibited.
9. Resource state requires zero prohibited mounts, one private runtime tmpfs, an immutable model content store, no privilege, no added capabilities, no-new-privileges, read-only root, exact cgroup ceilings, zero swap, and one inference slot.
10. Zero-egress state requires do-not-track, acquisition and registry access disabled, no proxy or Domain Name System path, default-deny firewall state, zero egress interfaces, and zero outbound bytes.
11. Evaluation order is stable and returns only one content-free refusal class: profile, observation, daemon privilege, socket ownership, API binding, container reachability, image identity, resource limits, or zero egress.
12. A failed or incomplete preflight is terminal for Docker mode. It cannot silently activate Docker, relax a control, select another Docker topology, or claim native-mode success.

## Verification

- Rust tests admit only the complete exact observation and mutate every declared field independently.
- Tests verify stable refusal ordering and distinct content-free error codes for all nine classes.
- The admission proof is non-clonable, non-serializable, and constructible only inside the validator module after all checks pass.
- A retained source-bound report records the contract version, refusal classes, mutation dimensions, Docker-absent host state, and explicit live-enforcement limitations.
- Existing clean Fedora and Ubuntu package, platform-control, parity, strict-local, supply-chain, documentation, and traceability gates must remain green.

## Consequences

- A future platform collector has one complete closed observation schema and cannot omit a control while reporting success.
- Runtime drift is an activation refusal, not a warning or degraded mode.
- The validator alone is not live Docker evidence. Story 9.2 remains open until clean native inspection, hostile reachability, and independent control-disablement lanes execute.

# Decision 0014: Kernel-Issued Effect Mediation

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-11 |
| Scope | Cross-crate effect authorization, platform dependency direction, and raw-effect visibility |
| Resolves | `RM-008` |
| Amends | Decision 0004 dependency direction and Decision 0013 Phase 5 boundary |
| Preserves | Canonical operation taxonomy, kernel authority ownership, accepted product scope, and immutable historical evidence |

## Context

Decision 0013 established one kernel-owned authority transaction but kept its
worker driver private. The Linux sandbox runner, Secret Service client,
configuration filesystem mutations, and Unix listener still had public or
shell-reachable raw entry points. Correct transaction ordering alone could not
prevent a shell or future capability from bypassing the transaction.

The effect boundary must make bypass fail structurally. It must also preserve
safe observation APIs without turning an observation result into authority.

## Decision

### Permit and Transaction Boundary

1. `AuthorityTransactionCoordinator::execute_effect` is the only public path
   that can issue an effect authorization.
2. `AuthorityTransactionRequest` has private fields and one production
   constructor. Cancellation and crash injection remain private test controls.
3. `EffectAuthorization` has private fields and no public constructor. It does
   not implement `Clone`, `Copy`, serialization, or deserialization.
4. The permit borrows the exact transaction request and consumed-grant digest.
   It carries the transaction, attempt, operation, and exact validated tool-call
   identities and cannot outlive the coordinator invocation.
5. `EffectDriver::execute` consumes the permit by value. A second use is a
   compile error. A driver receives the permit only after validation, policy
   evaluation, grant consumption, attempt recording, and launch commitment.
6. Driver construction is inert. Creating a driver or an observation object
   does not grant authority and performs no represented effect.

### Platform Direction

Decision 0004 is amended so an effect-bearing platform adapter may import the
narrow kernel-engine mediation API. The kernel still cannot import a platform,
capability pack, or shell. The current inward layers are contracts, kernel,
effect adapters, shells, packaging, and release orchestration.

The host may construct an exported platform driver, but it cannot call the
driver without an unforgeable permit. Platform raw-effect methods and types are
not exported. Read-only capability packs continue to depend only on contracts.

### Current Effect Inventory

| Surface | Raw entry point | Public mediated path |
|---|---|---|
| Configuration files | Migration, apply, and rollback are crate-private | `ConfigurationEffectDriver` |
| Linux sandbox | `LinuxSandboxRunner::run` is private | `LinuxSandboxEffectDriver` |
| Linux Secret Service | Probe, store, lookup, and clear are private | `LinuxSecretEffectDriver` |
| Linux Unix listener | Listener construction and acceptance are module-private | None; production socket creation remains unavailable |

A future network, provider, repository, shell-command, or delivery adapter must
implement the same consuming driver boundary before it becomes callable. It
must not restore a raw public method as a temporary integration path.

### Observation Boundary

Path inspection, held-object reads, process and socket inventory, manifest
verification, limits, configuration parsing, configuration comparison, and
content-free result access remain separately callable where they do not change
external state or launch an active operation. Observation types have no method
that constructs or converts into `EffectAuthorization`.

## Consequences

- Shell and capability source cannot call the represented platform and
  configuration effects without the coordinator-issued permit.
- Platform adapters depend inward on the kernel mediation interface. This is an
  intentional narrow exception to the earlier contracts-only adapter rule.
- A permit prevents API bypass and duplicate use. It does not make an arbitrary
  third-party driver trustworthy; admitted driver provenance remains a build
  and platform-activation responsibility.
- Phase 6 still owns canonical grant-target parity and exact held-object worker
  isolation. Phase 7 still owns encrypted durable transactions and restart
  recovery. This decision does not claim those guarantees.
- The current Unix listener remains unavailable to product code until a
  mediated network operation and its exact scope are implemented.

## Verification

- Compile-fail tests reject permit construction, cloning, reuse, raw
  configuration mutation, raw sandbox execution, and raw Secret Service calls.
- Rust tests exercise exact public transaction execution and reject
  call/context identity drift before state retention.
- The effect-boundary validator inventories all permit consumers, raw API
  visibility, internal crate edges, and shell/capability process, socket, and
  filesystem-mutation patterns.
- Mutation tests reopen each representative boundary and prove the validator
  fails.
- Dependency policy and manifest-report tests verify the inward platform edge,
  absence of a kernel reverse edge, and graph acyclicity.

## Approval Gate

The user approved this decision, the Phase 5 local commit, and entry into Phase
6 on 2026-08-11. That approval did not authorize a push or a Phase 6 commit.

# Platform Release Manifests

Files under `v1/` preserve the historical public contract fixtures. They are not
accepted by the Phase 8 production verifier, are not release manifests for a
shipped AgentMage build, and carry no release claim. Their package digests and
mechanism identities remain deterministic test values.

The production wire contract is version 2 and is described under `v2/`. No
version-2 signed release artifact is committed because there is no supported
package or production signing ceremony. Unit and conformance tests generate
ephemeral synthetic signing keys and exact manifests in memory.

A releasable manifest must replace every fixture identity with evidence from the
exact package and environment under review. It must then pass the same validator,
platform execution, package verification, and release gates. Evidence from one
platform cannot satisfy another platform's gate.

The trusted manifest verifier is implemented in the kernel. Package generation,
trusted-key distribution, signing ceremony, rollback policy, and end-user
installation remain later release work. Runtime code may activate an adapter
only through the shared independent fail-closed startup boundary.

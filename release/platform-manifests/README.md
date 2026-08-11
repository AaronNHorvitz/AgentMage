# Platform Release Manifests

This directory freezes the version 1 platform-manifest contract before any
release package exists. Files under `v1/` are public synthetic contract fixtures,
not release manifests for a shipped AgentMage build. Their package digests and
mechanism identities are deterministic test values and carry no release claim.

A releasable manifest must replace every fixture identity with evidence from the
exact package and environment under review. It must then pass the same validator,
platform execution, package verification, and release gates. Evidence from one
platform cannot satisfy another platform's gate.

The trusted manifest loader, signatures, package generation, and end-user
installation are intentionally outside this contract-fixture directory. Runtime
code may activate an adapter only through the shared fail-closed startup boundary.

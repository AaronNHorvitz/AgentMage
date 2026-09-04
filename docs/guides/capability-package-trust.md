# Capability Package Trust and Lifecycle Guide

Sprint 78 defines a fail-closed source contract for inspectable capability packages. It does not
install, enable, update, execute, or publish a package by itself.

Each version 1 manifest binds package and source identity, signer and public-key digest, license,
seven compatibility versions, tools, skills, hooks, nine scope limits, exact dependency locks,
entry-point hashes, and declared side effects. Lists use canonical ordering and package paths are
relative without traversal.

Admission recomputes the canonical manifest digest, compares the observed source digest, validates
the license and trusted signer, requires the exact dependency lock set and compatibility tuple,
matches the public-key digest, and verifies an Ed25519 signature under the AgentMage capability
package domain. Any mismatch leaves the package unadmitted.

Install, enable, disable, update, rollback, and uninstall produce inert previews. Each preview binds
the exact before/after manifests, permission delta, and approval identity. Actions that can admit
code require a verified package. No preview applies an effect.

Filesystem roots, commands, network domains, credential identities, connectors, publication
targets, memory, CPU, and retention are admitted only when the complete requested scope is within
the task-provided ceiling. Package declarations cannot mint authority.

Native lifecycle traces, capability-inventory deltas, malicious-package runs, safe-mode startup,
and independent review remain blocked until their untouched external artifacts are available.

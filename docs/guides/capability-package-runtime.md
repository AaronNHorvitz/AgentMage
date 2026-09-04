# Capability Package Hooks, Safe Mode, and Recovery

Sprint 79 extends the inert Sprint 78 package contract with deterministic lifecycle projections. It
does not discover, download, install, enable, or execute a package.

## Threat model and controls

The package boundary assumes manifests, hooks, payloads, dependencies, endpoints, and declared
permissions are hostile until verified. Public auto-discovery, automatic download or enablement,
unsigned execution, authority broader than the current task, and alternate-endpoint bypass are
fixed prohibited states.

Hooks are ordered by phase, ordinal, and identity. Each carries a bounded timeout and explicit
cancellation policy. Every observation must retain a receipt, may not change the action
transaction, and may not suppress its receipt. Failure, timeout, or cancellation is isolated and
requires safe mode.

The local catalog names the exact package and manifest providing each tool or skill and records why
it is active. A capability is active only when the package is verified, enabled, not pending
removal, and safe mode is off. Duplicate providers fail closed.

Compatibility compares exact kernel, tool protocol, configuration, memory, storage, shell, and
policy versions. Lifecycle recovery selects either the exact prior manifest after interruption or
hook failure, or the exact target after a complete operation. Incomplete cleanup, an unexpected
committed manifest, or any kernel-state change fails closed.

Process, file, tool, network-domain, and storage-namespace inventories are sorted and compared
before and after a synthetic lifecycle. The delta must equal the complete declared delta. Removal
uses the same comparison with an empty target inventory.

The package corpus, cryptographic admission, compatibility, permission ceiling, lifecycle state,
and rollback rules are independently reviewable in the Sprint 78 and 79 reports. Native process
and filesystem observations, crash injection, full removal, safe-mode startup, and independent
review remain external evidence and are not substituted by these local projections.

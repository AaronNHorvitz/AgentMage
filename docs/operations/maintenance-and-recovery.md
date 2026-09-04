# Maintenance and Recovery

AgentMage maintenance is explicit and offline. An update, backup, restore, migration, repair, or
rollback begins only from an exact local artifact and a current preview. No maintenance path checks
a remote service automatically.

## Backup and restore

A complete backup contains versioned configuration, the separately keyed encrypted operational
store, approved knowledge, rebuildable indexes, manifests, admitted packages, retained
conversations, and audit anchors. Secret-store values are excluded. Restore always targets a new
candidate, verifies its identity and integrity, and leaves live state unchanged until a separate
approval.

## Migration and updates

Migration binds source and destination machines, platform adapters, schema steps, model manifests,
and package versions. Compatibility must pass before approval. Update activation binds the signed
package, preview, staged tree, and rollback point. A changed field invalidates approval; downgrade,
tamper, interruption, or an unknown result leaves the prior state authoritative.

## Safe mode and repair

Safe mode disables browser, child-agent, connector, executable, MCP, network, package, and schedule
capabilities together. Diagnostics, evidence inspection, database integrity checks, index rebuild,
orphan cleanup, package disablement, and last-known-good proposal remain available. Recovery never
silently replaces live canonical state.

For full disk, corrupt state, missing key store, revoked credential, missing model, broken package,
lost worktree, interrupted update, or failed rollback, retain the failure evidence and stay in safe
mode. Repair rebuildable indexes only. Restore or rollback requires a verified fresh candidate and
a new exact approval. Diagnostics retain stable codes and redacted-detail digests, never raw content,
credentials, secret values, or authority.

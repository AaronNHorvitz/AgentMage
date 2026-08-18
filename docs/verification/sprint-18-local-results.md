# Sprint 18 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 18 |
| Local core result | Pass |
| Sprint result | Blocked |
| Release approval | No |

## Verified Locally

- Six exact language or dialect descriptors bind the parser runtime, grammar crate and version, upstream provenance, ABI, packaged node-type metadata, and descriptor hashes.
- Inventory construction is deterministic across traversal order and binds workspace, repository, worktree, branch or detached state, commit, policy, and freshness identities.
- Excluded, generated, vendored, and Git-ignored records reject supplied content; supported paths without authorized bytes remain visibly `content_not_read`.
- All admitted parser facts carry exact ranges and hashes. The only relationship edge is a directly observed module-to-import declaration; no target resolution is invented.
- The disposable SQLite cache uses every declared validity dimension, validates retained record integrity, misses on any key change, invalidates stale records atomically, and rolls back over-capacity writes.
- The native Fedora collector uses bounded NUL-framed Git output with credentials, filters, hooks, pagers, aliases, replacement objects, locks, and network protocols disabled. Cancellation and timeout both kill and reap the child.
- The Linux host projects regular files through held descriptors, retains symbolic links without following them, leaves Gitlinks metadata-only, rejects foreign repository identity, and keeps hostile filters inert.
- Operational-store schema version 8 retains encrypted derivative records with exact key and payload integrity, bounded retention and capacity, startup verification, tamper rejection, and current-key reconciliation.
- Rendering and source resolution require a freshly synchronized one-use permit, so citation cannot precede invalidation or reuse stale host state.
- Workspace architecture, dependency, strict-local source, supply-chain, formatting, lint, build, and product tests accept the new crate without adding undeclared network authority.

## Security Mapping

[`task-18-1-3-4-product-security-evidence.md`](task-18-1-3-4-product-security-evidence.md)
maps the seven assigned security requirements to the retained parser, inventory,
projection, cache, cancellation, and source-resolution evidence.

## Open Evidence

Native Ubuntu, macOS, and Windows repository-map campaigns are not retained. The manual parser fuzz campaign remains deliberately deferred, and independent review is absent.

Sprint 18 therefore remains blocked even though the platform-neutral core and native Fedora integration pass locally. The machine-readable source-bound record is [`local-evidence-report.json`](../../artifacts/sprints/sprint-18/local-evidence-report.json).

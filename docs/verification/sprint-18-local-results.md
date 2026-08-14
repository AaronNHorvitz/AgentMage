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
- Workspace architecture, dependency, strict-local source, supply-chain, formatting, lint, build, and product tests accept the new crate without adding undeclared network authority.

## Open Evidence

The production host does not yet collect the complete Git-aware and `.gitignore`-aware held-object projection for this capability. The packaged worker and encrypted operational-store cache are not integrated. Full host cancellation, dependency-failure, adversarial, native platform, deferred manual parser fuzz, and independent review evidence are also open.

Sprint 18 therefore remains blocked even though the platform-neutral core passes locally. The machine-readable source-bound record is [`local-evidence-report.json`](../../artifacts/sprints/sprint-18/local-evidence-report.json).

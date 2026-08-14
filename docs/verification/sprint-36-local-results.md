# Sprint 36 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 36 |
| Grant-consuming transaction coordinator | Pass |
| Receipt transition and integrity suite | Pass |
| Known partial restoration fixture | Pass |
| Fresh rollback proposal and later-change refusal | Pass |
| Native filesystem driver | Absent |
| Native race and crash matrix | Absent |
| Upstream Sprint 35 gate | Blocked |
| Independent Sprint 36 transaction review | Absent |
| Sprint result | Blocked |

## Verified Locally

- The coordinator re-observes every target, revalidates the Sprint 35 preimage and exact policy,
  consumes the existing single-use grant, and only then delegates an effect through an opaque driver
  authorization.
- The in-memory driver fixture covers complete success, pre-effect failure, ordered partial failure,
  postimage mismatch, stale preimage, malformed and uncertain reports, restoration failure, and
  fresh rollback refusal after later user work.
- Known partial effects restore the exact retained preimage bytes and are accepted only after fresh
  observation verifies every original target digest.
- Per-operation receipts form a recomputable hash chain and reject transition, identity, hash,
  ordering, reordering, and truncation mutations.
- Previewed formatter, test, build, migration, and other command labels remain unexecuted and require
  separately bounded command grants.
- The complete product gate verifies the Rust workspace, VS Code shell, strict-local source,
  hostile-network denial, effect mediation, build, tests, and documentation invariants.

## Open Evidence

Sprint 35 remains blocked, so Sprint 36's declared dependency is not satisfied. The platform-neutral
coordinator has no native filesystem driver, and therefore does not yet prove atomic replacement,
descriptor continuity, symlink and alias resistance, concurrent-writer behavior, mount changes, or
durability across process and machine crashes. The complete `S-029-ST01` and `S-029-RT01` matrices
and independent transaction review are absent.

Sprint 36 therefore remains blocked. Its retained report is generated at
`artifacts/sprints/sprint-36/local-evidence-report.json` from a committed revision.

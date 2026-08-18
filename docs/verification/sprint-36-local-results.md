# Sprint 36 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 36 |
| Grant-consuming transaction coordinator | Pass |
| Receipt transition and integrity suite | Pass |
| Known partial restoration fixture | Pass |
| Fresh rollback proposal and later-change refusal | Pass |
| Post-preview mutation matrix | Pass |
| Fedora native filesystem driver | Pass |
| Fedora descriptor-race fixtures | Partial pass |
| Mount-change and complete cross-platform race matrix | Absent |
| Crash/durability matrix | Absent |
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
- Independent post-preview mutations of target, arguments, bytes, preimage, metadata, preview,
  policy, grant nonce, workspace, and expected side effects all fail before apply and preserve the
  exact starting bytes.
- The Fedora driver performs descriptor-relative staging and atomic exchange, verifies the displaced
  preimage, and restores an exact retained preimage after a known partial transaction failure.
- Local race fixtures preserve competing state during file replacement, symlink substitution,
  directory rename, and concurrent writes before and after exchange. Consumed grant and approval
  replay never causes a second apply.
- The complete product gate verifies the Rust workspace, VS Code shell, strict-local source,
  hostile-network denial, effect mediation, build, tests, and documentation invariants.

## Open Evidence

Sprint 35 remains blocked, so Sprint 36's declared dependency is not satisfied. The Fedora native
fixtures do not substitute for a real mount-change test, every race boundary on every promised
platform, or durability across process and machine crashes. The complete `S-029-ST01` and
`S-029-RT01` matrices and independent transaction review are absent.

Sprint 36 therefore remains blocked. Its retained report is generated at
`artifacts/sprints/sprint-36/local-evidence-report.json` from a committed revision.

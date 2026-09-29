# Coding session recoverability — verification

Date: 2026-09-29. Source: `396669258eedaf6637ec1bae3fb33bca3affde15`.
[Decision 0108](../decisions/0108-coding-session-recoverability.md) governs this
AMR-04.3 component. The [retained record](coding-recoverability-2026-09-29.json) binds
the commits, stage commands, exit codes, log digests, inventory and CLI observation.

## Change

The runtime retained sealed change records and could propose one inverse write at a
time, but nothing declared which session effects are recoverable. A host component
now classifies effects from trusted records. For each file, writes are examined newest
first against the current digest; each is recoverable, already reverted or a conflict.
A conflict also blocks older writes on that file. Created files are not recoverable by
an admitted operation. Commands and uncertain effects need user reconciliation. The
sealed report orders recoverable writes newest first. Neither it nor its text calls an
effect undone. It grants and performs nothing, and no runtime path or CLI uses it yet.

## Checks

The host library returned 297 passed, 50 failed and 8 ignored. The five new tests
passed, and the 50 failures match the retained native-prerequisite baseline exactly.
The Linux library failures also match their baseline. Two blocks first differed only
by where the test harness printed its one-time backtrace note, which follows thread
scheduling; the comparison now ignores that note. The engine passed 1,300 tests.
Clippy first refused the large write variant; boxing it fixed that. Formatting, the
three source audits, selector tests, the task graph and document validation passed.

## Evidence pass

One SBOM write changed only the host component hash. The pass renewed the foundation
reports, the TASKS-bound Story 1.2 gate, index and security map, and the work selector,
which now covers 23 amendment rows. It re-bound the Sprint 11 security map and the
storage security, frozen-scope breadth, AC1 and AC3 reports. The routine Story 11.2
pin moved from `c6798ef4` to `a9c4063e`; it kept all 25 paths and explains three
regenerated reports. All three stages passed. A direct binding inventory finds no
newly stale artifact outside historical verification records.

## Actual CLI observation and limits

At clean revision `08f0721ee479fa75f9d488b9ec74aa4ee1de1b51` the rebuilt binaries behaved
as before. The clean scripted start exited 5 at native Git trust. The fixture with
pre-existing work was refused with exit 1, and both kept every byte and Git state.

Not run: building the effect list from actual session receipts, a CLI view of the
report, any actual-process rollback, the positive native coding workflow, real models,
manual user acceptance and release. Independent review remains open.

Correction (2026-09-29, [Decision 0109](../decisions/0109-review-fixes-for-component-batches.md)):
the retained JSON first said that no task row closes. Commit `a8e842f9`, which added
this record, also closed the component row AMR-04.3 on it. That closure covers the host
declaration only, and no acceptance gate closes. Independent review finding R8 later
bounded its identities, created-file paths and rendered length under Decision 0109.

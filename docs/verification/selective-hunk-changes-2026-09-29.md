# Selective hunk changes — verification

Date: 2026-09-29. Source: `3a3d9d540a956578f28bf93e14c2f1ea4d4b1390`.
[Decision 0107](../decisions/0107-selective-hunk-changes.md) governs this AMR-04.1
component, following the [AMR assessment](amr-assessment-2026-09-29.md). The
[retained record](selective-hunk-changes-2026-09-29.json) binds the commits, stage
commands, exit codes, log digests, inventory and CLI observation.

## Change

Write previews offered one whole-file escaped before and after text, and a user could
accept only the whole change. The kernel now divides an exact UTF-8 preimage and
proposal pair into hunks from a bounded shortest line edit script. Each hunk identity
binds both digests and its line ranges. A selection yields a complete postimage and a
canonical selection digest. Foreign identities and preimage drift are refused, and a
bounded display escapes control characters. Selection grants nothing. No preview, CLI
or host uses it yet; that is AMR-04.2.

The ledger now splits AMR-04 into AMR-04.1 and AMR-04.2, and the work selector accepts
both identities. Its next executable row is still 48.2.4.1, now among 1,624 open rows.

## Checks

The engine library passed 1,300 tests, including six new selection tests. They cover
both extremes, every subset of a three-hunk change, 300 seeded random pairs, identity
binding, drift, non-text input, size and complexity bounds, and rendering. The Linux
and host library failures match their retained sandbox baselines exactly. Clippy,
formatting, the three source audits, selector tests, the task graph and document
validation passed.

One known failure predates this batch and is unchanged: `npm run requirements:current-check`
fails in `scripts/build_contract.py`. Commit `5b731b36` gave `platforms/linux` optional
`ureq` and `url` dependencies for the Decision 0084 research worker. The accepted
dependency graph has no field for optional dependencies. The check fails the same way
at `6a4359d2`.

## Evidence pass

One SBOM write changed only the engine component hash. The pass renewed the foundation
reports, the Story 1.2 contract-boundary gate and evidence index, the work selector,
and the Sprint 13, codec and Story 13.4 context-profile reports. It also re-bound the
Sprint 11 security map to the regenerated component inventory, then the storage
security, frozen-scope breadth, AC1 and AC3 reports. The routine Story 11.2 pin moved
from `2e2b3c7f` to `c6798ef4`. It kept all 25 paths and explains three regenerated
reports. All four stages passed with no retained failure. A direct binding inventory
finds no newly stale artifact outside historical verification records.

## Actual CLI observation and limits

At clean revision `acfce0f679f4c46d42ae9e947bd52fbdd317ec18` the rebuilt binaries
behaved as before. The clean scripted start exited 5 at native Git trust. The fixture
with pre-existing work was refused with exit 1, and both kept every byte and Git state.

Not run: AMR-04.2, and actual-process preservation of human edits with hunk selection.
The positive native coding workflow, any real model or GPU use, manual user acceptance
and release were not run either. No task row or acceptance gate closes.

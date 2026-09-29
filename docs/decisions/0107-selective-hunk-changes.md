# Decision 0107: Selective Hunk Changes

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0081 and 0088; the implementation amendment; current owner restart |
| Scope | AMR-04 decomposition and CAP-12 inspectable selective changes |

## Findings

The [AMR assessment](../verification/amr-assessment-2026-09-29.md) found no further
dependency-ready AMR-01 through AMR-03 work that this lane can verify. AMR-04 was one
undivided package. The amendment requires splitting a package into bounded ledger
rows before coding.

The write-approval preview shows one complete escaped before and after text for each
file. A user can accept or reject only the whole change. CAP-12 requires exact hunk
decisions that preserve human edits, and revalidation on concurrent drift.

## Decision

Split AMR-04 in the ledger, keeping its identity. AMR-04.1 is a kernel selective-change
contract. AMR-04.2 binds a selection into the existing write preview, approval and
grant path, and presents hunks through the CLI and host. Record both identities in
the work selector's accepted set. Later AMR-04 capabilities get further rows when they
are selected.

A selective change starts from one exact reviewed UTF-8 preimage and proposal pair.
The kernel computes a shortest line edit script within fixed bounds: 1 MiB, 20,000
lines, an edit distance of 2,048 and 512 hunks per file. A hunk is a maximal run of
changed lines between unchanged lines. Its identity digests the exact preimage and
proposal digests plus its line ranges, so it cannot select another proposal's hunks.
A selection names accepted hunks. Every other hunk keeps its preimage lines, and the
result is a complete postimage with a canonical selection digest. Unknown identities
are refused. Accepting every hunk reproduces the proposal exactly, and accepting none
reproduces the preimage. Current bytes that differ from the reviewed preimage are
refused as drift, so the caller must build a fresh preview rather than overwrite a
concurrent edit. Non-UTF-8, NUL-containing, oversized or overly complex input keeps
whole-file review. A bounded display marks a missing final newline. As amended by
[Decision 0109](0109-review-fixes-for-component-batches.md), it escapes exactly what
the whole-file preview escapes, except tab and the ASCII quotes: control, format,
line and paragraph separator, private-use, unassigned and non-ASCII space characters,
a leading combining mark, and backslash. Selection takes the current bytes and
refuses drift itself.

Selection grants nothing. The selected postimage must pass the existing exact write
preview, syntax, approval and grant checks, and project validation still runs.
Hunks are line ranges; selecting a subset does not prove the result builds.

## Verification boundary

Kernel unit tests cover both extremes, every subset of a three-hunk change, seeded
randomized pairs, identity binding, forged and foreign identities, drift, non-text,
size and complexity bounds, and safe bounded rendering. No preview, CLI, host or
native process uses the contract yet. AMR-04.2, actual-process preservation of human
edits and independent review remain open.

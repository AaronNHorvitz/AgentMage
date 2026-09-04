# Structured Data Method Registry

## Purpose

This registry identifies the exact normalization, comparison, redaction, and reconciliation methods
implemented by the structured-data capability. A method identity is evidence, not ambient authority:
it does not permit file discovery, writes, link traversal, formula execution, or native application
launches.

| Method | Version | State | Inputs | Deterministic output |
|---|---:|---|---|---|
| Delimited strict parse | `1.0.0` | Implemented | Exact UTF-8 bytes and declared comma, tab, or semicolon dialect | Source-bound table with original and normalized headers |
| Tabular text normalization | `1.0.0` | Implemented | One string | NFKC, lowercase, whitespace-collapsed comparison value |
| Exact-key reconciliation | `1.0.0` | Implemented | Compatible tables and explicit unique key columns | Closed reason per key, source rows, differing fields, and source hashes |
| Structural JSON comparison | `1.0.0` | Implemented | Two bounded strict JSON documents | Canonical pointers, closed reasons, and value hashes |
| Canonical JSON redaction | `1.0.0` | Implemented | Strict JSON document and sorted pointer/key policy | Fully regenerated canonical JSON and removed-value hashes |
| Content-minimized workbook | `1.0.0` | Implemented | Exact-key comparison | Deterministic XLSX proposal with sources, reasons, rows, fields, table, chart, validation, and closed formulas |
| Numeric tolerance reconciliation | `1.0.0` | Implemented | Exact string-backed decimals and non-negative inclusive tolerance | Exact within/outside result without IEEE-754 coercion |
| Financial rounding | `1.0.0` | Implemented | Exact decimal, target scale, and half-even or half-away-from-zero rule | Exact rounded decimal with the selected rule retained |
| Many-to-many weighted allocation | `1.0.0` | Implemented | Exact total and stable unique recipients with positive integer weights | Recipient-ordered exact shares with every minor unit conserved |
| Stale-source admission | `1.0.0` | Implemented | Captured/current source digests, capture instant, evaluation instant, and maximum age | PASS or `reconciliation.advanced.stale_source` |
| Advanced reconciliation record | `1.0.0` | Implemented | Hash-only tabular comparison, exact totals/tolerance, and rounding policy | Source-, schema-, type-, formula-, join-, method-, discrepancy-, and total-bound record |

## Number Policy

Strict JSON uses `finite-ieee754-round-trip-v1`. Integers in the signed or unsigned 64-bit range
remain integers. Other finite JSON numbers are parsed through `serde_json` and serialized using its
shortest round-trip representation. The source digest remains exact, so canonical normalization is
never presented as the original byte representation. Financial reconciliation uses a separate
string-backed base-ten value admitting up to 4,096 coefficient digits and plain decimal notation
only. It retains scale, rejects exponent notation, never converts through floating point, and binds
the selected tolerance and rounding rule in the result.

## Allocation and Freshness Policy

Weighted allocation accepts only stable unique ASCII recipient identities and positive integer
weights. It divides the exact coefficient, then assigns indivisible remainder units by descending
fractional remainder and ascending recipient identity; the allocated coefficient sum therefore
equals the input exactly. Freshness admission separately requires the captured digest to equal the
immediately observed digest, forbids future capture instants, and treats the declared maximum age as
inclusive. A failed freshness check produces no reconciliation record.

## Formula Policy

`closed-summary-formulas-v1` admits only generator-owned summary formulas. Caller text is emitted as
typed inline strings. The current formulas count reconciliation rows and exact-row reasons. Their
exact text is hash-bound; cached values are deterministic from the comparison. Workbook calculation
mode is manual, and the core does not recalculate them.

## Verification Policy

The direct Open XML inspector verifies generated package structure and scans reopened cells for
formula errors. A separate pure validator can assess a caller-supplied native office observation
bound to exact application, executable, package, font, platform, workbook, formula, visual, and
accessibility identities. Synthetic observations can test validation semantics but can never close
native evidence or remove human review.

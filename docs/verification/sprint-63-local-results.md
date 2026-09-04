# Sprint 63 Local Verification

## Scope

The Sprint 63 recorder covers strict canonical JSON parsing and schema validation, deterministic
redaction and comparison, content-minimized reconciliation XLSX generation, bounded reopen,
formula-error scanning, synthetic external-verifier semantics, five closed runtime records, the
method registry, exact decimal tolerance and rounding, deterministic weighted allocation,
stale-source admission, and the review corpus.

## Local Campaigns

- Rust tests exercise valid, malformed, duplicate-key, prohibited-key, bounded, schema-failure,
  redaction, comparison, injection, deterministic package, reopen, formula, and exact no-effect
  behavior.
- Runtime schemas recompute canonical JSON and XLSX hashes and reject value, issue, redaction,
  reason, path, source, reopen, formula, evidence-class, and aggregate-state drift.
- Dependency records retain exact parser/container pins and distinguish direct inspection from an
  unadmitted native office runtime.
- Exact-method tests exercise 4,096-digit decimals, inclusive tolerance, both financial tie rules,
  sign behavior, exact allocation conservation and tie order, changed/aged/future source refusal,
  and a source/policy/method-bound reconciliation record.

## Truthful Disposition

The retained machine-readable baseline report binds 11 passing commands, zero focused skips, and
109 review cases to its exact historical source revision and records each command-output digest.
The later exact-method implementation adds 7 focused Rust cases and extends the current generated
review corpus to 124 cases; its current crate tree is bound by the supply-chain artifacts. The
baseline report remains immutable because reproducing its embedded full documentation command
requires the externally blocked native Podman lane.

Native recalculation, rendering, accessibility, independent review, cross-platform acceptance,
encrypted and legacy formats, and manual fuzzing remain blockers. Sprint 63 is **BLOCKED**.

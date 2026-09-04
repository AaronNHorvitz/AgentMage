# JSON and Reconciliation Review

## JSON Review

1. Confirm the exact source path, source digest, and parser profile.
2. Review the declared number policy before using decimal values.
3. Require a closed schema identity when downstream decisions depend on shape or type.
4. Treat every schema issue as an observed failure, not permission to infer a value.
5. Review redaction pointers and key names before regeneration.
6. Confirm the redacted byte digest and removed-value hash ledger.
7. Review comparisons by pointer and reason; hashes do not disclose the compared values.

## Exact-Method Review

1. Require plain base-ten source values; exponent notation and floating-point intermediates are not admitted.
2. Confirm the exact inclusive tolerance and either half-even or half-away-from-zero rounding rule.
3. For weighted allocation, confirm unique stable recipients, positive weights, exact total conservation, and deterministic remainder order.
4. Verify captured and current source digests match and the capture instant is within the declared inclusive maximum age.
5. Require the method record to bind sources, schema/type/formula decisions, joins, totals, discrepancies, and method version.

## Workbook Review

1. Confirm both exact source hashes and the reconciliation method/version.
2. Review key columns, duplicate reasons, missing sides, and differing fields.
3. Confirm the workbook contains no raw compared values unless a future approved profile says so.
4. Confirm generated formulas match the closed policy and reopen with no error cells.
5. Treat cached values as deterministic generator output, not native recalculation evidence.
6. Do not write the proposal without a separate authorized workspace effect.
7. Require native, visual, and accessibility evidence before making those completion claims.

## Unsupported Decisions

Do not use exact-key output as a substitute for the separately versioned tolerance, rounding,
allocation, freshness, or exact-decimal methods. Encrypted and legacy workbooks remain unsupported,
and no pure method result substitutes for native recalculation or visual/accessibility evidence.

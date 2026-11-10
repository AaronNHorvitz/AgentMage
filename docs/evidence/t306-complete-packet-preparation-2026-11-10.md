# T-303/T-306: retain the complete packet through commit preparation

Status: independent core review required. Both parent rows
remain open, without a new row or sub-row. All existing ancestry stays immutable.

The retained commit-finding review identified that the dormant manual approval
and preparation path carried a bare packet hash after planning. A caller could
assemble an integrity-valid inert plan without running the finding admission
check. The new reviewed-manual approval helpers require the complete typed
packet and verify its integrity, visible and suppressed material findings and
exact packet, repository, parent, logical group and proposed-message bindings.

`PreparedLocalCommit::new` now requires and owns the complete packet. Its fields
are private, with no deserialization or packet-hash restoration. The dormant
driver repeats the complete check at construction and immediately before issuing
its executor permit. The exact request and consumed-grant checks remain in place.
Debug formatting omits packet content. Older manual-receipt primitives remain
integrity-only; their receipts cannot bypass this preparation requirement.

This is packet admission, not authentication of reviewer independence or result
completeness, artifact capture/scanning, signer capture, candidate construction,
original ownership or person consent. The complete live-owner composition and
trusted approval channel remain required. No product route, native Git execution,
new grant or commit/push authority is introduced.

## Validation

The new tests deliberately assemble self-hashed plans and integrity-valid raw
approvals without using the commit planner. They cover 40 severity, confidence
and duplicate combinations; six packet/plan mismatch classes; retained owned
packet identity, canonical request bytes, content-minimized debug output and
approval time bounds; and private retained-packet drift at the common dispatch
guard and driver constructor. These are synthetic component tests, not an
authorized dispatch, native backend, independent review or model result.

The previous approval-field test's preparation and expiry assertions move to
the complete-packet positive test. The old field/receipt assertions remain.
A test-only bridge shares the existing typed packet fixture; it adds no product
constructor, deserialization or public observation-substitution path.

The first focused compile failed on incorrect cancellation-fixture identifiers.
That attempt is retained. The corrected focused run passes all four methods:
30.22 seconds compilation and 0.21 seconds tests, after the shared reservation.
The complete affected fast gate remains **FAIL**: **3,073 passed, 72 failed,
72 ignored**, zero measured or filtered, across **46 unfiltered targets**.
Every per-target failure and ignore identity exactly matches the earlier full
baseline. The direct base adds six Linux passes to that baseline; this batch
adds four kernel passes. Unlike the direct base's narrower selection, all
twenty additional kernel targets are included here, with no exclusion.

All four new methods and nine existing controls in the three affected sources
pass. Formatting and Clippy with warnings denied pass across all four affected
crates. All three source hashes remain fixed, with no later source edit. The
complete run takes 442.96 seconds including shared reservations: formatting
2.35, Clippy 31.12 and Rust tests 407.92 seconds. No timeout occurs. The raw log
SHA-256 is
`6bdb63b7ea462964ff50ad6f7a60b1d6c2a3810d9f1228caacef4744b347013b`.

All source edits precede the final freeze. Raw attempts and reproduction scripts
reside in the private bundle
`/lane/state/tmp/t306-complete-packet`. Its full runner reserves heavy commands
internally; never outer-wrap it. Its comparison script checks data only, not
gate acceptance. Final documentation, public-content, unchanged-row and staged
checks pass after recording the result. No complete gate PASS is inferred.

The retained independent reviews for the preceding commit-finding admission and
original-owner checker increments both have **Verdict: PASS**, with no high or
medium finding. Their low findings stay in the same rows. This batch supplies
packet admission at preparation/dispatch and an opt-in reviewed-manual receipt
path; raw receipt authenticity and full original-owner composition remain open.
Exhaustive severity policy, native refusal coverage and concurrent cancellation
reporting precedence remain backlog alongside earlier lows.

No evaluation, hidden/held-out task, scorer, prompt, threshold, dependency, SBOM,
lifecycle or scoreboard changes. No GPU, host key, download, push, release or
readiness claim. Native admission, actual independent checker/artifact
integration, full product approval composition, durable recovery and measured
coding acceptance remain open.

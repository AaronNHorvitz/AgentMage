# T-303/T-306: reject token-limited endpoint completion claims

Non-closing core fix under Decisions 0153 and 0154. Both parents remain open;
no row is added or closed. Independent exact-commit core review is required.
The predecessor is immutable and under independent review.

The endpoint completion constructor previously accepted an output-token-limit
finish as well as an ordinary stop. Exact bytes, valid JSON and passing tool
observations cannot establish that a token-limited answer finished. Require an
ordinary stop before constructing any endpoint completion claim. The transport
turn continues to accept and record both finish reasons as observations.

A new regression test uses both changed and no-op claim JSON. For each, the
same complete bytes and accepted descriptor produce a claim only with an
ordinary stop. Another test produces three passing deterministic candidates:
one native, one endpoint and another endpoint run with otherwise shared inputs.
Each passes its own exact registry; all six foreign pairings refuse with a
context mismatch. Original test bodies, verifier predicates and registry rules
are unchanged. This covers the candidate-identity boundary; live coordinator
selection of the correct registry remains required and open.

The targeted regression failed against the unchanged production constructor:
zero passed, one failed, at the token-limit refusal assertion. After the fix,
the complete scoped gate ran all six unmodified planned commands: formatting
and Clippy with warnings denied passed; both new tests passed. Across four
crates and 43 targets, results are **2,566 passed, 72 failed, 71 ignored, zero
filtered**. Every predecessor passing identity is retained with exactly the two
new methods. Failure and ignore identities and normalized failure diagnostics
match the predecessor exactly. The failures remain 53 host and 19 Linux library
cases under deferred T-301.1. Cargo tests return 101; the source gate remains
**FAIL**, not green.

The six command durations total 456.51 seconds, including shared-slot waits:
formatting 70.50 seconds, Clippy 21.65 seconds and tests 362.71 seconds. The
remaining light checks took 1.65 seconds. Both Rust input hashes stayed fixed
through the full gate. Final light checks verify documentation and public
hygiene, original test-body preservation and the exact production transformation.
Private raw logs, source snapshots and reproducible comparison helpers are kept
for independent review. No real model run or coding score is claimed.

No GPU, model, frozen-task, hidden/held-out, scorer, threshold, ignore, prompt,
dependency, SBOM, status, push or release change. No score is claimed. Full
endpoint orchestration and API wiring, native edit/test/retry, exclusive and
durable ownership, approved Git and parent measurements remain open.

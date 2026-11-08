# T-303/T-306: endpoint model turns

Non-closing core implementation under Decisions 0153 and 0154. Both parents
remain open; no sub-row or closure is introduced. Independent core review is
required. This is a coordinator model-turn component, not another agent loop.

`RuntimeEndpointTurns` admits explicit prepared model turns using the existing
endpoint phase, original context and streaming port. It retains one sealed run,
one cumulative resource ledger and one run clock. Accepted descriptors bind the
original context/request/response to the sealed run and coordinator-compatible
turn identity. No native profile or tokenizer identity is invented.

The existing coordinator's phase control now borrows its task and elapsed limit
directly; its native constructor delegates with the same original values. Both
paths use the same monotonicity, deadline, exact cancellation and callback checks.
The endpoint component rejects durable/write modes and resume cursors, closes on
every failed turn/refusal, and cannot retry or reset spent budgets. It retains
the exact owner-observed cancellation without claiming an exit receipt.

Known model resources get finite run-derived defaults, narrowed by declared
work-packet budgets. Full output/time envelopes must fit cumulative remaining
capacity before dispatch. Accepted observations commit actual output/time
atomically. Failed attempts retain admitted calls/prepared input; a pre-attempt
control stop records neither. Unknown token and memory counts are absent.

The synthetic tests exercise explicit multi-turn dispatch, changing original
context, exact run/turn identity, each count/input ceiling and narrower packet
budgets, cumulative output/time reservations, failed streams/dependencies,
callback expiry, clock regression, exact/mismatched cancellation, pre-attempt
expiry, request/context drift, unowned modes and cursor refusal. No real model,
tool execution or coding score is claimed.

The first complete scoped gate failed to compile the new test because it
imported the coordinator's private `derived_id` helper. Formatting and light
checks passed; Clippy and tests returned 101, and no test results were produced.
The correction removes that test-only import and substitutes three independently
calculated constant identities. Every assertion and production byte is preserved.
Initial source copies, hashes and the complete failed log are retained privately;
the final audit proves those exact four text replacements.

The first corrected complete scoped gate ran all six unmodified planned commands
under one shared build reservation, in **333.22 seconds** excluding queue wait.
Formatting passed in 2.00 seconds and Clippy with warnings denied passed in
26.24 seconds. Tests took 303.63 seconds: **43 targets, 2,545 passed, 72 failed,
71 ignored, zero filtered**. All eleven new methods passed. Cargo tests returned
101 and the source gate remains **FAIL**.

That run's audit compares every selected target with the predecessor: all 2,534
passing identities remain, plus exactly the eleven new methods. Failure
identities, normalized diagnostics and ignored identities match. The existing
failures are 53 host and 19 Linux library cases under deferred T-301.1 executable
ownership. Matching that baseline does not make the gate green.

All six Rust hashes remained fixed throughout that run, and its final light
checks passed. Subsequent code review found a reservation gap during unwinding:
if a port panicked and the caller caught it, the component stayed closed but
omitted its reserved call/input usage. The final source now spends admission
before phase entry and rolls it back only on a normal pre-attempt stop. A
twelfth test catches a synthetic port panic and checks spent usage, zero accepted
completion and refusal of re-entry. Existing tests and production admission
rules are preserved. Initial, intermediate and final source copies and raw logs
are retained.

The final complete scoped gate ran all seven unmodified planned commands under
one shared reservation in **303.43 seconds**, excluding queue wait. Formatting
passed in 2.00 seconds, Clippy with warnings denied in 18.93 seconds, and tests
took 281.05 seconds: **43 targets, 2,546 passed, 72 failed, 71 ignored, zero
filtered**. All twelve new methods passed. Every predecessor pass, failure
identity, normalized diagnostic and ignore is preserved. The source gate remains
**FAIL**, with Cargo tests returning 101 on the same two baseline targets.

All six final Rust hashes remained fixed during the complete gate. No further
Rust edit or Cargo run followed. Final light checks verify exact paths,
unchanged Cargo commands and ledger table rows, all source revisions, the exact
corrections, byte-identical retained review and public hygiene. Raw logs and
reproducible audits are retained privately. No owned validation or model job
remains. The initial private final-check helper hit a variable-name collision;
only the helper was corrected, its failed log retained, and light checks then
passed. Independent core review is required for this final increment.

This component emits no runtime events and interprets no proposal. Full endpoint
coordinator composition, native edit/test/retry, durable ownership, approved Git
and parent scoreboard runs remain open. No frozen development/held-out task,
hidden test, scorer, threshold, ignore, dependency, supply-chain output, model
download, GPU work, push or release is changed.

The predecessor's [independent core review](../reviews/7e7b2c2a9c69-local-coding-endpoint-run-admission.txt)
returned **Verdict: PASS** for exact commit
`7e7b2c2a9c699613048695cd9e15119124401e70`, with no high/medium finding.
Its model-name format-character and empty-authority lexical-guard lows stay
backlog with earlier lows. The report is retained byte-identically. This
accepts that static run-framing increment only, not these turns or either parent.

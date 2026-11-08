# T-303/T-306: streaming endpoint model port

Non-closing core implementation under Decisions 0153 and 0154. Both parents
remain open; independent exact-commit review is required. No new sub-row is
introduced.

The live snapshot, selected-workspace and repair proposal routes now dispatch
through `runtime_loop::endpoint::RuntimeEndpointModelPort`. The API adapter moves
its original prepared credential-owning request and borrows the existing
cancellation object. It rebuilds no request and reads no credential after repair
consent. The kernel compares the port's complete prepared request before
invocation and spending dispatch accounting. A mismatch spends the phase,
records no admitted dispatch and cannot re-enter.

The kernel independently hashes and counts the text delivered through this port.
The completed transport descriptor must match that exact UTF-8 byte sequence,
including order, before completion or resource accounting commits. It retains
no second plaintext buffer. Byte ceilings are enforced before consumer delivery;
control is checked before and after each callback. Stream refusal latches and
suppresses every later fragment even if a broken port ignores control or tries
to renew it. A synchronous callback remains cooperative, never preempted.
Transport dependency failures keep their existing code. Previously delivered
text stays provisional; no failed phase yields a completed inference or edit.

Six new synthetic streaming-port methods cover Unicode and empty fragments,
both complete codec finishes, different fragmentation, digest/length/order/
omission mismatches, complete prepared-request mismatches before invocation,
exact and exceeded multibyte limits, all four control stops before and after
consumer callbacks with attempted renewal, partial dependency failure and
single-use refusal. The earlier six descriptor phase methods remain and now
use a synthetic streaming adapter. Existing actual API-to-synthetic-server
contracts exercise the production adapter and independently compare returned
response digests and bytes against streamed text. These are neither native
workflow observations nor real-model scores.

The complete scoped gate selected `agentmage-host`, `agentmage-kernel-engine`,
`agentmage-local-api` and `agentmage-platform-linux`. Its seven unmodified
commands ran under one shared reservation in **307.42 seconds**, excluding the
reservation wait. Formatting passed in 2.15 seconds and Clippy with warnings
denied in 19.48 seconds. Tests took 284.29 seconds: **43 targets, 2,524 passed,
72 failed, 71 ignored, zero filtered**. Cargo returned 101; gate exit was 1.
The complete source gate remains **FAIL**.

The audit compares the same 43 targets with the predecessor's complete phase
gate, which had 2,518 passes. Every prior passing test name remains; the only
six added passes are the new streaming-port methods. Every failure identity,
normalized diagnostic and ignore matches. The 53 host-library and 19 Linux-
platform-library failures remain in the unchanged native targets, under the
deferred executable-ownership blocker. All twenty required phase, proposal and
repair methods pass unfiltered. No test selection, skip or trust check changes.

All three Rust input hashes stayed fixed throughout validation. No failed
compile, source correction or source rerun was needed. The predecessor review
arrived while the gate was queued, so its retained-report lint is present in
the initial and final seven-command plans. Final light checks require the
seven exact paths, byte-identical review, unchanged ledger table rows, Cargo
commands and source hashes, privacy and no added ignores. They rerun no Cargo
command. Raw logs, plans, source freeze and reproducible audit helpers are kept
privately; no owned validation or model job remains.

The predecessor's [independent core review](../reviews/b56d936d7147-local-coding-endpoint-phase.txt)
arrived while this batch was queued, with **Verdict: PASS** for exact commit
`b56d936d71473cdeb3cc80ff3c640b25b58d5087` and no high/medium finding. It is
retained byte-identically. Low L1 (phase-usage assertions on workspace/repair
terminals and API-level pre-dispatch zero-attempt coverage) remains backlog in
these rows with earlier retained lows. That verdict accepts the predecessor
phase only, not this new streaming port or either mission parent.

This port is one model execution boundary inside the existing runtime-loop
module, not a new agent loop or full coordinator admission. The native
coordinator's exact-profile, context, token, tool, permission and result checks
are unchanged. Endpoint run envelopes and full context/turn composition,
native edit/test/retry observations, durable ownership, separately approved
commit/push and parent scoreboard runs remain open. No frozen task, hidden or
held-out test, scorer, threshold, ignore, model, GPU run, supply-chain output or
release changes here.

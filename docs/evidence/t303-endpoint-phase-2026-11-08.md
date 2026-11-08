# T-303/T-306: endpoint model phase on the shared resource ledger

Non-closing core implementation under Decisions 0153 and 0154. Both parents
remain open; independent exact-commit review is required.

The live snapshot, workspace and repair proposal paths now execute their frozen
credential-owning request through a kernel-owned endpoint model phase in the
existing runtime-loop module. This is a single inference phase, not a new agent
loop or admission into the full native coordinator. It introduces no fabricated
exact model profile. Existing native admission and resource construction remain
unchanged.

The same runtime resource ledger has an internal endpoint constructor deriving
one dispatch allowance and input/output/time ceilings from the prepared request.
Its queue and artifact ceilings are zero; it offers no tool, write or retry.
The phase checks framed context/body fingerprints and bounds before disclosure,
checks the original borrowed control before and after dispatch, spends its
single-use state even on refusal, and requires exact returned request equality.
Completed text/time accounting commits atomically after result checks. The
transport remains responsible for sockets, credentials, codec validation,
response text hashing and control polling. Synchronous callbacks remain
cooperative. The phase cannot preempt them or validate server-side resources.

Repair framing occurs before consuming the existing one-use disclosure receipt.
The original prepared transport is moved into its single dispatch closure;
requests are never reconstructed after consent. Model transport errors retain
their existing codes. Phase refusals have closed runtime/control labels. The
API includes content-free phase usage with admitted attempts and prepared bytes,
and completed decoded text/time only for accepted complete responses. Partial
or invalid results leave completed fields null; unknown memory and tokens are
never exposed as observed zeroes. Completed output-limit responses and strict
edit failures still retain completed inference accounting without edit success.

The synthetic phase tests exercise six methods: exact byte/time boundaries,
missing/zero/nonzero provider usage, both codec finishes, refusal/re-entry,
pre-dispatch stops, post-dispatch stops, dependency failure, mismatched context/
selection/body/limits, malformed bounds, atomic refusal accounting, rounding
and complete empty text. Existing actual API-to-synthetic-peer fixtures check
terminal phase usage against the frozen request and completed transport result.
These are not real model or native coding workflow results.

The complete source gate selected `agentmage-host`, `agentmage-kernel-engine`,
`agentmage-local-api` and `agentmage-platform-linux`. Six unmodified gate commands
ran under one shared reservation in **376.83 seconds**, excluding the reservation
wait. Formatting passed in 2.00 seconds and Clippy with warnings denied in
32.41 seconds. Tests took 341.07 seconds: **43 targets, 2,518 passed, 72 failed,
71 ignored, zero filtered**. Cargo returned 101 and the gate returned 1.
The complete source gate remains **FAIL**.

The audit selects these same 43 targets from the earlier complete endpoint-
exchange gate and requires every failure identity, normalized diagnostic and
ignore to match. They do: 53 failures are in the unchanged host library and
19 in the unchanged Linux platform library. The selected predecessor targets
had 2,511 passes; six new phase methods and the already validated API control
method introduced after that older baseline account for the seven-pass increase.
The exact native executable-trust owner-action blocker remains deferred. No
selection, threshold, ignore or trust check was weakened. All six phase methods
and the existing snapshot, workspace, cancellation, incomplete, usage and repair
obligations pass unfiltered.

All six source hashes stayed fixed throughout the gate. There was no failed
compile, source edit during validation, or source rerun. The preceding review
arrived during tests, so the final seven-command plan adds only its retained-
report lint. Cargo commands and source inputs remain fixed. Final light checks
verify ten exact changed paths, the byte-identical review, unchanged ledger
rows and Cargo inputs, source hashes, privacy and absence of added ignores;
they rerun no build or source test. Raw logs, plans, freezes and reproducible
audit helpers are retained privately. No owned build or model job remains.

The preceding private-encoding [independent review](../reviews/623f06a3da55-local-coding-private-encoding.txt)
arrived during validation with **Verdict: PASS** for exact commit
`623f06a3da55b702516884718f5333783350cd43` and no high/medium finding. It is
retained byte-identically. Its low L1 (type-enforced zeroization of the public
encoders' returned buffers) remains backlog in these rows, alongside earlier
control-stop, connect-expiry, owner-polling and native failed-test receipt lows.
This verdict accepts its pinned increment only. No acceptance of this phase or
mission row is inferred from it. Full
endpoint run envelopes, context ports, coordinator edit/test/retry composition,
durable ownership, approved Git publication and parent scoreboard runs remain
open. No frozen task, hidden/held-out test, scorer, threshold, ignore, model,
GPU run, supply-chain output or release is changed by this batch.

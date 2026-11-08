# T-303/T-306: bounded private request encoding

Implementation: 2026-11-07; validation: 2026-11-08.
Non-closing core work under Decisions 0153 and 0154.
Both parents remain open; independent exact-commit review is required.

## Change

The endpoint-exchange [independent review](../reviews/6bee4c9d55be-local-coding-endpoint-exchange.txt)
accepted its exact increment with low L1/L2 left in-row. This batch addresses
those lows without changing the prompts, endpoint choice, wire contract,
permissions, resource ceilings or any evaluation input.

The exact message-array fingerprint now streams serde JSON directly into a
SHA-256 byte-counting writer. It allocates no second serialized message buffer.
The request-body encoder reserves the existing 256 KiB wire ceiling before
writing private bytes. Its bounded writer rejects overflow before appending;
the allocation cannot grow and leave freed plaintext buffers behind. On an
encoding failure its zeroizing owner erases the partial buffer. Successful
encoding transfers the same allocation to the caller, which remains responsible
for its bytes. Transport requests and both API preflight encodings use zeroizing
owners. This does not claim erasure of all conversation copies or hash state.
The fixed reservation costs 256 KiB per active encoded request, even for a short
message, and stays bounded independently of the amount of JSON escaping.

The API socket assertions now require the documented serialized finish values:
`stop` on the successful proposal fixture and `output_token_limit` on completed
length refusals. Neither establishes coding success. Incomplete streams still
have no completed result or accounting.

## Verification

The writer regression uses the production allocation constructor, fills that
allocation through multiple writes, checks that its address and capacity stay
fixed, and verifies that an overflowing write changes neither its contents nor
allocation. The request contract checks the
exact 256 KiB wire boundary and one byte beyond, with and without usage enabled;
existing raw/escaped oversize cases remain. The actual prepared-request socket
fixture uses a larger repeated Unicode/escaped prompt and independently compares
its transmitted body and message-array digests, lengths and original text, for
both usage settings. It also mutates the caller's prompt before dispatch.

The complete source gate selected `agentmage-host`, `agentmage-local-api` and
`agentmage-platform-linux-inference`. Its six unmodified commands ran under one
shared build reservation in **236.39 seconds**, excluding the reservation wait.
Formatting passed in 0.95 seconds; Clippy with warnings denied passed in 47.12
seconds. The complete all-target tests took 186.97 seconds: **27 targets,
904 passed, 53 failed, 19 ignored, zero filtered**. Cargo returned 101 and the
source gate returned 1. The source gate remains **FAIL**.

The audit requires the same 27 targets, every failure identity and normalized
diagnostic, and every ignore from the complete predecessor gate. They match:
903 prior passes plus the new allocation test, with the same 53 failures in the
unchanged host library. The deferred executable-trust owner-action blocker
remains open. No test selection, skip, ignore, threshold or trust check changed.
The added allocation test and strengthened encoding, prepared-request socket
and API success/length/incomplete fixtures pass unfiltered, as do the existing
repair, cancellation and controlled endpoint-exchange obligations.

All six Rust source hashes stayed fixed throughout validation. The source gate
started after the final source edit; there was no failed compile or source
rerun. The predecessor review arrived during tests, so the final seven-command
plan adds only its retained-report lint. Cargo commands and sources stay fixed.
Final light checks cover the ten exact paths, byte-identical review, unchanged
ledger rows and Cargo inputs, source hashes, privacy, and absence of new ignores;
they rerun no build or source tests. Raw logs, plans, hashes and reproducible
audit helpers remain private. No validation or model process remains running.

## Remaining work

The predecessor controlled-dispatch [independent review](../reviews/3642273c2711-local-coding-endpoint-control.txt)
arrived during validation with **Verdict: PASS** for exact commit
`3642273c2711c1346d692ad8cd8774574c2534c4`, and no high/medium finding. It is
retained byte-identically and accepts its pinned increment only. Its route-level
control-stop fixture, connect-expiry diagnostic and per-byte owner-polling lows
remain backlog, alongside the earlier native failed-test receipt fixture.
Endpoint admission into the
existing native coordinator, its run envelopes/context ports/resource ledger,
native edit/test/retry execution, durable ownership, approved Git publication
and parent scoreboard measurements remain open. No model/GPU, native workflow,
score, row closure, supply-chain regeneration or release is claimed.

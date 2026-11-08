# T-303/T-306: configured transport in endpoint coordinator turns

Non-closing core increment under Decisions 0153 and 0154. No new row or closure;
both parents remain open. Independent exact-commit core review is required.
The preceding coordinator commit remains immutable. Its retained independent
core review returned FINDINGS: medium M1 is fixed in this batch, and low L1
(sealing the model-port family trait) and L2 (direct read-only catalog-authority
coverage) remain backlog in these same rows. Re-review is required.

M1 let an untrusted model supply its own argument schema, encoded bytes and
SHA-256. A bad envelope then reached a trusted-boundary error and left the run
unterminated. Models now supply only a plain JSON argument value. The coordinator
rejects duplicate keys, selects the registered schema, encodes/hashes the bytes
itself and validates the complete call before requesting a tool. Invalid values,
old envelope-shaped arguments and malformed tool representations receive a
canonical non-success endpoint terminal, with zero tool effects and no further
dispatch. The native family decoder and trusted-port fault behavior are unchanged.
The regression requires the terminal event to hash the whole endpoint envelope,
checks its event chain and verifies repeat reads execute nothing.

The host now implements `RuntimeEndpointTurnPort` with the existing
credential-owning local chat transport. Its endpoint/model binding comes from
that transport. The original destination approval, credentials, local address
restrictions, immutable request body, socket deadlines and streaming codec stay
with their existing owner. No new transport, orchestration loop, retry, prompt
or dependency is introduced.

A trusted context source receives the original sealed request and current
coordinator-owned calls, results, evidence and pre-effect refusals. The adapter
borrows its original messages once, independently admits their ordered content
and JSON fingerprint, and freezes those same messages through the transport.
The prepared dispatch borrows only external configuration, so it never rebuilds
context or resolves a different credential. The transport erases its encoded
body; the context source owns retention and erasure of its own text.

The same run control is checked before and after context preparation and after
freezing, and passed through every transport socket wait and stream callback.
Mismatch or an initial stop refuses before context. Invalid original context or
destination approval refuses before connection. Transport failures map to closed
content-free port failures. Accepted response bytes, optional provider-reported
usage and dispatch time retain their existing endpoint descriptors; no native
profile, tokenizer, GPU or process identity is invented.

The executable socket campaign uses a synthetic endpoint, a test-only permission
owner and the existing native read-only snapshot operation. It exercises two
streamed turns, a pending exact read approval, delivery of the actual read result
to the next context, deterministic grounded completion, retained whole endpoint
provenance and idempotent terminal reads. Further cases cover invalid context,
wrong approval, model/address mismatch, every owner-stop category before context,
a stop after context preparation, invalid configuration, stalled sockets without
retry, truncated streams, token-limited output and ungrounded ordinary output.
These fixtures do not qualify a local model or the live filesystem/grant owner.

An initial full scoped gate retained **2,804 passes, 73 failures and 79 ignores**
over 52 targets, zero filtered. Every original passing identity remained, and the
original 72 failed identities, 79 ignored identities and normalized diagnostics
matched the retained baseline. The additional failure was this batch's fixture
expecting the cumulative-budget code at a static-envelope refusal: the runtime
correctly reported `runtime.port.resource_exhausted`. Formatting also required
moving the host module declaration to its sorted position. Strict Clippy passed.
That run took 445.09 seconds including shared-slot waits. Both fixture assertion
and ordering were corrected without changing a runtime limit or threshold; its
sources and logs remain retained. Only the final full rerun qualifies the final
sources.

The final complete scoped gate ran all seven commands over five crates and
52 targets: **2,805 passed, 72 failed, 79 ignored, zero filtered**. Every original
passing identity remains, with six new host methods and one new engine method.
Failed/ignored identities and normalized failure diagnostics match the retained
baseline exactly. The 53 host-library and 19 Linux-library failures remain the
deferred T-301.1 baseline. Formatting and Clippy with warnings denied passed;
Cargo tests return 101 and the source gate remains **FAIL**, not green.

The commands total 727.11 seconds including shared-slot waits: formatting
398.86 seconds, Clippy 13.68 seconds, tests 312.92 seconds and light checks
1.65 seconds. All eight Rust input hashes stayed fixed. Final light checks
recheck Markdown and report format, byte-identical review retention, public
hygiene, unchanged ledger rows, original native host test bodies and the
unchanged native tool-validation implementation. The existing endpoint
malformed-envelope test was deliberately strengthened under M1; the new
eight-case regression covers duplicate arguments, forged identity/authority,
missing or invalid argument values and old schema/media/digest envelopes.
Private raw logs, exact source copies and comparison helpers remain available
for independent review. They require every target and command, exact identities,
complete summaries and the source-freeze marker; a failing exit code alone is
never accepted as evidence of the known baseline.

Exclusive live HTTP ownership, concrete context and scoped filesystem/grant
composition, the endpoint HTTP/CLI/MCP projection, controlled edits, confined
tests/retries, durable recovery, approved Git and parent measurements remain
open. No GPU/model/evaluation/hidden/held-out/scorer/threshold/ignore/SBOM/status/
push/release or scoreboard change is made.

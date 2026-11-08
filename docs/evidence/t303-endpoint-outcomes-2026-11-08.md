# T-303/T-306: endpoint terminal framing without native provenance

Non-closing core increment under Decisions 0153 and 0154. Both parents remain
open; no row is added or closed. Independent exact-commit core review is
required. The preceding immutable stop-safety commit received independent core
PASS during this session, with no high/medium or new low finding; its report is
retained byte-identically under docs/reviews/.

While tracing endpoint coordinator composition, the builder found that native
terminal framing requires an exact admitted native model manifest for every
successful rendered answer. Reusing that assignment for a configured endpoint
would invent provenance. This batch shares the existing native outcome shape
predicates and introduces a distinct endpoint envelope for the actual identity
available. It does not connect an endpoint to the orchestration loop or API.

The native outcome validator still enforces its original identity, terminal
state, counter ceilings, evidence, receipts, diagnostic format, bounded inline
or artifact output and exact native answer-evidence predicates. Its sealer,
wire format and successful-answer assignment remain unchanged. Common predicate
extraction is mechanical; original test bodies are preserved.

Endpoint framing requires the exact sealed ephemeral read-only request without
a resume cursor. Successful rendered output requires the owning accepted
endpoint completion claim at both sealing and verification, exact equality of
the whole inline payload, and the claim's actual current turn with equal
nonzero model/turn counts. The claim already binds ordinary model stop, exact
stream bytes, original run, request/response descriptors and schema. Endpoint
answer provenance binds that complete claim and the ordered current evidence;
the nested native answer-evidence field must be null. The outcome digest covers
the whole envelope, including this separate provenance. Decoded metadata cannot
recreate accepted-turn authority. Failures carry no answer provenance or supplied
completion claim. Artifact-backed, durable and controlled-write endpoint framing
remain refused. Shared validation still allows success without rendered output
and with evidence, as before; this creates no completion authority.

Six new synthetic tests cover accepted-turn framing, deterministic sealing,
whole-envelope hashing, exact claim and output binding, foreign runs and resealed
request revisions, current-turn counters, changed evidence, shared terminal
predicates, native evidence substitution, unsupported modes and truthful failure
records. Closed versioned parsing checks round-trip behavior, unknown/duplicate/
missing fields, wrong versions and refusal as a native outcome. These fixtures
test framing; no verifier task proof, real coding run or model score is claimed.

The final complete scoped gate ran all seven planned commands over four crates and
43 targets: **2,572 passed, 72 failed, 71 ignored, zero filtered**. All six new
methods passed, and every predecessor passing identity is retained. Failure
and ignore identities and normalized failure diagnostics match the predecessor
exactly. The failures remain 53 host and 19 Linux library cases under deferred
T-301.1. Formatting and Clippy with warnings denied passed; Cargo tests return
101 and the source gate remains **FAIL**, not green.

The seven command durations total 339.46 seconds including shared-slot waits:
formatting 33.07 seconds, Clippy 19.09 seconds, tests 285.65 seconds and the
initial light checks 1.65 seconds. All six Rust input hashes stayed fixed.
Final light checks also validate the report that arrived during testing,
documentation and public hygiene, unchanged ledger table rows, original test
bodies and mechanically identical native outcome predicates, sealer and
verifier bodies. Private raw logs, immutable source copies and reproducible
comparison helpers are retained for independent review. The runner uses the
gate's per-Cargo build reservations; it assumes no particular lock descriptor
and accepts no failure without proving complete gate execution and its results.

An initial full gate had the same test totals and baseline diagnostics, taking
389.77 seconds. Final source review then found that endpoint sealing hashed
citations before rejecting malformed outcome metadata. Shape validation now
precedes answer construction and citation hashing in both sealing and
verification. An assertion in an existing new method checks preflight refusal
before missing-claim processing. The full gate was rerun after that correction;
only the second run qualifies the final sources. Both source snapshots and raw
logs remain retained.

Full endpoint coordinator/API wiring, exclusive/durable ownership, verifier
registry composition, native edit/test/retry, approved Git and parent
measurements remain open. No GPU, model, frozen-task, hidden/held-out, scorer,
threshold, ignore, prompt, dependency, SBOM, status, push or release change.

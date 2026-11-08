# T-303/T-306: recorded endpoint model turns

Non-closing core implementation under Decisions 0153 and 0154. Both parents
remain open; no new row or closure. Independent exact-commit core review is
required. The predecessor endpoint-turn commit is immutable. Its
[independent core review](../reviews/2164c8b71f60-local-coding-endpoint-turns.txt)
has **Verdict: PASS** for exact commit
`2164c8b71f6042caa4c5a6b766337b6a5b3f5184`, with no high/medium finding.
The guide edit addresses low L1's fresh-instance/exclusive-ownership wording;
low L2 (live coordinator/API wiring and scoreboard measurement) and earlier
lows remain backlog. This accepts the predecessor increment only.

The existing explicit endpoint turn component now uses the native coordinator's
shared event framing and its existing publisher, sequence verification,
subscriptions and replay implementation. Native identity, sensitivity, retention,
event fields and sealing behavior are preserved by the extraction; native model
admission, tools and verifier contracts are unchanged. No native profile is
constructed from endpoint selection and no second orchestration loop is added.

One initial run event and four events per admitted explicit turn bind run,
session, task, policy, correlation, turn, complete request/result descriptors and
completed-turn projection. Only digests and metadata enter the event envelopes.
Source text, model text, selected address/name and credentials are excluded.
Retention is ephemeral. No run terminal, task success, cancellation acknowledgement
or process-exit receipt is manufactured. Failed model intent can precede a
pre-attempt control stop; known dispatch usage stays zero in that case.

Full turn-event count and envelope capacity are checked against the shared ledger
before intent publication or dispatch. Preflight covers both completion shapes
and the largest legal timestamp encoding; actual canonical count/bytes are
committed. Capacity refusal consumes no model call and never enlarges a limit.
Failed streams emit a fixed content-free failure code. A caught dependency panic
leaves the model-intent prefix open, closes the component and retains reservations.
After a clock failure, recording uses the last trusted observation.

Six added synthetic methods check two-turn descriptor/digest binding, identity,
privacy, retention, exact recording counters, capacity boundaries, malformed and
dependency failures, bounded live delivery, lagged-subscription replay, invalid
cursors/batches, caught dependency panic and a pre-attempt timeout. They exercise
the existing event verifier and show replay never dispatches a model. Existing
synthetic cases and all frozen evaluation inputs are preserved.

The first complete scoped gate ran all six unmodified planned commands under
one shared reservation in **342.66 seconds**, excluding queue wait. Formatting
passed in 2.05 seconds and Clippy with warnings denied in 21.64 seconds. Tests
took 317.57 seconds: **43 targets, 2,537 passed, 87 failed, 71 ignored, zero
filtered**. Nine existing endpoint-turn cases and all six new event cases failed
because the new diagnostic code was not in the unchanged event allowlist;
preflight therefore refused even accepted turns. The other 72 failures are the
existing host/Linux baseline. The source gate was **FAIL**.

The correction substitutes the existing approved
`runtime.model.incomplete.unknown` diagnostic. It changes exactly four literals
across three files, including the new assertion's expected code. Every assertion
and all other Rust bytes are preserved. The validator and original tests are
unchanged. The first run's raw log, source copies and hashes are retained; the
final audit proves the exact correction. No threshold or allowlist is weakened.

The corrected complete gate ran all seven unmodified planned commands under one
shared reservation in **300.45 seconds**, excluding queue wait. Formatting
passed in 2.05 seconds, Clippy with warnings denied in 19.43 seconds, and tests
took 277.47 seconds: **43 targets, 2,552 passed, 72 failed, 71 ignored, zero
filtered**. All six new methods pass. Every predecessor passing identity is
preserved, with exactly those six additions. Failure identities, normalized
diagnostics and ignored identities match the predecessor. The failures remain
53 host and 19 Linux library cases under deferred T-301.1. Cargo tests return
101 and the source gate remains **FAIL**; a matching baseline is not a green gate.

All seven corrected Rust hashes remained fixed through that gate. The final
light checks verify exact paths, unchanged ledger rows and Cargo commands,
byte-identical retained review, public hygiene, original-test preservation and
the exact four-literal correction. The static native-frame audit also confirms
the original event initializer and sensitivity mapping after explicit field
rebinding, allowing only formatter differences. Raw logs, immutable source
copies and reproducible audits are retained privately. No owned validation or
model job remains. No real model run or coding score is claimed.

Full endpoint coordinator composition, native tool/edit/test/retry execution,
durable/exclusive ownership, approved Git publication and parent scoreboard
measurements remain open. This connects model-turn observations to the existing
coordinator event machinery; it does not connect endpoint requests to the
coordinator's native-only tool/verifier interfaces or a live runtime API route.
No GPU work, evaluation, hidden or held-out access, scorer, threshold, ignore,
model download, dependency, SBOM, release or status change is made.

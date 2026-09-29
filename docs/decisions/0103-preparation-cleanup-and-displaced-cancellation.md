# Decision 0103: Preparation Cleanup and Displaced Cancellation

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0061, 0081, 0095, 0101 and 0102; current owner restart |
| Scope | AMR-01/AMR-03 and the standalone coding runtime prerequisites |

## Findings

An independent read-only review of the Decision 0102 source commit `69cd9adc`
passed with one medium and five low findings. The medium finding is a source
defect: a controlled preparation that loaded the runtime and then failed its
readiness, serving or control check kept the model loaded. No cleanup ran and no
receipt or uncertainty reached the run. The host's exact 32K and one-slot served
profile check had the same gap after a successful controller preparation.

The low findings were: a local startup-ceiling expiry during readiness client
construction surfaced as a run-control stop; the legacy `load` entry point ignored
a consumed controlled attempt; a spawn failure changed the legacy entry point's
error category; and a dependency failure that displaced the coordinator's own
observed cancellation left no journal record of that observation.

## Decision

A controlled preparation attempt that loaded the runtime in the same call, then
failed for any reason, unloads that exact tuple before returning. It accepts only
a receipt with the exact profile, adapter and empty state. Any other cleanup result
returns cleanup uncertainty in place of the original error. The attempt remains
consumed either way. A failure on a later turn, after an earlier successful
preparation, retains the loaded tuple for its existing owner. That tuple follows
the lifecycle of a completed run.

A consumer's own served-tuple requirement is checked inside that same attempt
through an acceptance predicate. Rejection is a served-profile mismatch and
triggers the same cleanup. The host's development candidate uses this predicate
for its exact 32,768-token, one-slot requirement.

Readiness client construction maps a closed startup or outer budget through the
existing startup classification. Legacy `load` refuses after any controlled attempt.
A failed spawn reports cleanup uncertainty on controlled calls. Legacy calls keep
`model.llama-driver.process-start-failed`. Both leave the armed lease quarantined
under Decision 0095, and both still require dependency recovery.

When a controlled model phase latched the coordinator's own exact cancellation and
another failure then decided the terminal state, the coordinator journals that signal
once. `CancellationRequested` and `CancellationObserved` follow the closed turn,
carry no turn or operation, and precede `RunTerminal`. This follows the existing
tool-failure pattern. The terminal state and unresolved codes stay those of the
deciding failure; this is not a `Cancelled` transition or an acknowledgement of a
port's stop label. A foreign signal or spoofed label still records no cancellation.

## Verification boundary

Deterministic controller, driver and coordinator fixtures cover the cleanup receipt,
cleanup uncertainty, rejected served tuples, later-turn retention, the legacy latch,
startup classification, spawn categories and displaced-cancellation journaling. The
controller and coordinator tests failed against the unfixed source before the fix.
No GPU, model process or native workflow was run for this decision. Real-model
admission, positive native workflows, manual acceptance, independent review of this
batch and release remain separate. No task or capability closes from this decision.

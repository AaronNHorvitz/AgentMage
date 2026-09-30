# Decision 0123: Review Fixes for Suspension

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088, 0120 and 0122; current owner restart |
| Scope | Findings F1 to F3 of the independent review of `3c69304c`; amends Decision 0122 items 3 and 8 |

## Findings

An independent read-only review of `8a3a341e..3c69304c` passed with three low
findings.

- F1: when the host could not read the job's phase after deciding a resumption
  or cancellation, it handed a withheld suspension back to the worker whatever
  the decision had done. After an applied resumption or cancellation the ledger
  no longer asks for a suspension, but the worker could still stop at its next
  boundary. The ledger then refused the owner's observation and the run failed.
  Nothing was lost, but Decision 0122 item 3 did not hold on that path.
- F2: when the ledger queued or cancelled a suspended job but the host could not
  continue the run, the host no longer held the run. The driver could not tell
  that from a request the host never took. It showed "the run continues
  unchanged" and waited on a run that nothing held.
- F3: three rules had no test that failed when the rule alone was removed: the
  withheld state that stops the worker from claiming a suspension while a request
  is decided, the check that only a continuing decision rebinds the driver, and
  the suspended wait that never advances the host.

## Decision

F1. After each decision the host settles the suspension in one place. When the
job's phase was read, the suspension follows it, as before. When it was not:

- an applied decision that leaves the job in any phase other than suspending
  withdrew the suspension, so the suspension is withdrawn;
- a refusal or a failed write changed nothing, so a withheld suspension goes
  back to the worker.

Without the phase, no decision arms a suspension. A retried request is answered
with the decision it was first given, which may be older than the job's current
phase. Withdrawing only ever makes the worker continue, so a stale answer can
cost a suspension but can never cause an unrequested stop.

F2. When a job control request fails while the run is suspended, the driver
reads the job's status once more. If the host answers that it no longer holds
the run, the driver shows that the host released the suspended run and that the
job keeps the phase its ledger recorded. It then ends with the runtime error
instead of waiting. This covers a resumption and a cancellation alike. Any other
answer keeps the existing behaviour: a suspension or resumption the host did not
take is shown as not taken. The new notice has both output formats; its machine
form is `run_released` with the action and the boundary.

F3. New tests isolate each rule:

- the suspension state alone: a withheld suspension is never claimed, it then
  follows the decided phase, a claimed one stays claimed and only a withheld one
  is restored;
- a driver test in which the host refuses a resumption as stale in an answer that
  describes the resumed request, which must be refused as evidence without
  rebinding;
- the existing resumption and cancellation tests now wait three polls while the
  run is suspended and assert that the host was not advanced between the stop and
  the continuation.

## Verification boundary

Host unit tests cover the settled suspension for every decision kind with and
without a phase, and the suspension state transitions. Driver tests cover a
released run for a resumption and for a cancellation, the refused answer about
the resumed request, and the suspended wait. The CLI test renders the new notice
in both formats. Single mutations of each rule made the expected new test fail;
the results are in the batch record. No test uses a native host, a model or the
GPU. Independent re-review of this batch is requested and remains open.

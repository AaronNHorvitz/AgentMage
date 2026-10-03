# Decision 0146: Review Fixes for Chain Closing at Release and Resumed Start Failures

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-03 |
| Date | 2026-10-03 |
| Authority | Decisions 0054, 0081, 0129, 0143 and 0145; current owner restart |
| Scope | Findings F1 and F2 and notes N2 and N6 of the independent review of `22a2662f`; defect D1 found by this batch's evidence pass |

## Findings

An independent read-only review of `6c0f51fe..22a2662f` (batch 30) passed
with two low findings and ten notes.

- F1 (low): Decision 0145 says a run resumed after a restart keeps its
  chains open when its composition or start fails. No test showed it. A
  change that closed them anyway left every host test passing. A later host
  could then never resume the run, because the store refuses to attach a
  closed chain.
- F2 (low): when the store refused a pending incomplete mark at release,
  the release still reported success. The service logged one line to
  standard error and forgot the run, so nothing remained to retry the close
  or tell the person why every later declaration of the session is refused.

The notes:

- N1, N3, N4, N5 and N7 to N10 need no change.
- N2: Decision 0143's corrected Limits said a later host reaches the session
  only by resuming it after a restart. A later host may also start a new run
  of the session.
- N6: Decision 0145 rightly said that only the job case of a new run that
  cannot start was tested, not the spawn case.

This batch's evidence pass found a defect of the same kind as the review's
F2 of `6c0f51fe`.

- D1: the Sprint 11 storage migration compatibility producer appends
  Cargo's output to its retained log unredacted. When Cargo recompiles, its
  progress lines name the private checkout. Decision 0145 fixed the three
  sibling producers; this one was not covered, because its committed log
  held no path at the time. The batch's first core evidence stage
  regenerated the log with two such lines.

## Decision

### F2: a release closes the run's chains, or is refused

The service now closes an ended run's stored chains when the client
releases it, before it forgets the run. That covers the run's effect record
and its action history chains.

- If every chain closes, or the store does not hold it, the release
  succeeds, as before. Dropping the session afterwards closes each chain
  again, which writes nothing.
- If a chain cannot be closed, the release is refused with
  `host.runtime.failed` and the ended run stays held. The client learns
  that the release failed. A later release retries the pending mark and the
  close. If the host ends while it still holds the run, dropping it tries
  once more.
- An effect record stays open while the store refuses its pending mark
  (Decision 0145). An action history chain stays open when the store refuses
  to close it. Retention applies only to a closed chain, so an open one
  would keep its entries past their deadline.

The service still logs `coding.live.effect-record-close-failed` or
`coding.live.history-close-failed` for each refused close, and the effect
record also logs `coding.recoverability.store-mark-failed` for each refused
mark.

The review also suggested a job owner event or a reason inside the session
declaration that names the cause. Either would change the run declarations
or the job ledger's wire contract. This batch keeps both unchanged. The
refused release is the signal the client already understands.

### F1 and N6: tests for every start failure

Three new host tests run over a real encrypted store:

- A run resumed after a restart whose session cannot spawn, and one whose
  job cannot begin because its job already ended, keep the effect record
  and the job control chain open and incomplete. A later host can still
  attach both.
- A new run whose session cannot spawn closes its effect record empty. No
  job and no job control chain were begun for it.
- A release is refused while the store refuses the effect record's pending
  mark, and the run stays held and still answers its declarations. Once the
  store writes again, the next release closes the chain as incomplete. An
  action history chain the store refuses to close, here one held by another
  owner, refuses the release the same way, and dropping the service leaves
  that chain open.

The test service's coordinator can now refuse every event subscription, so
a session cannot spawn.

### D1: the fourth store producer redacts the checkout

The storage migration compatibility producer now uses the same shared
helpers as its three siblings. It replaces the checkout root with
`<repository-root>` in the output it retains, and it refuses to write, or
to accept, retained output that still names the checkout or a home
directory. A test covers both rules.

The first core stage's output was withdrawn before any commit. The core
stage was run again after this fix, and this batch's verification record
says so. Other producers that retain raw Cargo output stay an open owner
question: each is fixed when a batch would otherwise commit the path.

### N2

Decision 0143's Limits now say that, in a later host, a run resumed after a
restart is marked incomplete, and a new run of the session is refused its
declaration because the earlier run's chain is still open.

## Limits

- The development CLI treats a refused release like any other: it prints
  `host.runtime.failed` and `cli.runtime.release_failed` and ends with the
  runtime exit code. It does not print the run's final report. The run's
  events, including its terminal event, were already printed as they
  arrived.
- While a held run cannot be released, the host refuses any resumed run and
  any other run of the same session, as it does while any run of the
  session is held. A held run also takes one of the service's run slots.
- No actual process closes or refuses a chain in this sandbox. The
  development host is refused at native Git trust, as before. The tests
  refuse writes at the effect record's write seam and by another owner's
  chain, not inside SQLite.
- Independent review remains open.

## Consequences

- `shells/host`: the live service's release and chain closing, the gated
  test coordinator, and three new service tests.
- `scripts`: the storage migration compatibility producer, with its test.
- `docs`: Decision 0143's Limits and the local testing guide.
- No TASKS.md row changes state. AMR-04.7.3 keeps its evidence. This
  batch's verification record lists the fixes.

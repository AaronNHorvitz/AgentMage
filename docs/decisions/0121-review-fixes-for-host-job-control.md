# Decision 0121: Review Fixes for Host Job Control

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088, 0119 and 0120; current owner restart |
| Scope | Findings F1 and F2 of the independent review of `8a3a341e`; amends Decisions 0119 and 0120 |

## Findings

An independent read-only review of `6355a379..8a3a341e` passed with two low
findings and one wording note.

- F1: when the CLI driver had no job state for a run, the human view said the
  host kept no durable job ledger for it. The driver folds several causes into
  that one case: a host without a ledger, a ledger that stopped following the
  run, a refused or failed status read, and an answer about another run. The view
  therefore named a cause the driver could not know.
- F2: the host's start rule had two guards, one for an existing job and one for a
  running job. The only test case used a running job, so removing either guard
  alone left every test passing. A new run can reach the second guard only with a
  freshly created job, which is always queued, so that guard could never decide.
- Note: Decision 0119 said each of its four mutations made "exactly the expected
  new or extended test" fail. Turning recursive triggers off together with their
  check made two tests fail, both expected.

## Decision

F1. The driver keeps what it observed and nothing more. A run's job state is
either the host's status for this exact run or one closed reason:

- `not_offered`: the host answered that it offers no job state for this run;
- `not_answered`: the host refused the status request or did not answer it;
- `not_described`: the host's answer did not describe this run.

The human view prints the reason in those words. The machine view adds a
`reason` field next to `"available": false`. Neither view names a ledger.

F2. Each start rule now stands alone. A new run creates its job, so it never
takes over an existing job in any phase. A run resumed after a host restart
starts a queued job, continues a running job, and refuses every other phase.
The test now also covers a queued job for a new run and for a resumed run, and a
suspended job, which only a resumption continues (Decision 0122).

The note. Decision 0119's verification boundary now says the recursive-trigger
mutation made both expected tests fail.

## Verification boundary

The CLI tests render each reason in both formats and check that no view names a
ledger. The driver test for a transport without job control expects
`not_offered`. The start-rule test covers every phase a job can be in when a run
starts: none, queued, running, suspended, cancelling and completed.

Three single mutations each made the start-rule test fail: a new run taking over
an existing job, a resumed run continuing a suspended job, and a resumed run
refusing a queued job. A fourth made the driver report `not_answered` for a host
without job control, and the driver tests failed. The results are in the batch
record. No test uses a native host, a model or the GPU. Independent re-review of
this batch is requested and remains open.

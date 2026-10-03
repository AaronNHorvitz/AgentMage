# Decision 0147: Review Disposition for the Host Code of a Refused Release

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-03 |
| Date | 2026-10-03 |
| Authority | Decisions 0054, 0081 and 0146; current owner restart |
| Scope | Finding F1 and notes N3 and N4 of the independent review of `58df8b56` |

## Findings

An independent read-only review of `22a2662f..58df8b56` (batch 31) passed
with one low finding and ten notes.

- F1 (low): Decision 0146 and the local testing guide say that the
  development CLI prints `host.runtime.failed` and
  `cli.runtime.release_failed` when the host refuses a release. The CLI code
  is printed in every case. The host code is not always the release's. The
  runtime client keeps only the first transport error of a run. Before it
  releases an ended run, the CLI reads the run's declarations and its job
  status, and it accepts that either read fails. If one did, the client
  prints that read's code, not the release's. The same holds for any
  earlier exchange that failed without ending the run, such as a
  suspension the host did not take.

## Decision

Correct the statement and keep the client's rule.

The client keeps the first transport error on purpose. When a run cannot
start, the CLI releases it as cleanup, and that release can fail too. The
start's code is the cause the person needs, and it is the one printed. A
client that printed the latest error would show the cleanup's code instead.
Carrying the release's own error into the CLI's release error would change
the CLI error type for one rare case, while the CLI code already says that
the release failed.

Decision 0146's Limits and the local testing guide now say that the CLI
prints the run's first transport error, which is `host.runtime.failed`
when no earlier exchange of the run failed, and then
`cli.runtime.release_failed`.

The notes:

- N3: no test shows directly that a release refused because the run has not
  ended, or because the request digest differs, leaves the run's chains
  open. The suspension test catches the reviewer's change that closes the
  chains before those checks. A direct test is added with the next change
  to the live service. This decision changes no source.
- N4: the withdrawn core stage's own record does not say that it was
  withdrawn. Retained stage records are not edited after the fact. The
  verification record, its specification and the scope of the stage that
  replaced it say so.
- N1, N2 and N5 to N10 need no change.

## Limits

- This changes documentation only. No source, test or evidence changes. No
  actual process has refused a release in this sandbox (Decision 0146).
- Independent review remains open.

## Consequences

- `docs`: Decision 0146's Limits and the local testing guide.
- No TASKS.md row changes state.

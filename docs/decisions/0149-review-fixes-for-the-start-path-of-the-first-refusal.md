# Decision 0149: Review Fixes for the Start Path of the First Refusal

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-03 |
| Date | 2026-10-03 |
| Authority | Decisions 0054, 0081, 0147 and 0148; current owner restart |
| Scope | Finding F1 and notes N1 and N3 of the independent review of `f33245fb` |

## Findings

An independent read-only review of `7ddc1713..f33245fb` (batch 33) passed
with one low finding and eight notes.

- F1 (low): Decision 0147 keeps the runtime client's first-refusal rule so
  that a failed start's code, not the code of the release that cleans it
  up, is the one the development CLI prints. The test Decision 0148 added
  never called `start` or `advance`. A `start` that cleared the kept
  refusal before its exchange left every host test passing.
- N1: the record states each named test's result as a literal from its
  specification rather than deriving it from the hash-bound log.
- N3: the test channel ignored the receive bound.
- N2 and N4 to N8 need no change.

## Decision

- F1: the test now prepares a run, has its start refused, has the cleanup
  release refused too, and checks that the start's code is kept, also
  after a later refused advance. It then prepares again, has an advance
  refused with a code of its own, then the cleanup release and a later
  start, and checks that the advance's code is kept. A start or advance
  that cleared the kept refusal, or that bypassed the client's exchange,
  now fails the test. The rule does not change.
- N3: the test channel refuses a frame above the receive bound, as the
  authenticated session does.
- N1 is not changed in this batch. `scripts/verification_batch.py` is a
  bound input of 297 retained artifacts, so a change there would stale them
  all for a claim the retained, hash-bound log already lets a reader check.
  The record of this batch names the test and its log as before.

## Limits

- These remain host-unit tests over an in-process channel, not an
  authenticated session between actual processes.
- No actual process has refused a start and its cleanup release in this
  sandbox.
- Independent review remains open.

## Consequences

- `shells/host`: the first-refusal test and its test channel.
- `docs`: Decision 0148's Limits and the local testing guide.
- No TASKS.md row changes state.

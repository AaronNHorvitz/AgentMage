# Decision 0148: Review Fixes for the First Refusal of a Run and Refused Releases

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-03 |
| Date | 2026-10-03 |
| Authority | Decisions 0054, 0081, 0146 and 0147; current owner restart |
| Scope | Findings F1 and F2 of the independent review of `7ddc1713` |

## Findings

An independent read-only review of `58df8b56..7ddc1713` (batch 32) passed
with two low findings and seven notes.

- F1 (low): Decision 0147 keeps the runtime client's first-refusal rule on
  purpose, and Decision 0146 and the local testing guide now rely on it. No
  test showed the rule. A client that kept the latest refusal, or a
  `prepare` that no longer cleared it, left every test passing.
- F2 (low): Decision 0147 promised, under note N3 of the review of
  `58df8b56`, a direct test that a refused release leaves the run's chains
  open. The promise was written only in that decision's prose, where no
  check or record would show it was kept.
- N1 to N7 need no change. Under N3 an answer of an unexpected kind still
  records no refusal; the live service never sends one.

## Decision

Add both tests in this batch, and name them in its retained record.

- F1: the runtime client is built over a frame channel. Outside tests its
  only channel is the authenticated Linux session: `new` takes only that
  session, and only tests build a client over another channel. A test
  drives the real client over an in-process channel whose frames the host's
  own answer handles. It shows that a client with no refusal reports none,
  also after a successful exchange; that of two refused reads the first is
  kept, also after a later success; that a release refused after a refused
  read keeps the read's code, which the development CLI prints; that the
  next `prepare` clears it; that a release refused with no earlier refusal
  keeps its own code; and that a refused `prepare` is itself the first
  refusal. The rule does not change.
- F2: a test of the live service releases a run that has not ended, under
  its own and another request digest, and the ended run under another
  digest. Each release is refused and closes neither the run's effect
  record nor its job control chain, and the run is still held. The exact
  release then closes both. Decision 0147's Limits point here.

## Limits

- These are component and host-unit tests. The in-process channel is not an
  authenticated session, and no actual process exchanges these frames here.
  The authenticated session's own frame handling is unchanged.
- No actual process has refused a release in this sandbox (Decision 0146).
- Independent review remains open.

## Consequences

- `shells/host`: the frame channel of the runtime client and both tests.
- `docs`: Decision 0147's Limits and the local testing guide.
- No TASKS.md row changes state.

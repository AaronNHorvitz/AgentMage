# Decision 0140: Review Fixes, and Research Persistence Through the Coordinator

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0084, 0097, 0136, 0137, 0138 and 0139; current owner restart |
| Scope | Finding F1 and notes N1, N2 and N8 of the independent review of `f0e85caa`; AMR-03.1.1 |

## Findings

An independent read-only review of `b59afdb9..f0e85caa` (batch 24) passed
with one low finding and ten notes.

- F1: two clauses of the issuance owner had no test. One is the budget
  owner's match of the run's context in the non-spending check; the other
  is the owner's own usability check. Removing either left all 26 tests
  passing. Every composed case sent the run's own context, and none asked
  for a grant while an effect was pending. Without the context match,
  nothing else at issuance checks the run identity. Without the usability
  check, an issuance during a pending effect would commit a grant revision
  while a terminal was pending.
- N1: step 7 of Decision 0139 reads as a claim about the person. The
  parent's preview is the trusted host's attestation of the plan the person
  confirmed, as the decision's limits already say.
- N2: the owner decodes the call's argument bytes before it bounds their
  size and checks their digest. Decoding them is harmless, but a direct
  caller could hand the owner unbounded bytes.
- N8: the composed test leaves its store directory behind when a test
  panics. In a sandbox that reuses process identifiers, a later run can
  meet that directory and fail in an unrelated test.

The other notes need no change. N3 says a content-free refusal cannot tell
a secret apart from a scope error, which matches the reservation. N4 is
unreachable. N5 confirms the destination gap is now closed earlier. N6 is
the derived disposition record. N7 is the missing fuzz target. N9 is the
ledger stage. N10 confirms the record's claims.

AMR-03.1.1 asks that full plans and conservative reservations persist
through the existing owner before native dispatch. It lists the properties
to verify: bounded snapshot recovery, original clocks, terminal
cancellation and expiry, exact task and plan binding, failed-attempt
consumption, concurrent admission, and no replay after interruption. The
budget owner already has each of these, and its own tests cover them. The
[AMR assessment](../verification/amr-assessment-2026-09-29.md) named the
remaining gap: nothing composed the owner into a run. Decisions 0137 to 0139
now compose it with the coordinator, up to the native worker. The
coordinator publishes the plan and opens the budget, the owner issues the
grant, and the trusted port reserves each request before the start.

Composing these found one gap. When the coordinator cancels an admitted
run, nothing cancels the run's task budget. Dispatch is still refused,
because the owner refuses a requested operation after the run's
cancellation and any request of a run that has ended. But the budget's own
accounting never records the cancellation, so its projection still shows
the budget open.

## Decision

### Review fixes

F1. The composed probes add three budget contexts: one of another run of
the task, one of another session and one under another policy digest. Each
is refused as a binding error by the budget owner. Without the context
match, another session would reach the parent clause and another policy
the policy clause, and the owner would accept another run's request. A new
test sends the owner a valid request for the run's next call while the
first call's effect is pending. The owner refuses it as unusable, and the
first effect then completes. The batch's mutation specification removes
each clause.

N1. Decision 0139 step 7 is corrected in place. The trusted host attests
that the parent's preview is the plan the person confirmed. The canonical
research source document is corrected the same way.

N2. The owner refuses argument bytes that are empty, larger than the call
validator's bound of 16 KiB, or not bound to their digest, before it
decodes them. The call validation still repeats both checks against the
packet. A probe with oversized arguments is refused as a call mismatch. The
non-spending check writes nothing, so the new order changes no outcome;
the probe pins the bound for issuance.

N8. Each composed test's store directory carries the clock in its name. It
is removed when the test's port is dropped, also when the test panics, and
only after the owner has closed.

### Cancellation reaches the task budget

The research budget port gains `cancel_research_budget`. Before the
coordinator seals the outcome of an admitted run that has observed a
cancellation, it has the port cancel the run's task budget. The cancellation
may have been observed in any of three ways:

- between phases, including while a request waits for approval;
- through a cancelled tool's outcome;
- recorded after a failure that took precedence.

The coordinator accepts the owner's answer only when it describes the plan
the run published, the admission's scope and a cancelled budget.
Otherwise, or when the port fails, the run still ends in the state it
reached, and its outcome names
`runtime.research.budget_cancellation_unconfirmed`. The coordinator does not
retry, and nothing claims that an effect was undone. A run that observed
no cancellation leaves its budget alone. When the run ends otherwise, its
terminal already stops further reservations.

The coordinator keeps the published plan's reference once the owner has
opened its budget. A cancellation before that point cannot happen: the
budget is opened when the run starts, before the first phase.

### Persistence through the coordinator

The synthetic native worker can now report a failed, timed-out, cancelled
or uncertain attempt after it checks both proofs. Such an attempt seals no
result, and the owner's receipt records its outcome. The trusted test glue
commits that receipt's terminal with an attempt that retained nothing,
which is the result the coordinator admits for these outcomes.

A new composed module drives the real coordinator and the real owners
through these cases:

- the glue reserves the first request and stops before the start, as if
  its process ended. The reservation is durable and the grant unspent.
  Reopening the store recovers the same revision, head, counts and
  original clocks. The spent request is never reserved again, and a new
  request counts on from the recovered snapshot;
- after that reopening, a rolled-back clock or one at the plan's elapsed
  limit ends the budget. The end is retained and survives another
  reopening, also at a valid clock;
- the glue begins the first effect and stops before the terminal. While
  the terminal is pending, the owner refuses everything else. Reopening
  closes the transaction with the worker's recorded outcome. The grant and
  the visit stay spent, no terminal event names the call, nothing is
  published, and nothing is dispatched again;
- a worker that fails, times out or becomes uncertain ends the run in the
  matching state, under the owner's receipt. The visit and its worst-case
  bytes stay spent across reopening;
- a cancellation before any request, one while a second request waits for
  approval, and one that arrives during the effect each cancel the budget
  durably. Spent visits stay spent; an unapproved request is never issued
  or reserved, and a later reservation is refused as cancelled;
- the coordinator's clock passes the plan's elapsed limit between issuance
  and the start. The start's reservation is refused and the expiry is
  retained, with nothing spent or started and the grant unspent. The
  expiry survives reopening, at a later or an earlier clock;
- while the first owner is open, a second owner of the same store is
  refused. Another run of the same task, with the same plan or another one,
  is refused when its budget would open. The task's budget is unchanged
  and stays bound to the run that opened it.

Synthetic coordinator tests check that the budget is cancelled once,
after the cancellation is observed and before the terminal, both between
phases and through a cancelled tool. They check that each unconfirmed
answer is named in the outcome, and that a run without a cancellation
leaves the budget alone.

AMR-03.1.1 closes as a component row, engine only, when the batch is
verified. Its prerequisites are AMR-02.2 and the canonical artifact and
authority store; none is native. The native dispatch itself stays with
AMR-03.1.2, which needs the native boundary of AMR-02.3.2, and with
AMR-03.2.3.

## Limits

- Engine only. No host, worker, provider, model or network composes or
  uses the admission. The payload store is in memory, the native result is
  synthetic, and the glue is test code.
- A start that is refused or interrupted leaves the run without a
  terminal. As with every port failure, the host must reconcile it.
  Resuming an admitted run is still later work.
- The non-spending check at issuance records nothing. An expiry it
  observes is retained only when a start reserves.
- A second owner is refused by the store's exclusive connection, which
  the store's own tests already cover. The composed test shows that it
  holds while a run's owner is open.

## Consequences

- `kernel/engine/src/runtime_loop.rs` changes:
  - the research budget port's new method;
  - the budget cancellation before the outcome;
  - the plan reference kept after the opening.
- `kernel/engine/src/research_grant.rs` bounds the arguments before decoding
  them, and `research_effect_binding.rs` shares its bound.
- `kernel/engine/src/research_dispatch.rs` gives the synthetic worker an
  outcome.
- The admission and composed test modules gain the review-fix and
  cancellation tests and the fixture changes. The new module
  `runtime_loop_research_persistence_tests.rs` holds the persistence cases.
- `docs/architecture/reusable-runtime-coordinator.md` and
  `docs/architecture/canonical-research-source.md` describe the
  cancellation and the persistence through the coordinator.
- TASKS.md: AMR-03.1.1 is recorded with its evidence when the batch is
  verified, as a component row.

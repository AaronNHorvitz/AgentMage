# Decision 0142: Review Fixes, and Research Reports Through the Coordinator

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-02 |
| Date | 2026-10-02 |
| Authority | Decisions 0054, 0084, 0097, 0136, 0137, 0138, 0140 and 0141; current owner restart |
| Scope | Finding F1 and notes N1, N2 and N4 of the independent review of `a8fd53e3`; new component row AMR-03.1.4 |

## Findings

An independent read-only review of `f0e85caa..a8fd53e3` (batches 25 and 26)
passed with one low finding and eleven notes.

- F1: Decision 0140 cancels an admitted run's budget after a cancellation
  observed by any of three paths. The third path, a signal latched during a
  model phase that a dependency failure then outranks, had no test. The run
  ends `Failed`, and the observation is emitted only inside the terminal
  step. Two mutations survived every test: one cancelled the budget only for
  a `Cancelled` terminal, the other asked the owner before the latched
  observation was emitted.
- N1: the owner refuses every call while an effect's terminal is pending
  with the same `Poisoned` error it returns for a poisoned store.
- N2: step 2 of Decision 0139 did not show the argument bound and digest
  check that Decision 0140 placed before decoding.
- N4: the worker contract says a failing worker "writes no frame". A worker
  whose frame write fails may already have written part of it.

The other notes need no change. N3 confirms the record's wording, N5 and
N6 describe conservative behaviour, N7 is the derived disposition record,
N8 the missing fuzz target, N9 the ledger stage, N10 a counting convention
and N11 the agreement of the records with the tests.

AMR-03.1 asks for bounded quick and deep plans and source-backed reports
under canonical artifact ownership. The canonical report owner already
checks an untrusted draft against complete source bundles and the run's
accounting, retains only the exact draft through its own entry, and reads
a retained draft back only through fresh checks. Only tests call it. The
coordinator, which owns a run's publication and journal, has no report
path: an admitted run can retrieve sources but cannot publish a report
of them.

## Decision

### Review fixes

F1. A synthetic admitted run's model phase observes the cancellation and
then reports a dependency failure. For an uncertain and for an unavailable
dependency the run ends `Failed`, and for an exhausted resource it ends
`Exhausted`. In each case the outcome names the failure's own code, the
cancellation is observed after the closed turn, and the budget owner is
asked exactly once, after the observation and before the terminal. The
batch's mutation specification adds both surviving mutations.

N1. The canonical research source document says that the error does not
by itself mean damage, and how a pending effect is closed.

N2. Decision 0139 step 2 is corrected in place.

N4. The worker contract says that a failing worker writes no complete
frame. After status 5 the parent never treats standard output as a frame,
which the Linux parent already enforces.

### Reports through the coordinator

New component row AMR-03.1.4: an admitted run publishes its report through
the coordinator and the canonical report owner.

The research budget port gains `publish_research_report`. The trusted host
implements it over the report owner. It decodes the draft, supplies the
independently admitted native identity and has the owner check the draft
at the manifest's creation instant and retain it. The owner's existing
checks apply unchanged:

- every source is a complete public GET bundle of this run;
- every excerpt is the exact byte range of its source body, within the
  quotation bounds;
- the accounting of the run's own plan is read;
- the run has not ended;
- the manifest's classification, retention and policy cover every source.

A refusal returns `Invalid` and retains nothing. Any other failure is a
dependency failure, as for every publication.

The coordinator treats a completion whose payload has the report draft
media type as a report only in an admitted run:

1. The payload must decode as a draft and be its exact canonical encoding,
   because the owner retains the draft's own encoding under the
   coordinator's manifest. Anything else is an invalid proposal, and the
   owner is not asked.
2. The verifier runs as for every completion. When it fails, the run ends
   `Failed` without output: an unverified draft is never retained.
3. The output and artifact budgets are charged as for any retained output.
4. The coordinator prepares the draft's manifest: a run-level report of
   the current turn, with no operation, receipt or preview. It flushes the
   journal and asks the port to publish the draft.
5. On a refusal the run ends `Failed`, and its outcome names
   `runtime.research.report_refused` without output.
6. Otherwise the coordinator accepts only the reference its own manifest
   describes, records the draft's creation event and completes with the
   draft's reference as the output.

A cancellation observed after the publication still cancels the run and its
budget, and the retained draft then reads back as cancelled. Outside an
admitted run's completion, a payload with this media type is an invalid
proposal; a draft is retained only through the report owner.

The draft's claims are the model's. Publication proves only what the owner
checked when it retained the draft: the excerpts are present in complete
sources of this run. Every later read rebuilds the report through fresh
source, accounting and lifecycle checks, as before. Its disposition is
checked, partial, cancelled or expired.

A new composed module drives the real coordinator and the real owners. A
deep plan with three disclosed queries and three visits makes three
searches and two visits, and the run completes with a draft that cites one
search result and both pages. The tests then check:

- the retained draft reads back through the retained-report reader as
  checked;
- a draft with unresolved questions reads back as partial;
- after the plan's elapsed limit the draft reads back as expired;
- a cancellation after the publication reads back as cancelled, with the
  budget cancelled;
- after source drift, every read is refused;
- after reopening, the same report reads back, with no request reserved
  or dispatched again;
- the owner refuses, and nothing is retained for, a draft that cites the
  plan instead of a source, a forged reference to a source, an excerpt
  absent from its source, an excerpt bound to another body, and more
  sources than the plan's visits;
- a draft that names one source twice, or that is not its own exact
  encoding, never reaches the owner.

Synthetic coordinator tests cover the manifest, the flush before the owner
is asked, the creation event after its answer, each refusal of a draft
outside an admitted run's exact completion, an unverified draft, a refused
or failed publication, a mismatched reference, an exhausted output budget
and a cancellation after the publication.

AMR-03.1.4 closes as a component row, engine only, when the batch is
verified. AMR-03.1 stays open for AMR-03.1.2 (the native boundary) and
AMR-03.1.3 (the external consumer pin).

## Limits

- Engine only. No host, worker, provider or model composes the admission
  or publishes a report. The payload store is in memory, the native result
  is synthetic, and the glue is test code.
- The synthetic model takes the bundle references from the test glue. A
  real model can cite a source only when the context owner shows it the
  checked references and bodies. That presentation is later work for the
  research campaign rows (AMR-03.2.3 and AMR-03.2.4).
- The verifier is the run's ordinary verifier. No research-specific
  verifier judges whether a report answers its task.
- A cancellation that arrives after the publication cannot withdraw it.
  The draft stays retained, and its reads say the run was cancelled.

## Consequences

- `kernel/engine/src/runtime_loop.rs` changes:
  - the research port's new method and the run's publication hook;
  - the report path in completion verification;
  - the refusal of the draft media type outside an admitted completion.
- `runtime_loop_research_admission_tests.rs` gains the F1 test and the
  synthetic report tests. The new module
  `runtime_loop_research_report_tests.rs`, under the composed completion
  tests, holds the report cases through the real owners.
- `docs/architecture/reusable-runtime-coordinator.md` and
  `docs/architecture/canonical-research-source.md` describe the report
  path; the worker contract and Decision 0139 are corrected as above.
- TASKS.md gains AMR-03.1.4. It is recorded with its evidence when the
  batch is verified, as a component row.

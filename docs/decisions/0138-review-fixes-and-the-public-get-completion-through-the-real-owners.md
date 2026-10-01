# Decision 0138: Review Fixes, and the Public GET Completion Through the Real Owners

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0084, 0096, 0097, 0106, 0135, 0136 and 0137; current owner restart |
| Scope | Findings F1 to F3 and notes N1 and N2 of the independent review of `7611c8bf`; AMR-03.1.2.1 |

## Findings

An independent read-only review of `4bef629b..7611c8bf` (batches 21 and 22)
passed with three low findings and twelve notes.

- F1: the producer contract named three omitted members of the transport.
  An approval response's `selection`, which an `advance` request carries,
  is a fourth. An explicit `null` for it is refused, but the document did not
  say so.
- F2: an admitted network attempt that was uncertain, or that changed
  nothing, could carry output, evidence and artifact candidates. The
  coordinator retained the output and published the candidates under the
  call's receipt. Nothing was granted or completed, and the canonical reader
  cannot read a partial bundle. The bytes still had no trusted provenance.
  An uncertain ordinary read must carry none of these.
- F3: five checks of the admission had no test. Two are the task and session
  clauses of the request binding. Three are the start-time budget answers:
  spent queries, reserved bytes and an exhausted deadline.
- N1: Decision 0137 and a source comment said the plan digest covers the
  task, the scope and the network mode. It covers the plan's whole canonical
  bytes, so the binding is stronger than stated.
- N2: the TASKS row AMR-03.2.2 said the coordinator admits a network
  operation "only through the existing consumed grant, durable reservation
  and dual-proof dispatch". The coordinator does not verify those. They are
  the trusted port's obligations.

The other ten notes need no change. N3 is the derived disposition record. N4
says the number sentence is complete. N5 records that runtime events are read
by type and verified at the store's verification points. N6 is cost. N7 and
N10 concern the test helpers and the missing fuzz target. N8 and N9 cover the
Git checkout tests and the retained ledger stage. N11 and N12 need no
action.

AMR-03.1.2.1 is dependency-ready, because AMR-03.2.2 is recorded. The
coordinator's side of an admitted run is tested only against synthetic
ports. The research owners are each tested on their own: the budget owner,
grant issuance and consumption, the authority transaction with its start
observer, dual-proof dispatch, completion normalization and the canonical
reader. No test composes them with the coordinator.

Composing them found one defect in the authority owner. Its terminal commit,
`finish_effect_with_runtime_event`, checked the pending receipt it was given
but not the terminal event. The trusted glue could hand completion a receipt
under another identity, correctly sealed, together with a transaction
snapshot that named it. Completion checks only that the descriptions agree,
so it accepted them. The owner then committed a `ToolCompleted` that named a
receipt it never issued. The coordinator reported success and published the
six artifacts under that receipt. The canonical reader refused the bundle,
because it reads the owner's own receipt, but the journal recorded a
completion that the authority never issued.

## Decision

### Review fixes

F1. The contract document names the four omitted members. The wire test
sends an `advance` request whose approval response has no selection. It
decodes. The same request with `"selection": null` is refused.

F2. For the admitted tool, only a success carries output, evidence or
artifact candidates. An uncertain attempt, and a failure, denial,
cancellation or timeout that changed nothing, must carry none of them. The
coordinator refuses such a result before publishing anything. The result
matrix and a coordinator test cover each case.

F3. The composition test adds a request of another task and one of another
session, each naming the admission exactly. The budget test adds owners that
answer with spent queries, spent bytes and an exhausted deadline.

N1. Decision 0137 and the comment say the plan digest covers the plan's whole
canonical bytes. Decision 0137 is corrected in place.

N2. The TASKS row says that reservation, grant consumption and dual-proof
dispatch are the trusted port's obligations, which AMR-03.1.2.1 composes.

### The terminal names the owner's pending effect

The authority owner commits a pending effect's terminal only when it closes
that effect. The terminal must be a completed or failed tool event that names
the start's call and the receipt this owner issued, in the start's turn and
operation. Otherwise the owner refuses the commit and poisons itself, as it
already does for a pending receipt it does not hold. The store's journal
already refuses a terminal that does not directly follow the start, or that
names another run, session or task. After reopening, the effect is a known
effect with no completion, and it cannot be dispatched again. Every
existing caller already builds such a terminal: the coordinator's builder,
the coding host and the research fixtures.

### AMR-03.1.2.1: the completion through the real owners

A new engine test module composes the real coordinator with the real owners
under the AMR-03.2.2 admission. Only the native result is synthetic.

The owners are the engine's own:

- the encrypted operational store and its journal;
- the budget owner, which opens the budget for the plan the coordinator
  publishes and reserves each request;
- grant issuance: one session parent grant and one exact single-use network
  grant per call;
- the authority transaction, which consumes the grant and commits the start
  before dispatch;
- the fresh-reservation dispatch proof;
- completion normalization through the coordinator's borrowed builder;
- the terminal commit;
- artifact publication;
- the canonical reader.

The payload store is in memory. The test's trusted glue implements the
coordinator's ports over these owners. It does what Decision 0137 requires of
the trusted port:

1. Before execution, it prepares the exact packet from the call's arguments.
   It issues the exact grant together with the permission event, under the
   plan's task-authorized mode.
2. At the start time, it reserves through the budget owner. It begins the
   effect with the coordinator's start observer.
3. The worker receives two proofs: the consumed grant and the fresh
   reservation. It is a synthetic driver in the existing audited effect-test
   boundary, `research_dispatch.rs`. It checks both proofs and seals one
   complete result from the reservation it was given. It sends nothing.
4. The glue normalizes the completion through the borrowed builder. It
   commits the terminal through the owner.

The coordinator then publishes the six artifacts under the receipt, and the
canonical reader reads the source back.

The cases are these:

- The positive case. The canonical journal equals the coordinator's stream.
  One visit of the request's size is spent. There is one receipt. Six
  artifacts are published under it, and the reader returns the exact frame.
  After the store is reopened, the reader returns the same source. The same
  request cannot be reserved again.
- A forged receipt digest. The glue hands completion the owner's receipt
  with a forged digest, and a snapshot that names the same digest. Only the
  receipt's canonical representation can refuse it, and it is refused before
  any artifact is prepared. Nothing is published. After reopening, the owner
  keeps the known effect: the consumed grant, the terminal transaction and
  its receipt. There is no completion and no replay.
- A receipt the owner did not issue. The glue hands completion a correctly
  sealed receipt under another identity, and a snapshot that names it.
  Completion accepts the agreeing descriptions. The owner refuses the
  terminal that names that receipt, so nothing is published or completed.
- A changed terminal. The glue reseals the terminal with one binding
  changed: the receipt, the call, the operation, the turn, the session, the
  task, the run, the sequence, the predecessor, or a failure kind without a
  receipt. Each is refused: by the owner's new check, or by the store's
  journal for the run, session, task, sequence and predecessor.
- A stale receipt. The glue hands the second call's completion the first
  call's receipt and transaction. It is refused before preparation. The first
  source still reads. The second call is a known effect with no completion.
  Neither request can be dispatched again.
- Partial bundles. In one case the frame's publication fails, so the result
  and the bundle are never published. In the other, every payload is placed
  but the bundle's creation event fails. Neither bundle is readable, before or
  after reopening.
- Source drift. After a successful read, a retained payload changes or
  disappears, the expected native identity changes, or the read names another
  task. Each read is refused, before and after reopening.

Apart from the terminal check above, no source outside the tests changes for
AMR-03.1.2.1. The coordinator, the other owners and the reader compose as
they are.

## Limits

- AMR-03.1.2.1 is a component row. The glue is test code, and the native
  result is synthetic. No host, worker, provider, model or network takes
  part. The payload store is in memory.
- Completion normalization still checks only that the descriptions it is
  given agree. The owner's terminal check and the canonical reader are what
  bind a completion to what the authority recorded.
- Grant issuance under task-authorized mode is the test glue's choice. A host
  that issues grants for an admitted run needs its own decision under
  AMR-03.2.3, with the native adversarial matrix of Decision 0084.
- AMR-03.1.2 still needs the native boundary. AMR-03.2.3 and AMR-03.2.4 stay
  open.

## Consequences

- `kernel/engine/src/runtime_loop.rs`: the F2 rule and the N1 comment.
- `kernel/engine/src/operational_store.rs`: the terminal check in
  `finish_effect_with_runtime_event`. The store schema does not change.
- `kernel/engine/src/runtime_loop_research_admission_tests.rs`: the F2 and F3
  cases, and the new completion module.
- `kernel/engine/src/runtime_loop_research_completion_tests.rs`: the
  AMR-03.1.2.1 cases.
- `kernel/engine/src/research_dispatch.rs`: the synthetic worker, in its
  test module.
- `shells/host/src/runtime_ipc.rs`: the F1 wire case.
- `docs/architecture/runtime-producer-contract-v1.md`: the four omitted
  members. No fixture, record, encoding or version changes.
- `docs/architecture/reusable-runtime-coordinator.md`: the F2 rule and the
  composed test. A misplaced row of its port table is returned to the table.
- Decision 0137: the corrected digest sentence.
- TASKS.md: the AMR-03.2.2 wording, and AMR-03.1.2.1 when the batch is
  verified.

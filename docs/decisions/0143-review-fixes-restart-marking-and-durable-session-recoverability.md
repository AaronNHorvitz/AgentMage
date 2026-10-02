# Decision 0143: Review Fixes, Restart Marking, and Durable Session Recoverability

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-02 |
| Date | 2026-10-02 |
| Authority | Decisions 0054, 0081, 0108, 0116, 0117, 0122, 0129, 0135 and 0142; current owner restart |
| Scope | Findings F1 and F2 of the independent review of `8842c770`; defect D1; new component row AMR-04.7.3 |

## Findings

An independent read-only review of `a8fd53e3..8842c770` (batch 27) passed
with two low findings and seven notes.

- F1: the composed deep research run sets `max_turns` to 12 for a run of
  six turns. Only a test comment explained why.
- F2: if a run ends between the owner retaining its report draft and the
  run's terminal, the draft has no creation event. It can never be read.
  No document said so.

The notes need no change. N1 and N2 describe conservative behaviour. N3 is
a counting convention. N4 is the missing fuzz target. N5 is the private
mutation runner. N6 is the dirty tree the review ignored. N7 confirms the
corrected texts.

A read-only survey of the coding host for AMR-04.7.2 found a defect.

- D1: Decision 0129 marks the effects chain of a run resumed after a host
  restart as incomplete. The factory reads the host's resume flag to tell
  that case from a run continued in the same host. But preparing the resumed
  run clears the flag, and preparing always comes before composing. So a run
  resumed after a restart attached its effects chain as if it had continued
  in the same host. That chain still claimed to be complete. The route chain
  was not affected, and neither was the job control chain, which the service
  begins from the request.

AMR-04.7.2 asks for two things. One is a native proof through the actual
CLI and host. The other is a recoverability declaration that covers the
whole session durably. Today a declaration covers one run (Decision 0116).
Its effect record lives in the memory of the host that ran the run. A run
continued in the same host after a suspension, or resumed after a restart,
declares nothing. No declaration covers a whole session. The session part
needs no native host except for the boundary glue. The store, the recorder,
the session declaration, the service, the wire and the CLI can all be built
and tested here, as AMR-05.9.6 did for action histories.

## Decision

### Review fixes

F1: the composed deep run now asserts that it used six turns, below its
limit of 12. It also asserts that its artifacts fit the allowance for 12
turns but not the allowance for six. Decision 0142 states the fixture's
turn and tool-call limits and the reason for them.

F2: Decision 0142's Limits and the canonical research source document now
say what happens to a draft retained without its creation event. Every read
refuses it, it stays in the payload store until retention removes it, and a
host should expect it after an interrupted completion. No extra test was
added, because the owner's reader already requires the event.

### D1: the restart belongs to the prepared run

Each prepared run records whether it was resumed after a restart. Only the
preparation of a resumed run sets that flag. Composition reads it through
one function, which maps a request to how its stored chains begin:

- a request without a cursor begins new chains;
- a run resumed after a restart marks its chains incomplete;
- a run continued in the same host attaches to its chains as they are.

The host's resume flag still allows only one resume per host process.

### AMR-04.7.3: durable session recoverability

AMR-04.7.2 is split. The new component row AMR-04.7.3 holds the durable
session scope. AMR-04.7.2 keeps the native proof, which now includes showing
the session declaration.

Store. Operational store migration 25 adds three tables for run effect
records:

- A root per run. It is immutable and names the run's session, task, owner
  and one-based position in its session. A session holds at most 64 runs.
- A record per position, with its sequence, its chained entry digest and the
  canonical encoding of one closed entry, at most 4 KiB.
- A head per run, with its entry count, head digest, a complete flag and a
  closed flag.

An entry is one of two things:

- an execution: the operation, what it can do (write, create with its path,
  command or no effect), its outcome or unknown, and whether it changed
  state;
- a publication: the exact reference of a change record and the policy
  revision it was published under.

Triggers enforce these rules:

- Roots, records and heads are never deleted.
- Roots and records never change.
- A head only moves forward, and only to the record it names.
- The complete flag can only be cleared and the closed flag only set.
- Closing a chain does not change its count.
- A closed head never changes again.

The migration first verifies the prior schema history. It stays retryable
at version 24.

`DurableRunEffectRecords` shares the store's lock and offers `create`,
`attach` (optionally marking the chain incomplete), `append`,
`mark_incomplete`, `close`, a run read and a session read. Rules:

- Every append replays the chain and then commits the record together with
  the advanced head, in one immediate transaction that names the head it
  read.
- Every read and every store open replay every chain. Each entry must
  decode to the closed shape, be stored in its canonical encoding and chain
  to the stored head. Anything else poisons the store.
- A run holds at most 1,024 positions. A session read loads at most 2,048
  positions across its runs.
- Only the run's recorded owner may change its chain.
- The derived export lists the records and heads by digest.

Recorder. The tool boundary's effect recorder can be persisted. It keeps each
execution and each verified change record in memory first, then appends it
to the run's chain:

- If the memory record cannot keep an entry, both the memory record and the
  stored chain are marked incomplete.
- If only the store refuses, the stored chain is marked incomplete and the
  recorder remembers the miss. The run's own declaration, which comes from
  memory, stays available, but the chain is never used for the session.

How a chain begins follows Decision 0129:

- a new run creates its chain at the session's next position;
- a run continued in the same host attaches to its chain;
- a run resumed after a restart attaches and marks its chain incomplete;
- a run recorded before schema 25 begins a chain marked incomplete;
- a chain naming another session or task is refused.

Composition. The factory begins the effect chain after the native manifests
are verified and before the boundary exists. It then hands the store handle
to the host service. Failures are handled this way:

- If the session already holds 64 runs, or a continued run has no chain,
  the run runs without a stored record. Its session is never declared again,
  because a declaration needs the declaring run's own chain.
- Any other store failure fails the composition.
- If the composition fails after a new chain was created, that chain is
  closed empty.
- The service closes the chain together with the run's action history
  chains, when the client releases the ended run or the service drops an
  ended run it still held.

Session declaration. At the end of each run the effect owner declares the
recoverability of every effect of the session. It reads the stored chain of
every run of the session, oldest first, and checks these conditions:

- The declaring run must be the session's last stored run, and still open.
- Every earlier run must be closed.
- Every chain must be complete and name the session and task.
- Every change record must read back from the canonical artifact store,
  under the run and policy revision that published it, as that run's own
  verified record.
- The declaring run's own recorder must have missed nothing.

Each run's effect list is built from its own entries as for a run
declaration (Decision 0116). The lists are joined in session order and
assessed against the held worktree's current bytes, as one declaration that
names no run. So writes to one path across runs chain newest first. An
operation that appears in two runs is refused. A declaration is refused for
a session with more than 512 effects, a change record over 4 MiB, or more
than 32 MiB of change records.

A run continued in the same host now has a session declaration, even though
its own run declaration stays unavailable. A session with a run resumed after
a restart is never declared again.

Wire and CLI. Run declarations schema 5 adds `session_recoverability`. The
service asks for it for every ended run, resumed or not, at the host's
clock. The Linux IPC wire version becomes 16.

The CLI verifies the session declaration as it verifies the run declaration,
except that the session declaration must name no run. A session declaration
that fails is dropped alone. After the run's declaration, standard error
shows `recoverability of this session's effects: ...`, or says that it is
unavailable. JSON output adds `session_recoverability_available` and
`session_recoverability`. A declaration grants nothing. Every inverse write
it lists still needs its own fresh approval.

Producer contract. Under its change control, the runtime producer contract
moves to version 2:

- `fixtures/runtime-producer/v2` holds the run declarations at schema 5,
  with the session part, and wire 16.
- `docs/architecture/runtime-producer-contract-v2.md` describes it.
- Version 1 keeps every byte its manifest names. A test shows that its run
  declarations are dropped whole by the current client.

## Limits

- The boundary's persistence glue, its reads of change records from the
  artifact store, the factory's composition and the service's closing in a
  real host run only on a native host. This sandbox refuses the development
  host at native Git trust. So no actual process has stored or declared a
  session's effects yet. The store, the persisted recorder over a real
  encrypted store, the session declaration, the service's declarations, the
  wire, the CLI and the contract fixtures are tested here.
- The store might refuse both an append and the mark that follows it. The
  chain then claims to be complete while it misses that entry. The recorder
  remembers the miss, so the host that ran the run never declares the
  session from that chain. A later host reaches the session only by resuming
  it after a restart, and that marks the resumed run incomplete.
- A session is declared only at the end of one of its runs. A view of a
  session after its host ended, without a run, is later work.
- A change record whose retention has expired makes its session's
  declaration unavailable.
- Independent review remains open.

## Consequences

- `kernel/engine`: migration `0025-run-effect-records.sql`, the
  `run_effect_record_store` module with its tests, `SCHEMA_VERSION` 25 with
  its history, verification at open, `DurableAuthorityRuntime::run_effect_records`,
  the schema 25 fixture and migration tests. The Sprint 11 store producers
  and their documents name schema 25.
- Batch 27's review fixes: `runtime_loop_research_report_tests.rs`,
  Decision 0142 and `docs/architecture/canonical-research-source.md`.
- `shells/host`:
  - the `coding_session_recoverability` module with its tests;
  - the persisted recorder and session verification in
    `coding_recoverability`;
  - the boundary's persistence and session declaration;
  - the factory's restart flag, chain and handover;
  - the service's declaration and closing;
  - run declarations schema 5 and wire 16;
  - the CLI's verification and output;
  - the producer contract tests.
- `fixtures/runtime-producer/v2`, the contract version 2 document,
  `scripts/runtime_producer_fixtures.py` and
  `tests/test_runtime_producer_contract.py`.
- TASKS.md gains AMR-04.7.3, and AMR-04.7.2 names it. AMR-04.7.3 is recorded
  with its evidence when the batch is verified.

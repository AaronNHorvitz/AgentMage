# Decision 0129: Durable Run Action Histories in the Operational Store

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088, 0118, 0124, 0125, 0127 and 0128; current owner restart |
| Scope | AMR-05.9.6 |

## Findings

Decisions 0127 and 0128 keep three action histories for each coding run:

- the effects chain, kept by the coding tool boundary;
- the job control chain, kept by the live runtime service;
- the route chain, kept by the development runtime factory.

They live in memory for one host process. The run's declarations show and export
them after the run, and they are lost when the host ends. AMR-05.9.6 requires
three things:

- the histories survive a host restart and replay against their stored heads;
- an ended run's histories can be viewed and exported through the host and CLI;
- restart, tampering, retention and the export of an ended run are tested.

The operational store already persists job control ledgers (Decision 0118). It
uses an immutable root, append-only entries and a forward-only head, with each
append committed together with its head in one immediate transaction that names
the head it read. The same pattern fits an action history chain.

## Decision

### Store

Operational store migration 22 adds three tables for run action histories,
keyed by run and chain (`effects`, `job-control` or `routes`):

- A root per chain, which is immutable and names its owner.
- A record per position, with its sequence, entry digest, an expired flag and the
  canonical encoding of the kernel record (Decision 0124).
- A head per chain, with its entry count, head digest, a complete flag and a
  closed flag.

Triggers enforce the following rules:

- Roots and heads cannot be deleted, and records cannot be deleted.
- A record may change only from kept to expired, keeping its run, chain,
  sequence and digest.
- A head only moves forward. An append raises the count by one. The complete
  flag can only be cleared, and the closed flag can only be set. Once a head is
  closed, its count and its complete flag no longer change.

The migration first verifies the prior schema history, and it stays retryable at
version 21.

`DurableRunActionHistories` shares the store's lock, as the job ledgers do. Its
operations are:

- `create` a chain for a run under its owner;
- `attach` to an open chain, optionally marking it incomplete;
- `append` one redacted entry;
- `mark_incomplete`;
- `close`;
- read a chain;
- apply retention to a closed chain.

Every append replays the stored chain, appends through the kernel history and
commits the record and the advanced head in one immediate transaction. That
transaction's head update names the head it read. Every read and every store
open replays every chain. The chain must reproduce exactly in canonical form and
end at its stored head, and the head's count and digest must match. Anything else
fails as an integrity error that poisons the store. A chain holds at most 512
positions. Only the chain's recorded owner may append to it, mark it incomplete
or close it. The store's key authenticates writers, as for the job ledgers.

Retention applies only to a closed chain. It rewrites the records whose
retention deadline passed into their expired places. The head does not change,
and the chain must still replay. No append can follow an expired entry, so the
time lost with an expired place (Decision 0125) cannot matter.

### Owners

All three chains of a run are owned by the coding host, under the identity it
already uses for job ledgers. Each component appends only to its own chain:

- When it composes a new run, the development factory creates the effects and
  route chains and attaches the tool boundary's recorder to the effects chain.
  It hands the store handle to the live service, as it does the job ledgers, and
  the service creates the job control chain when it starts the run. If a chain
  cannot be created, composition fails, as it does for a job ledger.
- The boundary and the service record each entry in memory and in the store, in
  that order. A failed store append makes both the memory history and the stored
  chain incomplete, so neither claims an entry it missed.
- The factory appends the route entry before it builds any model, so no route can
  be used without its entry.
- A run resumed after a host restart attaches to its open chains. Its effects and
  job control chains are marked incomplete, because the host that ended may have
  missed an entry between an effect and its record. Its route chain stays
  complete.
- A run continued in the same host (Decision 0122) attaches without marking.
  Before the continuation opens the store again, the held run releases every
  store handle, including the job control recorder's.
- The service closes all three chains when the client releases the ended run,
  or when it drops an ended run it still held. It does not close them at the
  terminal outcome, because the ledger still decides, and the service still
  records, control requests that arrive after the outcome (refused as
  terminal). A run whose host ended before its release stays open, and the view
  says so.

### Ended runs

A new IPC operation, wire version 11, asks the host for the stored histories of
one run. The host serves it only while it holds no run, because the store admits
one connection. The factory opens the store and applies retention to each
closed chain. It then answers with each chain's records, head and its complete
and closed flags, and closes the store again.

The development CLI's `--ended-run RUN_ID` replaces the objective. It launches the
host and asks for that run's histories. It replays each chain and checks its
owner's kinds. It shows each chain with whether it is complete and whether the
run ended, and exports a requested range with `--action-history-export`. A chain
that is open, incomplete or missing is said to be so. The CLI never treats such a
chain as a complete record.

## Limits

The ended-run view needs a host that can start. In the builder's sandbox the
development host is refused at native Git trust before it serves IPC, so no actual
process can store or read a history there. The composition glue is exercised only
where native composition runs. The store, the recorders, the service, the wire and
the CLI are covered by unit and service tests. Native proof remains AMR-05.10, and
independent review remains open.

## Consequences

- `kernel/engine`: migration `0022-run-action-histories.sql`, the
  `run_action_history_store` module with tests, `SCHEMA_VERSION` 22 with its
  history, verification at open, `DurableAuthorityRuntime::run_action_histories`,
  the schema 22 fixture, and migration tests.
- The Sprint 11 store producers and their documents name schema 22.
- `shells/host`: the persisted recorder, the factory's handover and route append,
  the service's job control appends and closing, the ended-run read through the
  transport and IPC, and the CLI's `--ended-run` mode.
- TASKS.md: AMR-05.9.6 is recorded with its evidence when the batch is verified.

# Decision 0118: Durable Job Control Ledgers

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0081, 0088, 0110 and 0113; current owner restart |
| Scope | AMR-04.6.1: persist the CAP-35 job control ledger through the operational store owner; splits AMR-04.6 into AMR-04.6.1 to AMR-04.6.3 |

## Findings

Decision 0110 built a single-owner job control ledger that lives only in memory.
Decision 0113 (findings V1 and V4) found that its hash chain proves only that its
entries agree with each other. It does not prove who wrote them or that none are
missing after the head. The AMR-04.6 row therefore requires the store that keeps the
ledger to do three things:

- keep the head and entry count in the same transaction as each append;
- authenticate its writers;
- scope request identities by the authenticated client.

AMR-04.6 also asks for host control over authenticated IPC and for an actual-process
proof across UI closure. Those parts need a host service change, a wire change and a
native host, so they are separate rows.

## Decision

1. Rows. AMR-04.6 is split into three rows:
   - AMR-04.6.1: the durable store (this decision).
   - AMR-04.6.2: host control over authenticated IPC.
   - AMR-04.6.3: the native actual-process proof.

   AMR-04.6 closes only when all three have closed.
2. Schema. Operational store migration 21 adds three tables:
   - `job_ledger_roots`: one row per job with its owner and the digest of its first
     entry. Rows cannot be updated or deleted.
   - `job_ledger_entries`: the entries of each job in order, with their digests and
     canonical encoding. Rows cannot be updated or deleted.
   - `job_ledger_heads`: the entry count and last digest of each job. The head must
     name an existing entry, can only move forward and cannot be deleted.

   A `REPLACE` removes a row without firing delete triggers unless recursive
   triggers are on, so the store turns them on and verifies it at every open
   ([Decision 0119](0119-review-fixes-for-durable-job-ledgers.md), review F2).

   The migration first verifies the complete prior history, then runs in one
   transaction like every earlier one. A corrupt history or a failure leaves the
   store at version 20, and a failed migration stays retryable.
3. Appends. Every append inserts its entries and moves the head in one immediate
   transaction. The head update names the head the owner read, so a writer holding an
   older head changes nothing. A failure in any statement writes nothing. A retry
   that the ledger answers from its record writes nothing.
4. Reads. Every read replays the job's retained entries. The replay must reproduce
   each entry exactly, find each stored in its canonical encoding, match the root's
   owner and first digest, and end at the retained head. Opening the store replays
   every ledger. A ledger that fails replay makes the store fail its integrity check;
   through an open handle it poisons the store for further use.
5. Writers. The operational store authenticates writers by its key. It opens only
   with its key, holds an exclusive writer lock, and refuses pages whose
   authentication fails, so no process without the store key can append, remove or
   reorder entries. A key holder is trusted as the owner, and the ledger cannot
   attribute an entry beyond the key
   ([Decision 0119](0119-review-fixes-for-durable-job-ledgers.md), review F1).
   Within the process, only the job's recorded owner identity can record an owner
   observation.
6. Client scope. Each client request is recorded under a client scope that the owner
   supplies from the client it authenticated; a client never names its own scope. A
   retry returns the original decision only from the same client. The same request
   identity from another client is that client's own request. Ledger entries become
   format 2, which records the scope; no format 1 entry was ever persisted.
7. Access. `DurableAuthorityRuntime::job_ledgers` returns a handle that shares the
   store's lock. A control request can therefore be recorded while the runtime is
   busy with an effect. The handle grants no authority and performs no effect, and
   a poisoned store refuses it.
8. Export. The content-free derived export lists the three families with hashed
   identities and digests only. It never includes request identities, client scopes
   or decisions.

Each job keeps at most 4,096 entries, the ledger's existing bound. Ledgers are not
yet subject to retention and are kept until a later retention decision covers them.
Every operation replays its job's entries, so its cost grows with the job's history,
within that bound.

## Verification boundary

Engine tests use real encrypted stores in private temporary directories. They cover:

- persistence and replay after reopening;
- retries and conflicts, per client scope;
- owner-only observations;
- rejected head and entry commits that leave nothing behind;
- append-only triggers;
- seven tampering cases that fail replay and block reopening;
- the derived export;
- the migration from version 20, refusal over a corrupt history, and a failed
  migration that stays retryable.

A mutation of each of five rules made its test fail: the shared head transaction,
the canonical encoding check, replay on open, poisoning and client scoping.
[Decision 0119](0119-review-fixes-for-durable-job-ledgers.md) (review F3) adds tests
for the two rules that no test covered: an append that names another head, and a
root whose first digest differs from the chain.

No host service, IPC, CLI or native process uses the store yet; that is AMR-04.6.2 and
AMR-04.6.3. Independent review of this batch is requested and remains open.

# Decision 0119: Review Fixes for Durable Job Ledgers

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088, 0115, 0116, 0117 and 0118; current owner restart |
| Scope | Findings F1 through F5 of the independent review of `6355a379`; amends Decision 0118 |

## Findings

An independent read-only review of `8fbd2bc6..6355a379` passed with five low
findings.

- F1: Decision 0118 and the store's module comment said the store authenticates
  writers "so no other process" can change a ledger. The store authenticates writers
  by its key. Any holder of the key, such as the tests' own tampering helper, can
  write a chain that replays exactly. The ledger therefore attributes an entry to a
  key holder, not to the job's owner.
- F2: SQLite does not fire delete triggers for rows that `REPLACE` removes unless
  recursive triggers are on, and the store left them off. An `INSERT OR REPLACE`
  could therefore change a root, entry or head without any trigger firing. Replay
  still refused the changed row.
- F3: removing two of the rules Decision 0118 names left every test passing. One is
  the check that the head update changed exactly one row. The other is the check
  that the root's first digest names the chain's first entry.
- F4: Decision 0118 changed AMR-04.5's retry rule, but the AMR-04.5 row did not say
  so. Decisions 0115 and 0116 did not point to the parts Decision 0117 narrowed.
- F5: the retained intermittent inference lease test has a demonstrated cause.
  Sibling tests in the same process spawn processes. A forked child holds a duplicate
  of the lease descriptor until it runs its program, and a `flock` stays held until
  every duplicate closes. A release followed at once by an acquire can therefore see
  `busy`.

## Decision

F1. The wording now matches what the store proves. The store authenticates writers
by its key: no process without the key can append, remove or reorder entries. A key
holder is trusted as the owner, and the ledger cannot attribute an entry beyond the
key. Decision 0118 item 5, the store's module comment and the AMR-04.6 and
AMR-04.6.1 rows say so. Attributing entries to the owner would need a secret that
the store key does not reveal, which would be a separate decision.

F2. Every operational store connection turns recursive triggers on, and the runtime
configuration check refuses a connection where they are off. A `REPLACE` that would
remove a ledger root, entry or head now fires the delete trigger and fails. The same
holds for the research budget and workflow attempt tables, which use the same
append-only triggers. No store statement uses `REPLACE`. Only one existing trigger
writes to another table, and that table has no trigger, so no trigger recurses.

F3. Two tests cover the uncovered rules. The first appends against a head whose
digest differs from the retained one. It expects an integrity refusal, no new entry
and an unchanged head. The second is an eighth tampering case: it changes the root's
first digest and expects reopening and an open handle to refuse the store.

F4. The AMR-04.5 row now names Decision 0118: a retry returns the original decision
only from the same authenticated client, and entries are format 2. Decision 0115
(V2) and Decision 0116 (items 1 and 4) point to Decision 0117.

F5. The tests that release the lease and then acquire it, in the same process or in
a child it spawns, retry only `busy`, for at most two seconds. Any other result
returns at once, so a real refusal still fails the test. This is a test-only change.

## Verification boundary

Engine tests use real encrypted stores. They cover the `REPLACE` refusals on all
three ledger tables, the configuration check for recursive triggers, and both F3
rules. Four single mutations each made exactly the expected new or extended test
fail:

- turning recursive triggers off together with their check;
- dropping only the check;
- removing the head check (the review's S7);
- removing the root digest check (the review's S9).

Before this decision, S7 and S9 left the engine suite passing. The F5 change is
test-only and was not mutated. Independent re-review of this batch is requested and
remains open.

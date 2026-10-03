# Decision 0145: Review Fixes for Session Effect Records and Producer Privacy

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-02 |
| Date | 2026-10-02 |
| Authority | Decisions 0054, 0081, 0129, 0143 and 0144; current owner restart |
| Scope | Findings F1 to F5 and notes N2, N4, N5 and N7 of the independent review of `6c0f51fe` |

## Findings

An independent read-only review of `8842c770..6c0f51fe` (batches 28 and 29)
passed with two medium and three low findings and ten notes.

- F1 (medium): a run's recorder remembered a missed entry only in its own
  memory. If the store refused both the entry and the incomplete mark that
  follows it, the service still closed the chain, which then claimed to be
  complete. A later run of the same session in the same host has its own
  recorder, so it declared the session from that chain without the missed
  effect. Decision 0143's Limits said the opposite.
- F2 (medium): the batch 29 evidence pass committed a log with the private
  checkout path. The Sprint 11 content deduplication producer appends Cargo's
  output unredacted, and Cargo names the checkout whenever it compiles.
- F3 (low): no test showed that the live service closes a run's effect
  record. A change that stopped closing it left every host test passing.
- F4 (low): the local testing guide still named contract version 2 as
  current.
- F5 (low): two paths left a new effect record open with no owner: the
  service failing to spawn a composed run, and the factory failing after it
  created the chain of a run resumed after a restart.

The notes:

- N1, N3, N6, N8, N9 and N10 need no change.
- N2: a grant catalog the factory cannot read fails the composition. That
  deserved a sentence in Decision 0144.
- N4: a resumed chain was marked incomplete before its session and task were
  checked.
- N5: the observation script was not retained with its results.
- N7: no test stored a catalog that holds one grant twice.

The same survey found the two sibling producers of F2 (Sprint 11 source
lifecycle transactions and workflow attempt invariants) capture Cargo's
output the same way. Their committed logs do not hold the path today.

## Decision

### F1: the miss is shared, retried and closes nothing

`PersistedRunEffects` now holds, behind a shared handle, what the host knows
about the run's chain: whether an entry missed the store, and whether the
store holds the incomplete mark. Every clone of the handle shares this. That
includes the recorder inside the tool boundary and the handle the factory
gives the service.

- A missed entry, or an entry the memory record could not keep, sets the
  miss and asks the store for the mark.
- A mark the store refuses stays pending. Each later append retries it
  before its own entry.
- The service closes a chain only after the store holds a pending mark. If
  the store still refuses, the chain stays open. Every later declaration of
  the session is then refused, because an earlier run was never released.
- An in-host continuation reopens the store, so the earlier handle is
  dropped. The service keeps the shared knowledge, without the store handle,
  across the reopening, and the run's next handle continues from it: a miss
  carries over and a pending mark is retried.

The factory now hands the service the run's `PersistedRunEffects`, or
nothing when the run has no chain, in place of the raw store handle.

Decision 0143's Limits sentence is corrected in place, with a pointer to
this decision.

The review proposed a test that fails the store's insert and head update
with temporary triggers. The host crate has no raw access to the encrypted
store, and adding one would add a dependency to the host. The store's own
atomicity under such triggers is already tested in the engine. The host
tests instead refuse the chain's store writes at the one seam the handle
writes through, as a failing disk would. Production code always writes
through the operational store. The tests show:

- run 1 misses an entry and its mark, and its release leaves the chain
  open, so run 2 of the same session in the same host is refused;
- once the store writes again, the release stores the mark and closes, and
  the session stays refused;
- the next append stores a pending mark before its entry;
- a miss carries across an in-host continuation, whether its mark was
  stored or not, at the handle and through the live service, and a run
  that missed nothing carries nothing.

### F2: producers redact the checkout

The three Sprint 11 store producers replace the checkout root with
`<repository-root>` in the output they retain, using the shared helper the
Story 13.4 producer already uses. Each also refuses to write, or to accept,
retained output that still names the checkout or a home directory. A test
covers both rules for all three. The content deduplication log is
regenerated in this batch's evidence pass, with every report that binds it.

The tracked files that already held the path before this batch remain an
open owner question, kept privately. This batch does not rewrite them.

### F3 and F5: the service closes what it holds or could not start

- A test now shows the live service closing the run's effect record when
  the client releases the ended run, and when the service is dropped while
  it holds an ended run.
- A new run whose session cannot spawn, or whose job cannot begin, ran
  nothing. The service closes its effect record and action history chains
  empty, and a test shows it for the job case.
- A run resumed after a restart keeps its chains open when its composition
  or start fails. The store refuses to attach a closed chain, so closing it
  would keep a later host from resuming the run. The chain is already marked
  incomplete, so its session is never declared again either way. The
  factory's comment and this decision state the reason.
- A run whose start fails after its worker began keeps its chains open
  unless it ended, because the worker might still record an effect.

### F4 and the notes

- F4: the local testing guide names version 3 and its fixtures, and says
  that versions 2 and 1 stay unchanged.
- N2: Decision 0144's Limits now say that an unreadable catalog fails the
  composition and blocks every run of its state root until it is repaired.
- N4: a resumed chain is attached first. Its session and task are checked,
  and only then is it marked incomplete. A test shows that a chain of
  another session is refused and left complete.
- N5: this batch's verification record retains the observation script and
  the mutation runner with their results.
- N7: a test stores a catalog, in its own encoding, that holds the same
  grant twice in one scope. The factory's read refuses it, as it refuses
  two live grants of one route. The owner's listing, which reads every
  grant, refuses it only through the catalog's identity rule, and the test
  shows that too.

## Limits

- No actual process stores, closes or declares a session's effect records
  in this sandbox. The development host is refused at native Git trust, as
  before. The fixes are shown by host tests over a real encrypted store.
- The host tests refuse writes at the chain's write seam, not inside SQLite.
- Independent review remains open.

## Consequences

- `shells/host`: `coding_session_recoverability` (shared chain knowledge,
  the write seam, pending marks, closing and continuation, the check before
  the mark) with its tests; the factory's handover and closing; the
  service's closing, its start failure paths and its continuation; the
  `NativeChatRuntimeFactory` handover type; the service tests; the catalog
  decode test.
- `scripts`: the three Sprint 11 store producers, with their test.
- `docs`: Decisions 0143 and 0144 and the local testing guide.
- No TASKS.md row changes state. AMR-04.7.3 and AMR-05.9.7.1 keep their
  evidence. This batch's verification record lists the fixes.

# Decision 0136: Review Fixes for Exact Decoding, and the AMR-03.2 Split

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0054, 0081, 0084, 0088, 0097, 0106, 0109 and 0135; current owner restart |
| Scope | Findings F1 to F4 and notes N1, N2, N4 and N5 of the independent review of `4bef629b`; AMR-03.2 split; AMR-03.2.1 |

## Findings

An independent read-only review of `84e531fb..4bef629b` passed with one
medium and three low findings and nine notes.

- F1 (medium): exact decoding compared the frame, read as JSON, with the
  decoded value converted directly to JSON. The runtime writes a 32-bit
  number as its shortest decimal, `0.95` for example. Read back, that text is
  a different 64-bit number than the 32-bit value widened. A run request
  whose decoding profile held such a value was therefore refused in both
  directions, though the runtime itself had written it. The three
  development models use `0`, `1` and `1`, so no run was refused. A catalog
  profile with `top_p` `0.95` would have been. Decision 0135 and the batch 20
  record claimed that no conforming frame changed, and the contract obliged a
  consumer to decode exactly. Both were therefore wrong for such a request.
- F2 (low): a member repeated inside a map-typed member passed exact
  decoding. The types and a JSON value both keep the last of two equal names,
  so they agreed. Struct members were already refused. Nothing was granted,
  because the host validates and seals the values it kept. A consumer that
  kept the first value would read a different request.
- F3 (low): two sentences of the contract document were untrue. Obligation 6
  said records carry no content, but the run request carries the task's
  objective, acceptance criteria and constraints, and the run recipe carries
  parameter values. The transport section named two omitted members; a step's
  `suspended` is a third, and it did not say that an explicit `null` for an
  omitted member is refused (note N1).
- F4 (low): `JobControlDecision` was not closed in the types. Off the wire,
  where only the types decode a decision, a member beside its own was dropped
  silently.
- N2: Decision 0135 called the fixture request "the deterministic native
  read fixture"; it is the controlled-write coding-run fixture.
- N4: the mutation specification and results the request named were in this
  lane's private state, which the reviewer could not read.
- N5: other closed enums with tag-only variants exist outside the runtime
  transport.

Notes N3 and N6 to N9 need no change. N3 is the derived disposition record;
the review request names it. N6 is cost, not exposure. N7 is the missing
fuzz target. N8 covers tests that need a Git checkout. N9 is the retained
ledger stage.

The AMR-03 assessment of 2026-09-29 found AMR-03.2 not started and gated.
Every runtime mode refuses network operations. Decision 0084 requires the
native adversarial matrix before host or provider activation, and AMR-03.2
also needs a configured provider and an admitted model. One part of the row
does not depend on any of these. Decision 0109 recorded residual R10: before
task-authorized issuance, a visit's query fields must be refused or
disclosed. A visit is a request to an allowed domain other than the search
provider's.

## Decision

### Review fixes

F1. Exact decoding reads the frame as JSON. It decodes the frame into its
types, re-encodes the result to bytes, reads those bytes as JSON and compares
the two. Numbers on both sides go through the same writing and reading, so
every value the runtime writes decodes. A number written another way is still
refused, such as `0.950000001` for the 32-bit `0.95`, or `1` for `1.0`. The
wire test sends a start request and a prepared answer whose decoding values
are `0.7`, `0.95` and `1.1`. A producer test checks that such a request
decodes exactly and verifies. A transport test covers a sweep of 32-bit values.
The wire version stays 15. Decision 0135 and the batch 20 record are
corrected in place.

F2. The frame is read as JSON by a reader that refuses an object naming a
member twice, at any depth. That reader is the left side of the comparison.
The wire test and the producer tests repeat the first member at every object
position. Only the types' maps admitted a repetition: the recipe's parameter
values in the request and the declared plan's. Exact decoding refuses both.

F3 and N1. The contract document names the task and the recipe's parameter
values as the person's content. It names the three omitted members. It says
that an explicit `null` for one of them, a repeated member and a number
written another way than the runtime writes it are all refused. The wire test
checks the explicit `null` of `recipe` and of `suspended`.

F4. `JobControlDecision` refuses unknown members in the types. In the wire
test, the positions only exact decoding refuses shrink to the two tag-only
positions: the `shutdown` operation and the `released` result.

N2. Decision 0135's Limits name the controlled-write coding-run fixture.

N4. The batch's mutation specification and results are retained with its
record, beside the stage logs.

N5. No change in this batch. Runtime events are read from the operational
store only when their stored bytes equal their canonical encoding. They cross
the runtime transport under exact decoding. The engineering protocol's
requests and events and the headless client's commands are outside the
runtime transport. They stay a residual for the rows that next change those
paths.

### AMR-03.2 split

Split AMR-03.2 into four rows. The parent closes only when every row closes.

| Row | Scope | Prerequisites |
| --- | --- | --- |
| AMR-03.2.1 | Refuse, before anything is spent, a visit that carries any query field under a task grant (residual R10). In ask mode the exact request, its fields included, is approved before it is sent, so it is disclosed. | AMR-02.2; Decision 0106 |
| AMR-03.2.2 | In the engine only, an explicit research admission for one run. The coordinator admits a network operation only for a run whose sealed request names that admission, bound to the persisted plan and budget, and only through the existing consumed grant, durable reservation and dual-proof dispatch. The three ordinary modes keep refusing network operations. No host, worker or provider composition. Its design needs its own decision first. | AMR-03.2.1; Decisions 0084 and 0097 |
| AMR-03.2.3 | The deterministic adversarial matrix through the coordinator with the admitted native worker on a native Linux host | AMR-02.3.2; AMR-03.2.2 |
| AMR-03.2.4 | The separately pinned real-provider and local-model research-to-patch campaign | AMR-03.2.3; a configured provider; an admitted model |

AMR-03.1.2 gains a component row, AMR-03.1.2.1. Through the real
coordinator under the AMR-03.2.2 admission, and with synthetic native
results, a public GET completion is prepared, committed, published and read
back through the canonical reader. It covers forged and stale receipts,
partial bundles, source drift and reopening without replay. It depends on
AMR-03.2.2, because Decision 0097 forbids a read or write operation standing
in for the positive case. AMR-03.1.2 still needs the native boundary.

### AMR-03.2.1

When the scope's mode is task-authorized, the owner's classification of a
prepared request refuses a visit with one or more query fields as a
destination error. The durable reservation therefore spends nothing and
records only a clock observation, as for every refused shape. A visit
without query fields and the plan's disclosed search are unchanged. Ask
mode is unchanged. Schema 2 scopes and plans, their bytes and their policy
digests are unchanged.

## Limits

- AMR-03.2.1 is a component row. No runtime mode admits a network operation
  yet. No task grant is issued, and no worker, provider or model runs.
- Exact decoding makes no record authoritative. Each consumer still verifies
  what it uses.
- A visit's path is still not a disclosed query. The secret detector, the
  exact destination list and per-request approval in ask mode still apply to
  it.

## Consequences

- `shells/host`: exact decoding with the unique-member reader, its tests,
  the wire test and the producer contract tests.
- `kernel/engine`: the closed job control decision; the task-authorized
  visit refusal and its budget and journal tests.
- `docs/architecture/runtime-producer-contract-v1.md`: the corrected
  transport section and obligation 6. No fixture, record, encoding or
  version changes.
- Decision 0135 and the batch 20 record: the corrected sentences.
- TASKS.md: rows AMR-03.2.1 to AMR-03.2.4 and AMR-03.1.2.1. AMR-03.2.1 is
  recorded with its evidence when the batch is verified. The work selector
  records the five identities.

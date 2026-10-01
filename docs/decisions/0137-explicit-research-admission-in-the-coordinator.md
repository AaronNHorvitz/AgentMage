# Decision 0137: An Explicit Research Admission in the Coordinator

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0054, 0081, 0084, 0096, 0097, 0106 and 0136; current owner restart |
| Scope | AMR-03.2.2, the coordinator's side of an admitted research run; engine only |

## Findings

Every runtime mode refuses a network operation, and only one place enforces
this. When the coordinator is composed, every registered tool must pass
`operation_allowed_for_mode`, so a catalog that holds a network tool is
refused before the run starts. The state-change rule refuses a network
result again, if one were ever returned.

The research owners already exist, and only tests call them:

- the canonical budget owner opens a task budget for a published plan and
  reserves each request durably before dispatch;
- the authority owner consumes the exact grant and begins the effect with
  a start observer;
- dispatch requires two proofs, the consumed grant and a fresh durable
  reservation;
- completion normalization prepares the six-member public GET bundle
  through the coordinator's borrowed builder.

The budget owner opens a budget only for a plan that the run itself
published: a run-level JSON report with its creation event in the run. No
one publishes such a plan today.

Decision 0084 forbids host or provider activation before the native
adversarial matrix. Decision 0097 keeps every ordinary mode refusing
network operations, and forbids a read or write operation standing in for a
positive network case.

## Decision

Add an explicit research admission to the coordinator, in the engine only.

### The admission

`RuntimeResearchAdmission` is built from three inputs: the plan's canonical
bytes, the budget context (session, task, run and governing policy), and
the one public GET tool definition. Construction refuses each of these:

- a plan that does not decode exactly;
- a plan whose task differs from the context;
- an offline plan;
- a schema 1 plan, which cannot reserve;
- a tool that does not declare exactly one network effect under a
  single-use network grant.

Its digest covers a domain label, its own version, the plan digest, the
session, run and policy, and the tool definition's digest. The plan digest
covers the task, the scope and the network mode. The definition digest covers
the tool's identity and version. The admission holds no grant and makes no
I/O.

### Naming the admission in the sealed request

The sealed run request names the admission with exactly one task constraint:
`Research admission digest: <64 lowercase hex>`. This follows the
recipe-plan line. The request seal covers it, and the coordinator compares
it with the admission it was given.

### Composition

A new constructor composes a fully durable coordinator with an admission and
a research budget port. It refuses the run unless all of these hold:

- the mode is durable read-only;
- the run has no resume cursor;
- the request names this admission exactly once;
- the admission's context is the request's session, task, run and policy;
- the catalog holds the admission's tool exactly once, and every other tool
  passes the ordinary durable read-only rule.

Every other constructor refuses a request that carries an admission line.
An admission therefore never appears implicitly, and the three ordinary
modes keep refusing network operations unchanged.

### Starting the run

After the run-start event and the request artifact, the coordinator, which
owns publication, publishes the plan bytes as a run-level JSON report. It
flushes the journal and asks the budget port to open the run's budget for
that exact artifact. It continues only when the owner answers with that
artifact, the admission's scope and an unspent first revision. The port is
implemented by the trusted host over the existing budget owner. Its answer
is a description to compare, not a grant.

### Executing the admitted tool

Only the admitted tool may return a network result, and only these:

- success that changed state;
- uncertain that is uncertain;
- failure, denial, cancellation or timeout that did not change state.

A success must carry a report output and exactly six report artifacts. Those
must have been prepared through the borrowed builder under the call's
receipt; this is the public GET completion of Decision 0097. Any other
network result is refused, as now.

### What stays with the trusted port

The coordinator still delegates evaluation, approval and execution to the
trusted correctness transaction port. That port must reserve through the
budget owner, consume the grant, begin the effect with the start observer,
dispatch with both proofs and prepare the completion. Each of those owners is
already tested on its own. AMR-03.1.2.1 proves the composition through the
real owners and the real coordinator.

## Limits

- Engine only. No host, worker, provider, model or grant issuance composes
  or uses the admission. The coding host and its catalog are unchanged.
- A resumed admitted run is refused. Resuming one is later work.
- The coordinator cannot see the canonical store. Apart from the budget it
  checks at start, it relies on the trusted port for reservation, grant
  consumption and dispatch, as it does for every effect.
- The producer contract is unchanged, because no host produces an admitted
  request.

## Consequences

- `kernel/engine/src/runtime_loop.rs` changes:
  - the admission type and the research budget port;
  - the constructor and the composition checks;
  - the plan publication and the budget check at start;
  - the admitted rule in execution validation and in the borrowed builder.
- A new coordinator test module covers each of the following:
  - the admitted positive case and each composition refusal;
  - plan publication and the budget check;
  - each admitted and refused result;
  - the unchanged refusals of the ordinary modes.
- `docs/architecture/reusable-runtime-coordinator.md` describes the
  admission.
- TASKS.md: AMR-03.2.2 is recorded with its evidence when the batch is
  verified, as a component row.

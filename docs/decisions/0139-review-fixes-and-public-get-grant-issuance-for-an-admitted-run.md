# Decision 0139: Review Fixes, and Public GET Grant Issuance for an Admitted Run

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0084, 0097, 0106, 0136, 0137 and 0138; current owner restart |
| Scope | Findings F1 and F2 and note N2 of the independent review of `b59afdb9`; the AMR-02.2 remainder |

## Findings

An independent read-only review of `7611c8bf..b59afdb9` (batch 23) passed
with two low findings and thirteen notes.

- F1: the owner's terminal check binds the call, the receipt, the turn and
  the operation, and the journal binds the sequence and the predecessor.
  Neither binds the terminal's cause to the start. A terminal that names an
  earlier event of the run as its cause, such as the run start or the
  permission decision, is still committed. Nothing is granted or completed
  differently, but Decision 0138's claim that the terminal names the pending
  effect is not complete.
- F2: the two Linux callers of the changed owner path are compiled only
  with the `public-research-worker` feature. The batch's source stage ran the
  Linux suite and clippy without that feature, so those callers were not even
  type-checked. Decision 0138 said they comply, but that was a reading
  claim.
- N2: Decision 0138 listed "grant issuance" among the engine's owners. The
  issuer is the engine's, but the decision to issue under a task-authorized
  plan was the test glue's.

The other notes need no change. N1 explains why poisoning the owner after a
refused terminal is right. N3 and N4 concern the test's fixture model and
in-memory payloads. N5 and N6 confirm the stated behavior. N7 is the derived
disposition record. N8 covers the mutation specifications. N9, N11 and N13
confirm the record's claims. N10 is the missing fuzz target. N12 is the
ledger stage.

AMR-02.2 needs one more piece: issuing a grant for an admitted run. The
budget owner, the research restrictions, the modes, destination and secret
refusal, checked budget consumption and cancellation already exist. Today
only test glue issues a network grant, and it decides on its own whether the
plan's mode admits the request. The engine has no owner that issues a grant
for an admitted run under the plan's mode.

Designing that owner found one gap in the start of a research effect. The
authority transaction evaluates the policy against a network scope that the
caller supplies, and nothing ties that scope to the packet's destination. The
plan's domains still restrict the packet, but a caller could make the policy
evaluate a destination other than the one the worker would contact.

## Decision

### Review fixes

F1. The owner commits a pending effect's terminal only when the terminal
also names the start as its cause. The changed-terminal test adds a terminal
whose cause is the event before the start. That event is a real earlier event
of the run, so only the owner's new clause can refuse it.

F2. The batch's source stage type-checks the Linux crate's tests with the
`public-research-worker` feature, and it runs the research tests that need no
native fixture. The record says that the two native dispatch tests, which
commit terminals through the owner, were compiled but not run. They need the
native DNS, HTTP and model fixtures of AMR-03.1.2.

N2. Decision 0138 is corrected in place: the grant issuer is the engine's,
and the task-authorized issuance decision there is the glue's.

### The owner issues one exact public GET grant

`DurableAuthorityRuntime::issue_public_get_grant` issues one exact
single-use network grant for an admitted run. It takes the run's budget
context, the exact call, the session parent, the confirmation, the
kernel-selected identities, one target, and two instants: when the request's
packet was prepared and when the grant is issued. Before committing anything,
it checks the following in order, and it refuses the first one that fails.

1. The owner is usable.
2. The registry has the call's tool. The call's arguments decode to one
   public GET request.
3. The budget owner would reserve this exact request at the issuance
   instant, without spending:
   - the budget exists for the run's context, and the plan is not offline;
   - the packet is prepared from the retained plan's restrictions and
     original clock, and the issuance instant lies inside the packet's
     interval;
   - the target classifies as a query the plan disclosed or as a visit.
     Under a task grant, a visit with query fields is refused (R10);
   - a dry-run reservation on the loaded copy passes. The budget must not be
     cancelled, the operation must be new, the clock must not be rolled back
     or expired, the query must be one the plan disclosed, and the counts and
     bytes must fit. The journal's capacity is checked when the start
     reserves;
   - the full plan is still live in a run that has not ended.
4. The call matches its registered tool and the packet exactly. This uses the
   shared call validator that the effect binding and completion already use.
5. The plan's mode matches the confirmation:
   - a task-authorized plan takes the plan confirmation and nothing more;
   - an ask plan takes the person's approval of the exact request. The
     approved preview must be the packet's digest.
6. The policy is the run's policy.
7. The parent belongs to the run's session and task. Its preview is the
   digest of the plan's full canonical bytes. This means the person confirmed
   this exact plan when the parent was issued.
8. The call's action holds no grant yet, in any state. A call receives at most
   one grant.
9. The existing issuer derives the child grant. The parent must be a session
   read that is issued, unexpired, not exhausted and under the same policy,
   and the target must lie inside its scope. The child is a single-use network
   grant for the call's tool, action and arguments. Its one expected effect
   names the packet's digest, which is also its preview. It expires at the
   packet's deadline or the parent's expiry, whichever comes first.
10. The governing policy evaluates the exact child grant in the run's session
    and task, against the packet's own destination, `https:<domain>:443`. The
    authority transaction repeats this evaluation at the start.

The owner then digests a closed issuance decision: the mode, the plan, the
parent revision, the grant revision, the request, the approved preview and
the policy result. The caller builds the permission event from the issued
grant and that digest. The owner commits the parent's new revision, the child
grant and the event atomically.

A refusal commits nothing and spends nothing. Its error is a new closed type,
so the existing `DurableAuthorityError` and its exhaustive users in the host
do not change.

`prepare_public_get` runs the same checks 1 to 4 and returns the packet
that an ask plan presents for approval. It records nothing.

### Ask plans through the coordinator

For an ask plan, the trusted port prepares the packet when it evaluates the
call. It answers `Ask` with the packet's digest as the preview. The
coordinator's challenge already presents the exact argument bytes. When the
person approves, the port asks the owner to issue the grant with that
approval and the packet's original preparation instant. The owner prepares
the packet again from the retained plan and accepts only the approved digest.
When the person declines, nothing is issued.

### The start names the packet's destination

`matches_research_start` also requires that the transaction's policy context
names the packet's own destination as its network scope. Every existing
caller already passes that scope.

### Tests

The engine owner is tested through the real coordinator and the real owners
in the AMR-03.1.2.1 module. Its test glue now issues each grant through the
owner instead of deriving it itself.

- Positive cases. A task-authorized plan completes as before, and its grant
  comes from the plan-bound parent, names the packet and expires with it. A
  grant expires with a parent that expires before the packet. An ask plan
  pauses at the exact challenge, which presents the exact request, with
  nothing issued or spent. The person approves, and the run completes under
  the grant the challenge proposed. A request the person declines issues
  nothing and spends nothing.
- Refusal cases. Just before its own issuance, the glue sends the owner one
  changed request and records the refusal. The run then continues, and
  nothing else is issued or spent. Each change is refused with its exact
  error:
  - an approval under a task-authorized plan, and the plan confirmation
    under an ask plan;
  - an approval of another preview;
  - a parent that confirms another plan, a parent of another task, a parent
    of another session, and a missing parent;
  - another policy;
  - a destination of the plan that the policy refuses;
  - a visit with a query field, and a query the plan did not disclose;
  - a target outside the parent's scope;
  - an unregistered tool version, and argument bytes that no longer match
    their digest;
  - the parent's own nonce;
  - an issuance instant at the packet's deadline, and one before the packet
    was prepared.
- More refusals. A second grant for the same call is refused. A cancelled
  budget and an already reserved request receive no grant. A parent that
  allows one grant refuses the person's approval of a second request. After
  the run ends, a new request inside the plan is refused.
- Start case. A start whose network scope is another domain that the policy
  also allows is refused before the grant is consumed. The reservation it
  made first stays spent.

## Limits

- Engine only. No host, worker, provider or model issues or uses these
  grants. Decision 0084 still forbids host or provider activation before the
  native adversarial matrix of AMR-03.2.3.
- The engine cannot see the person. A parent whose preview is the plan's
  digest records that the trusted host showed that plan to the person, just as
  every other approval digest does. Where a host shows the plan is AMR-03.2.3
  work.
- The issuance check is a point-in-time answer. The reservation at the start
  checks the budget again and spends. A grant whose reservation is later
  refused stays unused until it expires.
- The payload store in the composed test is in memory, as in Decision 0138.

## Consequences

- `kernel/engine/src/research_grant.rs` (new, a submodule of the operational
  store) adds the issuance owner, its request, its refusals and the issued
  grant.
- `kernel/engine/src/research_journal.rs` adds the budget owner's
  non-spending check.
- `kernel/engine/src/operational_store.rs` declares the submodule and adds
  the F1 clause. The store schema does not change.
- `kernel/engine/src/authority_transaction.rs` adds the destination binding
  at the start.
- `kernel/engine/src/runtime_loop_research_completion_tests.rs` issues
  through the owner and adds the ask, refusal and start cases and the F1
  mutation.
- `docs/architecture/canonical-research-source.md` describes issuance.
- Decision 0138 is corrected in place (N2).
- TASKS.md: AMR-02.2 is recorded with its evidence when the batch is
  verified, as a component row.

# Decision 0134: AMR-06 Decomposition

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0054, 0081, 0088 and 0124; the implementation amendment; current owner restart |
| Scope | AMR-06 split into rows with exact prerequisites; no capability is implemented by this decision |

## Findings

AMR-06 groups nine capabilities: CAP-14, 15, 27, 32, 37, 38, 40, 41 and 43. It
depends on AMR-04 and on pinned producer and consumer contracts. It asks the
runtime only for the adapters later delivery, team, knowledge and workflow
features need, and it leaves publication, scheduling and enterprise identity
with the Coordinator role. Four roles appear in the roadmap. The Runtime is
this project. The Coordinator, Memory and Host roles are separate consumers.
Their contracts are not in this repository.

The runtime already produces records such a consumer would read:

- the sealed run request;
- run declarations, now schema 4: recoverability, context views, the effect,
  job control and route histories, the route receipt and the recipe plan;
- job status from the durable job ledger;
- the stored action histories of an ended run.

Each is a versioned host wire type with closed decoding and tests, but no
document states which of them a consumer may rely on, and no fixture holds
them apart from the tests.

Every capability of AMR-06 already has preserved-plan rows that own its
detailed work:

| Capability | Role | Preserved-plan owners |
| --- | --- | --- |
| CAP-14 controlled Git delivery | Coordinator + Runtime | Sprints 47, 73, 85, 86 and 106 |
| CAP-15 CI and review repair | Coordinator | Sprints 73, 74 and 109 |
| CAP-27 cross-repository impact graph | Memory + Coordinator | Sprint 43 |
| CAP-32 authorized knowledge connectors | Runtime + Memory | Sprints 70, 87 and 88 |
| CAP-37 teams and roles | Coordinator | Sprint 95 |
| CAP-38 independent review and repair | Coordinator | Sprints 47, 74 and 95 |
| CAP-40 typed visual workflows | Coordinator + Host | Sprint 141 |
| CAP-41 team administration | Coordinator | Sprints 104 and 129 |
| CAP-43 private and offline deployment | Runtime + Host | Sprint 96 and AMR-05.9.5.1 |

## Decision

Split AMR-06 into ten rows and record every identity in the work selector.
The parent closes only when every row closes. No preserved-plan row is
replaced or closed by this decision.

| Row | Capability | Runtime scope | Prerequisites |
| --- | --- | --- | --- |
| AMR-06.1 | all | Pin the runtime's coordinator-facing producer contract: a versioned specification of the run request, run declarations, job status and ended-run histories a consumer may rely on, with producer fixtures written by the runtime's own types and a test that decodes them | AMR-04 host and CLI paths |
| AMR-06.2 | CAP-14 | Branch and commit adapters under exact grants; push, pull request, check and merge stay publication effects the Coordinator owns | AMR-06.1; a pinned Coordinator consumer contract; native Git trust; network and account grants for remote effects |
| AMR-06.3 | CAP-15 | Expose verification receipts and run declarations for one exact finding and commit; correction attempts stay with the Coordinator | AMR-06.1; a pinned Coordinator consumer contract |
| AMR-06.4 | CAP-27 | Produce versioned repository map relations with provenance for impact queries; no cross-repository write | AMR-06.1; a pinned Memory and Coordinator consumer contract |
| AMR-06.5 | CAP-32 | Connector reads under provider-specific authorization, retention and revocation | AMR-02.3 mediated network worker; provider accounts; a pinned Memory consumer contract |
| AMR-06.6 | CAP-37 | Keep role labels unable to mint authority at the grant boundary | AMR-06.1; a pinned Coordinator consumer contract |
| AMR-06.7 | CAP-38 | Expose immutable candidate and evidence packets for review in a separate context | AMR-06.1; a pinned Coordinator consumer contract |
| AMR-06.8 | CAP-40 | Execute workflow nodes through the existing runtime with schemas, provenance and effect ownership; no new scheduler | AMR-06.1; pinned Coordinator and Host consumer contracts |
| AMR-06.9 | CAP-41 | Accept organization policy that constrains models, tools and projects; the single-user path needs no organization login | AMR-06.1; a pinned Coordinator consumer contract |
| AMR-06.10 | CAP-43 | Qualify proxy and certificate configuration, offline installation, update channels and model and documentation packs, with actual no-egress validation | A native Linux host; packaging and update channels (Sprint 96, AMR-07) |

AMR-06.1 is dependency-ready in this lane. It follows the AMR-03.1.3 pattern.
It is an unexecuted specification, not a counterpart's approval, a consumer
wire contract or an integration proof. A consumer's own contract is pinned
separately and validated against the producer fixtures when it is offered.
The specification names only roles, never a private consumer.

Rows AMR-06.2 to AMR-06.10 stay open until their prerequisites exist. A
fixture success never substitutes for a missing consumer contract, network
grant, account or native host.

## Consequences

- TASKS.md gains rows AMR-06.1 to AMR-06.10. The parent row names this
  decision.
- `scripts/remaining_plan_blocker_audit.py` and the amendment work selection
  test record the ten identities.
- No source changes.

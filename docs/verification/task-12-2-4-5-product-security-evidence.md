# Task 12.2.4.5 Product Security Evidence

## Result

Pass for complete Story 12.2 acceptance-test execution and security-control mapping with explicit remaining release work.

Mapped acceptance tests and controls: **11 of 11**.

## Acceptance Tests

| Acceptance test | Story 12.2 result | Primary retained evidence | Remaining product gate |
|---|---|---|---|
| `AT-AGENT-001` | Pass in synthetic story scope | State, failure corpus, verifier, restart | Complete agent-snapshot storage, production adapters, and release rerun |
| `AT-CLASS-001` | Pass in synthetic story scope | Policy seed, classifier traces, context invariance, reclassification | Live fact collectors, production classifier, and release rerun |

## Control Map

| Requirement | Story 12.2 contribution | Primary retained evidence | Remaining product gate |
|---|---|---|---|
| `SR-ACC-001` | Demonstrated in story scope | Proposal identity, verifier, restart authority | Complete production effect composition |
| `SR-AI-003` | Demonstrated in story scope | Advisory authority matrix and false-completion campaign | Production model and rendered-UI evaluation |
| `SR-AI-005` | Partial story evidence | Allow-like and authority-field classifier attacks | Complete direct and indirect prompt-injection corpus |
| `SR-AI-007` | Demonstrated in story scope | Failure map, named terminals, failure corpus | Live UI rendering and broader semantic corpus |
| `SR-AI-015` | Partial story evidence | Exact proposal/context identities and closed classifier schema | Full candidate-neutral codec and production runtime corpus |
| `SR-AI-017` | Demonstrated in story scope | 5,120 policy mutations and 1,280 classifier outputs | Live fact collectors and production classifier rerun |
| `SR-AI-018` | Demonstrated in story scope | Verifier-only completion, ceilings, restart campaign | Complete agent-snapshot storage and product-loop integration |
| `SR-OPS-001` | Partial story evidence | Content-free receipts, source identities, campaign records | Complete durable product audit-event registry |
| `RV-17` | Demonstrated for agent-state scope | 104 state positions and six encrypted authority faults | Remaining model, checkpoint, shutdown, and release recovery fixtures |

## Retained Proof

- Raw policy seed and allocation are retained by
  [`d027-policy-campaign.json`](../../fixtures/agent-policy/v1/d027-policy-campaign.json).
- Transition coverage is retained by
  [`d027-s12-state.json`](../../artifacts/sprints/sprint-12/story-12.2/d027-s12-state.json).
- Denial invariance is retained by
  [`d027-s12-policy.json`](../../artifacts/sprints/sprint-12/story-12.2/d027-s12-policy.json).
- Classifier class traces and exact counts are retained by
  [`d027-classifier-campaign.json`](../../fixtures/agent-policy/v1/d027-classifier-campaign.json)
  and [`d027-s12-classifier.json`](../../artifacts/sprints/sprint-12/story-12.2/d027-s12-classifier.json).
- Verifier evidence is retained by
  [`verifier-registry.json`](../../artifacts/sprints/sprint-12/story-12.2/verifier-registry.json)
  and [`d027-s12-state.json`](../../artifacts/sprints/sprint-12/story-12.2/d027-s12-state.json).
- Restart receipt and replay assertions are retained by
  [`d027-restart-campaign.json`](../../fixtures/agent-policy/v1/d027-restart-campaign.json)
  and [`d027-s12-restart.json`](../../artifacts/sprints/sprint-12/story-12.2/d027-s12-restart.json).

## Limits

- This map closes Story 12.2 evidence organization and acceptance execution; it
  does not mark any security requirement complete for a product release.
- Acceptance tests use deterministic synthetic records. Production models,
  learned classifiers, platform workers, provider adapters, and live UI remain
  later integration and release gates.
- Complete agent-snapshot storage, full prompt-injection coverage, and the
  product audit-event registry remain later owning-sprint work.
- All retained records are local, content-minimized, private-data-free, and
  network-free. Cross-platform packaging and release acceptance remain later
  gates; manual fuzzing remains deferred to its assigned final gate.

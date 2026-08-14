# Task 12.2.2.3 Advisory Policy Results

## Result

Pass for authority-reducing advisory classification over exact deterministic clearance.

Focused cases closed: **6 of 6**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `ADV-01` | Allowed effects | Apply each of deny, narrow, redact, isolate, and escalate without authority | Pass |
| `ADV-02` | Combination | Normalize combined restrictions and preserve an unchanged deterministic boundary for an empty advisory | Pass |
| `ADV-03` | Result closure | Reject malformed digest, missing action, invalid confidence, and duplicate dispositions | Pass |
| `ADV-04` | Clearance identity | Reject task, action, or fact-set drift | Pass |
| `ADV-05` | Incomplete classifier | Keep seven incomplete or unavailable states from changing the boundary | Pass |
| `ADV-06` | Prohibited authority | Reject grant, denial-override, destination, model-switch, execution, and completion fields | Pass |

## Evidence

- [`advisory_policy.rs`](../../kernel/engine/src/advisory_policy.rs) requires an opaque deterministic `PreclassificationClearance` before evaluating advisory output.
- Clearance is bound to the exact task, action, canonical deterministic fact digest, and policy revision.
- Complete schema-valid output normalizes only `deny`, `narrow`, `redact`, `isolate`, and `escalate`; an empty advisory preserves rather than broadens the deterministic boundary.
- `AdvisoryBoundaryDecision` has no grant, operation, destination-selection, model-switch, execution, denial-override, or completion method.
- Six focused tests exercise every permitted disposition, deterministic normalization, malformed and duplicate output, identity drift, every incomplete status, prohibited-field injection, and denial non-override.

## Limits

- Task 12.2.2.4 assigns explicit narrower, user-decision, or blocked outcomes to incomplete classifier states; this task only prevents them from changing clearance.
- Tool-set subtraction and scope/redaction/isolation materialization remain later typed product-boundary integrations.
- Tests use synthetic content-free facts and classifier records with no model, classifier runtime, tool, worker, platform effect, private user data, or external network operation.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.

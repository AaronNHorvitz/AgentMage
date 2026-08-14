# Task 12.2.2.1 Classification Contract Results

## Result

Pass for five separate closed deterministic and advisory classification schemas.

Focused cases closed: **6 of 6**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `CLS-01` | Data sensitivity | Round-trip all four sensitivity labels in a dedicated assessment | Pass |
| `CLS-02` | Action risk | Round-trip all four risk labels without sensitivity or model fields | Pass |
| `CLS-03` | Model capability | Round-trip all seven roles and four measured dispositions | Pass |
| `CLS-04` | Deterministic facts | Retain the complete typed policy-fact envelope with no advisory fields | Pass |
| `CLS-05` | Advisory result | Retain eight availability states and only five restrictive or escalating dispositions | Pass |
| `CLS-06` | Schema isolation | Reject cross-schema input, unknown authority fields, and an unsupported allow-like disposition | Pass |

## Evidence

- [`classification.rs`](../../kernel/contracts/src/classification.rs) defines distinct versioned assessments for data sensitivity, action risk, and measured model capability.
- The same module defines one deterministic fact envelope over actor, session, task, action, autonomy, operation, source, destination, path, repository, credential, data, risk, model capability, reversibility, network, disclosure, budget, exact authority, and static checks.
- `AdvisoryClassifierResult` carries only classifier identity, input identity, availability state, restrictive dispositions, optional confidence, rationale identity, and evidence.
- The advisory schema has no grant, operation, destination, model-selection, execution, or completion field; unknown fields and an allow-like disposition fail parsing.
- Six focused tests cover every sensitivity, risk, model-role, model-status, classifier-status, and advisory-disposition enum value plus cross-schema and authority-field attacks.

## Limits

- This task defines closed wire schemas; deterministic fact collection and policy evaluation begin in Task 12.2.2.2.
- Advisory disposition enforcement, failure mapping, and continuous boundary reclassification remain Tasks 12.2.2.3 through 12.2.2.5.
- Tests use synthetic content-free records with no model, classifier, tool, worker, platform effect, private user data, or external network operation.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.

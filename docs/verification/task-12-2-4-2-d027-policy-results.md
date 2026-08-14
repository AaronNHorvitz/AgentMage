# Task 12.2.4.2 D027 Policy Results

## Result

Pass for `D027-S12-POLICY`; 5,120 seeded fact mutations produced zero decision drift.

Focused cases closed: **4 of 4**.

## Cases

| Case | Campaign boundary | Expected result | Result |
|---|---|---|---|
| `D027-POLICY-01` | Reproducible generator | Retain one fixed LCG seed and constants | Pass |
| `D027-POLICY-02` | Dimension coverage | Execute exactly 320 mutations in each of 16 required dimensions | Pass |
| `D027-POLICY-03` | Deny-first invariance | Evaluate every mutation twice with an identical clearance or first denial | Pass |
| `D027-POLICY-04` | Outcome diversity | Exercise at least one clearance and one denial without network or private data | Pass |

## Evidence

- [`d027-policy-campaign.json`](../../fixtures/agent-policy/v1/d027-policy-campaign.json)
  retains the seed, LCG constants, ordered dimensions, allocation, and expected
  result.
- The Rust campaign mutates actor, session, task, autonomy, operation, source,
  destination, path, repository state, credential, label, reversibility,
  network, disclosure, budget, and authority.
- Each fact set is evaluated twice through `PreclassificationPolicyGate`; the
  full `Result` must match, including the deterministic clearance or the exact
  denial reason and completed-check position.

## Limits

- The campaign uses deterministic synthetic fact records and does not collect
  live platform facts or invoke a semantic classifier.
- It proves in-process policy reproducibility, not production execution,
  provider integration, or cross-platform packaging.
- No private user data or external network operation is used.
- Release acceptance and manual fuzzing remain later tasks and gates.

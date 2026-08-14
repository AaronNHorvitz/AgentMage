# Task 12.2.4.3 D027 Classifier Results

## Result

Pass for `D027-S12-CLASSIFIER`; 1,280 advisory outputs produced zero broader authority or completion.

Focused cases closed: **5 of 5**.

## Cases

| Case | Campaign boundary | Expected result | Result |
|---|---|---|---|
| `D027-CLASS-01` | Output volume | Exercise 128 outputs in each of ten hostile or valid classes | Pass |
| `D027-CLASS-02` | Allow-like output | Reject unknown allow, approved, safe, and authorized fields | Pass |
| `D027-CLASS-03` | Failure states | Map all seven incomplete statuses to fixed safe actions | Pass |
| `D027-CLASS-04` | Authority escalation | Reject grant, operation, destination, model, execution, and completion fields | Pass |
| `D027-CLASS-05` | Complete output | Preserve only deny, narrow, redact, isolate, and user escalation | Pass |

## Evidence

- [`d027-classifier-campaign.json`](../../fixtures/agent-policy/v1/d027-classifier-campaign.json)
  retains the class schedule, expected counts, and prohibited capabilities.
- The Rust campaign produces 256 parser rejections, 896 deterministic safe
  failure decisions, and 128 complete restrictive decisions.
- Confidence varies from 0 through 10,000 basis points and never changes the
  fixed failure mapping.
- Every output is bound to one deterministic clearance or rejected before an
  advisory decision exists.

## Limits

- The campaign uses synthetic classifier records and does not run a production
  model or learned classifier.
- It proves the typed in-process advisory boundary, not platform execution,
  provider integration, or cross-platform packaging.
- No private user data or external network operation is used.
- Release acceptance and manual fuzzing remain later tasks and gates.

# Task 12.2.3.3 Agent Failure Corpus Results

## Result

Pass for the versioned, content-free agent and policy failure corpus.

Focused cases closed: **6 of 6**.

## Cases

| Case | Category | Expected result | Result |
|---|---|---|---|
| `APF-001` | Restart | Lock transitions until exact reconciliation | Pass |
| `APF-002` | Uncertain effect | Block resume and duplicate effect | Pass |
| `APF-003` | Consumed grant | Reject unreconciled grant replay | Pass |
| `APF-004` | False completion | Reject nondeterministic completion evidence | Pass |
| `APF-005` | Classifier disagreement | Require an explicit user decision | Pass |
| `APF-006` | No progress | Enter the sticky `STALLED` terminal state | Pass |

## Evidence

- [`failure-corpus.json`](../../fixtures/agent-policy/v1/failure-corpus.json)
  retains the six synthetic, content-free scenarios.
- Every case binds an exact source test and a SHA-256-identified retained
  Sprint 12 artifact.
- The validator recomputes artifact hashes, confirms source-test ownership,
  enforces one case per category, and requires every case to prohibit false
  success.

## Limits

- The corpus indexes focused deterministic unit evidence; it is not the full
  crash-before-and-after transition campaign required by Task 12.2.4.4.
- The cases use no production worker, model, classifier, tool, provider,
  private user data, or external network operation.
- Encrypted persistence integration, cross-platform execution, packaging,
  release acceptance, and manual fuzzing remain later tasks and gates.

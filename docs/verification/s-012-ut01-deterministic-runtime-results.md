# S-012-UT01 Deterministic Runtime Results

## Result

Pass for deterministic state, objective, revision, budget, stop, and completion boundaries.

Focused cases closed: **5 of 5**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `DVR-01` | Loop transition matrix | Exercise 20 phase/method pairs, admit the four phase methods only from their exact source phase, and admit the four declared stop edges | Pass |
| `DVR-02` | Objective bounds | Reject empty and 4,097-byte objectives with identical typed, field-scoped findings across repeated runs | Pass |
| `DVR-03` | Plan revision | Advance each legal immutable transition by one and preserve revision/history after an illegal transition | Pass |
| `DVR-04` | Budget boundary | Admit the exact inclusive limit, reject one-unit overage without accounting it, and retain the first stop reason | Pass |
| `DVR-05` | Stop and completion | Preserve five signaled stops, reject direct completion/budget and undeclared deadline signals, and require newer complete evidence | Pass |

## Evidence

- [`agent_runtime.rs`](../../kernel/engine/src/agent_runtime.rs) contains five named `s_012_ut01_*` tests over the public runtime, plan-progress, run-control, and work-packet contracts.
- The targeted Cargo test runs only those five tests and reports every case passing.
- Warning-denying Clippy verifies the complete kernel-engine target set.
- [`deterministic_runtime_evidence.py`](../../scripts/deterministic_runtime_evidence.py) binds this report, source, and command identities to an immutable Git revision.
- [`test_deterministic_runtime_evidence.py`](../../tests/test_deterministic_runtime_evidence.py) rejects source, case, claim, command, revision, limitation, and artifact identity drift.

## Limits

- The suite exercises deterministic in-process kernel state and accounting only.
- Actions remain descriptive proposals; no model, tool, grant, shell worker, platform adapter, or filesystem effect executes.
- The transition matrix does not claim operating-system cancellation, persisted restart recovery, or UI behavior.
- Synthetic fixtures contain no private user data and use no external network.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.

# S-012-RT01 Cancellation and Recovery Results

## Result

Pass for interruption propagation, exactly-once cleanup, and truthful terminal reporting.

Focused cases closed: **5 of 5**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `CRV-01` | Named stages | Interrupt model calls, tools, planning, status rendering, and finalization; clean each stage exactly once | Pass |
| `CRV-02` | Replacement | Cancel the active plan and every shell, kernel, model, and tool descendant before replacing work | Pass |
| `CRV-03` | Status | Keep a status query non-interrupting while making status rendering observe an existing cancellation | Pass |
| `CRV-04` | Finalization | Refuse to convert cancelled or evidence-incomplete work into a completed final response | Pass |
| `CRV-05` | Sticky cancellation | Preserve the first signal for late descendants and keep repeated cleanup idempotent | Pass |

## Evidence

- [`s012_rt01.rs`](../../kernel/engine/src/s012_rt01.rs) is a test-only interruption harness over the public cancellation, progress, replacement, and final-response contracts.
- The suite exercises five named interrupt stages and four cancellation-tree boundaries.
- Every interrupted stage completes cleanup exactly once and records zero false success claims.
- [`cancellation_recovery_evidence.py`](../../scripts/cancellation_recovery_evidence.py) binds this report, the Rust harness, and exact verification commands to an immutable Git revision.
- [`test_cancellation_recovery_evidence.py`](../../tests/test_cancellation_recovery_evidence.py) rejects case, claim, command, source, revision, limitation, and artifact identity drift.

## Limits

- This is test-only in-process verification; it does not prove operating-system process termination or durable restart cleanup.
- No production model, tool, shell worker, platform adapter, filesystem effect, or external network request executes.
- Status and final-response records are synthetic contract objects rather than live UI transcripts.
- Cross-platform execution, persistence, packaging, release acceptance, and manual fuzzing remain later gates.

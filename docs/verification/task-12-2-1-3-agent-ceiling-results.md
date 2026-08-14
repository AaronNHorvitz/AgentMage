# Task 12.2.1.3 Agent Ceiling Results

## Result

Pass for complete deterministic agent ceilings and one sticky terminal result.

Focused cases closed: **6 of 6**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `ACL-01` | Inclusive limits | Admit the exact limit and reject one-unit overage for all 13 resources | Pass |
| `ACL-02` | Sticky terminal | Preserve the first breach across every later resource event | Pass |
| `ACL-03` | Terminal mapping | Map no-progress to `STALLED` and every other breach to `EXHAUSTED` | Pass |
| `ACL-04` | Profile closure | Reject a missing, duplicate, or zero-valued ceiling | Pass |
| `ACL-05` | Overflow | Stop on counter overflow without changing admitted usage | Pass |
| `ACL-06` | Determinism | Produce identical usage for equivalent event partitioning | Pass |

## Evidence

- [`agent_ceiling.rs`](../../kernel/engine/src/agent_ceiling.rs) defines turns, input/output/context tokens, retries, denials, tools, effects, elapsed time, memory, disk, processes, and repeated no-progress as a closed 13-resource set.
- Construction requires one unique positive limit for every resource.
- Overage and overflow do not change admitted usage and install one exact terminal breach.
- Six focused tests exercise all boundaries, terminal mapping, profile failures, overflow, and deterministic partitioning.

## Limits

- The controller is in-memory; durable state integration and restart reconciliation remain later Story 12.2 tasks.
- Platform collectors that supply observed memory, disk, process, and elapsed-time values remain later integration work.
- No model, tool, grant, worker, platform effect, private user data, or external network operation is used.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.

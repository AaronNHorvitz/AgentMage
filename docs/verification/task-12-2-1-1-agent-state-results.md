# Task 12.2.1.1 Agent State Results

## Result

Pass for the closed persisted agent-state and legal-transition contract.

Focused cases closed: **6 of 6**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `AST-01` | State catalog | Preserve eight active and nine exact named terminal states with stable wire names | Pass |
| `AST-02` | Transition matrix | Give all 289 state pairs one deterministic disposition and admit exactly 52 edges | Pass |
| `AST-03` | Success authority | Reject ordinary transitions to `SUCCESS` and verified `NO_OP` without mutation | Pass |
| `AST-04` | Terminal behavior | Make every terminal state sticky with no outgoing edge | Pass |
| `AST-05` | Persistence shape | Append monotonic revisioned transition records without overwriting history | Pass |
| `AST-06` | Verifier attachment | Declare success edges from verification/checkpoint while exposing no callable ordinary route | Pass |

## Evidence

- [`agent_state.rs`](../../kernel/contracts/src/agent_state.rs) defines the shared serializable 17-state contract and append-only transition record.
- [`agent_state.rs`](../../kernel/engine/src/agent_state.rs) implements the 52-edge deterministic graph and bounded 4,096-transition controller.
- Ordinary callers cannot produce either verified success state; the verifier registry remains the only planned completion route.
- Six focused engine tests plus the contract terminal-set test cover state identity, every pair, success denial, terminal stickiness, history, and the future verifier attachment boundary.

## Limits

- This task defines in-memory shared contracts and controller behavior; durable store integration begins in later Story 12.2 tasks.
- Proposal identity, ceilings, verifier records, restart reconciliation, deterministic policy, and advisory classification remain separate sub-tasks.
- No model, tool, grant, worker, platform effect, private user data, or external network operation is used.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.

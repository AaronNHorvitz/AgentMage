# S-012-I01 Bounded Agent Runtime Results

**Status:** Pass for the current in-process, non-executing kernel boundary  
**Task:** `12.1.1.1` / legacy `S-012-I01`  
**Scope:** Configuration-bound observe, plan, action-proposal, review, budget, stop, and completion control

## Result

The kernel now composes the validated configuration, active `WorkPacket`,
deterministic packet-to-plan adapter, and sticky `RunController` into one
`AgentRuntime`. The runtime can describe the next phase, account for an action
proposal, and require review. It has no grant issuer, tool executor, model
client, platform driver, or success path based on prose.

Focused cases closed: **8 of 8**.

| Case | Boundary | Verified result |
|---|---|---|
| `AR-01` | Complete loop | Observe, deterministic plan, bounded tool proposal, exact review, and next observation preserve profile/configuration identity and admit one cycle. |
| `AR-02` | Phase order | Plan, action, and review calls outside their exact phase fail before budget or state changes. |
| `AR-03` | Action input | Unsupported schema, wrong task, wrong state, empty identity, missing/foreign step, empty/duplicate effects, empty description, and disabled-model proposals fail before accounting. |
| `AR-04` | Budgets | Input overrun enters one sticky terminal state without admitting bytes or a later phase. The positive loop separately meters plan steps, tool calls, input, output, and elapsed time. |
| `AR-05` | Review | Wrong action identity is inert; policy denial, error, cancellation, uncertainty, and user-decision requirements map to their exact non-success stop condition. |
| `AR-06` | User decision | A user-decision action stops without a pending review, tool call, or model call. |
| `AR-07` | Completion | Missing completion evidence is rejected; only a newer evidence-complete packet reaches `AcceptanceSatisfied`. |
| `AR-08` | Construction | Missing mandatory stop conditions or all run budgets prevents runtime creation. |

## Authority and Side Effects

- Action inputs must remain fresh descriptive `Proposed` records bound to the
  exact task and one generated plan step.
- A returned `Review` directive means only that the proposal passed this
  descriptive loop boundary. Kernel policy, a current single-use grant,
  platform mediation, and postcondition verification remain mandatory.
- Tool executions: **0**.
- Model invocations: **0**. The exact test profile has no enabled model, and a
  model-inference proposal is refused before plan-step or model-call accounting.
- External network attempts: **0**.
- Private user data used: **0**.

## Limits

- State is in process for this sub-task. Operational-store persistence,
  restart reconciliation, named terminal results, and consumed-grant recovery
  remain Story 12.2 work.
- Plan history, one-active-step transitions, progress/status events, new-user-
  message handling, and cancellation transcripts remain `12.1.1.3` and later
  verification tasks.
- No production tool, model, worker, interface, platform, package, or release
  workflow is exercised or claimed.
- Manual fuzzing remains deferred and is not represented by these deterministic
  tests.

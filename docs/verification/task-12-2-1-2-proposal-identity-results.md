# Task 12.2.1.2 Proposal Identity Results

## Result

Pass for exact proposal identity binding and inert rejection.

Focused cases closed: **6 of 6**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `PIV-01` | Exact tuple | Canonically round-trip and admit one proposal bound to every required identity | Pass |
| `PIV-02` | Parser | Reject malformed, partial, duplicate-field, unknown-field, and oversized input | Pass |
| `PIV-03` | Identity drift | Reject each ownership, model/context, evaluation-context, and correlation substitution without mutation | Pass |
| `PIV-04` | Turn | Reject stale and future turns without recording a candidate | Pass |
| `PIV-05` | Replay/ambiguity | Reject exact replay and a competing proposal while preserving the first admitted identity | Pass |
| `PIV-06` | Invalid identity | Reject zero turn, empty identity, and malformed digest before registry admission | Pass |

## Evidence

- [`agent_proposal.rs`](../../kernel/contracts/src/agent_proposal.rs) binds schema, proposal, session, task, turn, model-run, context-packet, repository-snapshot, tool-catalog, policy, correlation, and proposal-digest identities.
- [`agent_proposal.rs`](../../kernel/engine/src/agent_proposal.rs) admits at most one exact candidate for one expected turn and retains replay identities only after full tuple validation.
- The shared closed JSON parser rejects malformed, missing, duplicate, unknown, unsupported, trailing, and oversized input before an `AgentProposal` exists.
- The admitted record contains only proposal identity, turn, and digest and cannot represent an operation, grant, destination, execution, or completion claim.

## Limits

- The proposal digest identifies separately validated closed bytes; the model codec and payload validator remain Sprint 13 work.
- Durable proposal persistence and restart reconciliation remain later Story 12.2 tasks.
- No model, tool, grant, worker, platform effect, private user data, or external network operation is used.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.

# Task 12.2.1.5 Restart Reconciliation Results

## Result

Pass for exact restart reconciliation before another restored agent-state transition.

Focused cases closed: **6 of 6**.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `RST-01` | Clean restart | Reconcile the exact snapshot and unlock one matching restored state revision | Pass |
| `RST-02` | Current context | Reject task, repository, policy, profile, state, or revision drift | Pass |
| `RST-03` | Authority lifecycle | Block issued authority and consumed grants lacking terminal reconciliation | Pass |
| `RST-04` | Completed effects | Admit an exact consumed grant, terminal transaction, and content-addressed receipt chain | Pass |
| `RST-05` | Uncertain effects | Reject launched, reconciling, grant-uncertain, and terminal-uncertain effects | Pass |
| `RST-06` | Integrity and replay | Reject receipt duplication, tampering, and a permit applied to another state revision | Pass |

## Evidence

- [`agent_restart.rs`](../../kernel/contracts/src/agent_restart.rs) defines one closed versioned restart snapshot containing current task, repository, policy, selected profile, agent state, authority transactions, grants, and receipts.
- [`agent_restart.rs`](../../kernel/engine/src/agent_restart.rs) validates exact context, bounded unique records, policy-bound grants, receipt order and hashes, terminal bindings, unresolved consumption, pending authority, and uncertain effects.
- Only `RestartReconciler` can construct the opaque, non-cloneable `RestartPermit`.
- [`agent_state.rs`](../../kernel/engine/src/agent_state.rs) keeps restored controllers locked until a permit matches the exact active state and monotonic revision.
- Six focused tests exercise a clean restart, all current-context fields, pending and consumed authority, completed effect history, every represented uncertainty path, receipt integrity, and permit mismatch.

## Limits

- This proves the in-process agent restart gate over typed persisted facts; wiring the snapshot to the encrypted operational store remains a later integration task.
- Existing durable authority recovery resolves interrupted effect transactions before this gate, but the complete crash-before-and-after-every-agent-transition campaign remains Task 12.2.4.4.
- Tests use synthetic authority and receipt records with no model, classifier, tool, worker, platform effect, private user data, or external network operation.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.

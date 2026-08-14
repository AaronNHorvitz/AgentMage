# Task 12.2.4.4 D027 Restart Results

## Result

Pass for `D027-S12-RESTART`; every declared interruption reconciled with no replay, duplicate effect, duplicate receipt, or false success.

Focused cases closed: **6 of 6**.

## Cases

| Case | Interruption boundary | Expected result | Result |
|---|---|---|---|
| `D027-RESTART-01` | State | Reconcile before and after all 52 legal edges | Pass |
| `D027-RESTART-02` | Grant | Recover around prepare and grant-consumption persistence | Pass |
| `D027-RESTART-03` | Worker | Recover around launch commitment and worker return without relaunch | Pass |
| `D027-RESTART-04` | Receipt | Recover before publication and after hash-chained terminal receipt | Pass |
| `D027-RESTART-05` | Verifier | Lose unapplied proofs safely and terminalize only with fresh proof | Pass |
| `D027-RESTART-06` | Terminal | Return one idempotent terminal receipt and reject replay | Pass |

## Evidence

- [`d027-restart-campaign.json`](../../fixtures/agent-policy/v1/d027-restart-campaign.json)
  maps every interruption point to its semantic boundary.
- The state campaign executes 104 before/after positions over all 52 legal
  edges. Active states require an exact restart permit; terminal states cannot
  be restored and have no outgoing edge.
- The authority campaign injects all six transaction faults into both in-memory
  and encrypted durable-store recovery, then attempts the same transaction
  again with a fresh driver. Every replay is rejected before launch.
- Receipt recovery retains exactly one terminal receipt per transaction, checks
  its hash chain, and returns the same receipt when terminal recovery repeats.
- Verifier-proof loss is tested for both `SUCCESS` and `NO_OP`; creating a proof
  has no state effect, and ordinary transitions cannot replace verification.

## Limits

- Agent-state snapshots are exercised as typed restart records; direct storage
  of the complete agent snapshot remains a later operational integration.
- Authority transaction cases do use the encrypted local operational store,
  but no production model, provider, platform worker, or external service.
- No private user data or external network operation is used.
- Cross-platform packaging, release acceptance, and manual fuzzing remain later
  tasks and gates.

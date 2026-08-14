# S-012-I05 Material Claim Results

**Status:** Pass for typed evidence-required material claims

**Task:** `12.1.1.5` / legacy `S-012-I05`

**Scope:** Read, change, test, commit, push, publish, and completion claims

## Result

The kernel now keeps proposed material claims unverified until an exact,
current, subject-bound proof satisfies the closed contract for that claim
class. Evidence roles cannot be substituted, added, omitted, duplicated, made
stale, or reused for another claim. A claim-bound final response requires one
verified completion claim after the complete ordered set of its independently
verified prerequisites.

Focused cases closed: **6 of 6**.

| Case | Boundary | Verified result |
|---|---|---|
| `MCL-01` | Claim closure | Read, change, test, commit, push, and publish each reject empty proof and accept only their exact current role set. |
| `MCL-02` | Proof integrity | Stale revision, wrong evidence kind, duplicate identity, omitted role, and extra role fail closed. |
| `MCL-03` | Completion prerequisites | Completion cannot verify until every declared material prerequisite is verified. |
| `MCL-04` | Final response | Omitted, unverified, reordered, duplicated, or missing claims cannot enter a claim-bound final response. |
| `MCL-05` | Evidence reuse | One evidence identity cannot be consumed to prove a second material claim. |
| `MCL-06` | Authority | Proposed, verified, and final claim records remain descriptive and always denied as authority. |

## Exact Proof Roles

| Claim | Required proof roles |
|---|---|
| Read | Operation receipt and read observation |
| Change | Operation receipt and postcondition observation |
| Test | Tool output and deterministic test validation |
| Commit | Operation receipt and commit observation |
| Push | Operation receipt and remote-ref observation |
| Publish | Operation receipt and publication observation |
| Complete | Independent acceptance verification after all declared prerequisite claims |

Every proof reference binds the claim's exact subject and expected revision.
The role also requires its exact evidence kind. No record in this family can
issue or widen authority.

## Limits

- The boundary verifies explicitly typed material claims. Natural-language
  extraction of every claim from arbitrary response prose remains later
  model-output and integration work.
- The ledger is in memory only; encrypted persistence, restart reconciliation,
  and production session/UI composition remain later work.
- Synthetic evidence references exercise the claim protocol. No file, test
  command, Git remote, publication provider, model, or platform worker runs.
- Cross-platform, packaging, release, and quantitative adversarial acceptance
  remain later gates.
- Manual fuzzing remains deferred and is not represented by these deterministic
  tests.

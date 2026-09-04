# Decision 0047: First-GA Freeze Completion and Full-Plan Continuation

| Field | Value |
|---|---|
| Status | Accepted execution-scope decision |
| Date | 2026-09-04 |
| Scope | Records completion of the Decision 0046 frozen-set pass, ends that scope freeze, and resumes execution across the complete preserved plan: Epics 0 through 16 and the Universal Story Definition of Done |
| Supersedes | Decision 0046 items 1 through 3 only: the active first-GA scope freeze and its restriction of the Decision 0021 execution rule to Epics 0 through 8, 10, 11, and the Universal Story Definition of Done |
| Preserves | Every stable identifier, addition-only history, completed artifact, exact blocked disposition, Decision 0021 execution rule, Decision 0040 execution venue, Decision 0045 first-release mandate, evidence binding, and current status truth |
| Does not authorize | The AGENTS.md section 5 option; substitution of platform, model, provider, credential, signing, reviewer, or release evidence; deletion or renumbering; weakening evidence binding; closure of blocked rows; release publication |

## Context

Decision 0046 began with 1,842 of 5,148 task rows closed and 3,306 open. Its
verification baseline counted 2,248 open rows in the executable frozen set, 1,007
open rows outside the freeze, and approximately 55 additional physical-platform
rows retained as blocked rather than executable work.

At the 2026-09-04 boundary audit, 3,162 of 5,148 rows are closed and 1,986 remain
open. The complete preserved plan therefore advanced by 1,320 net row closures.
The unattended run recorded 1,328 closure events because eight newly introduced
rows were both added and closed during the freeze. Exactly 1,007 open rows remain
in Epics 9 and 12 through 16. The other 979 open rows remain in the former frozen
set and are downstream of exact external platform, credential, signing,
instrumentation, reviewer, release-owner, or strict-local host blockers. Their
substitution sets are empty.

The Decision 0046 execution pass has therefore reached its authorized terminal
condition: every locally executable row in its scope is closed with committed
evidence and every remaining row is blocked on a specific external act. Keeping
Epics 9 and 12 through 16 outside the execution scope would now prevent progress
despite 1,007 unblocked preserved rows.

## Decision

1. The Decision 0046 first-GA scope-freeze pass is complete. Its blocked rows
   remain open with their exact blockers and empty substitution sets.
2. The stabilization scope freeze is **inactive (Decision 0047)**. Epics 9 and 12
   through 16 join Epics 0 through 8, 10, 11, and the Universal Story Definition
   of Done as the active complete preserved plan.
3. Decision 0021's first-authoritative-incomplete-dependency-gate rule applies
   across that complete plan. Exact blocked rows and their dependents are skipped
   only while their recorded external prerequisite remains unavailable.
4. All work continues under the same batching, review-pin, evidence-generation,
   hash-binding, truthful-status, and no-substitution requirements.
5. Decision 0046's physical-platform and Decision 0040 venue constraints remain
   binding. This decision changes execution scope, not support or release status.
6. The eight `documentation_contract` documents and
   `architecture/status-model.json` carry the same Decision 0047 inactive-freeze
   marker, and the status-model validator recognizes `ADR-0047` as an amendment.

## Consequences

- Sprint 101 is the next authoritative incomplete dependency gate because Epic 9
  is the first newly opened epic in plan order.
- The 1,007 formerly excluded open rows are eligible for unattended execution.
- The 979 open rows from the Decision 0046 set do not become complete and do not
  gain substitutes; each remains governed by its recorded blocker.
- No product, component, platform, model, package, or support promotion follows
  from ending the freeze. Promotions still require bindable committed evidence.
- Decision 0047 authorizes no architectural capability and no evidence shortcut;
  it changes only the execution scope after the Decision 0046 pass completed.

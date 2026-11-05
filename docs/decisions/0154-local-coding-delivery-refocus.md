# Decision 0154: Local Coding Delivery Refocus

| Field | Value |
| --- | --- |
| Status | Accepted under explicit owner instruction |
| Date | 2026-11-04 |
| Authority | Owner refocus, 2026-11-04; Decision 0153 |
| Scope | Finish evaluation, measure the baseline, build the coding engine |

## Decision

1. Finish the commit already in progress when the refocus arrived. Retain its
   independent exact-commit review. Fix any high or medium finding in the same
   row; low findings become backlog notes. Create no further per-task or
   per-finding T-311 rows.
2. Resolve the existing T-311.15/T-311.16 cleanup finding together. The reviewed
   third attempt still has a medium finding, so its existing report cannot
   establish closure. Repair it in these rows and request a combined independent
   verdict under this explicit owner restart. Do not restart a separate sequence
   of cleanup sub-rows. Close only on an exact-commit PASS without open high or
   medium findings.
3. Complete the suite in one batch: at least 30 development tasks and 10 held-out
   tasks, both frozen, using the existing grading machinery. Never tune against
   held-out tasks, edit hidden tests or weaken thresholds. Resolve admission and
   integrity requirements within that batch, rather than creating a row for
   each adapter or review finding.
4. Measure the current harness before changing the agent. Publish actual local
   model first-attempt and with-retry pass rates, time, tokens, GPU minutes and
   failures, with raw logs, in docs/evidence/coding-scoreboard.md. Only then is
   parent T-311 eligible for independent closure.
5. The owner names the suite batch T-311.17 and the baseline T-311.18. These
   identifiers already identify completed oracle work in the active queue.
   Extend their scope for the new assignments when implementing them, preserving
   the original exact-commit reviews and completed milestones in their text.
   Those historical PASS verdicts do not approve the expanded assignments.
   Preserve the older programme ledger and its checkboxes.
6. Then implement T-303 and T-306 together: repository context, strict edits,
   confined tests, bounded retries, isolated worktrees, diff review and approved
   commits and pushes. Follow with T-312 model measurements and T-313 team mode.
   Add a development scoreboard entry after each row; run held-out tasks only
   for row-closing scoreboard entries. Product team mode does not replace the
   fixed separate independent reviewer.
7. Split a row only when it requires more than one session, at most one level
   deep, and never because of a review finding. The existing three-review rule
   continues outside the explicit cleanup restart above. No evidence, permission,
   sandbox, resource, IP, privacy or release boundary is relaxed.

## Current truth

The current collection has 30 development tasks and no held-out set. The independent
oracle covers 19 historical control pairs; generated-candidate admission is disabled.
There is no current-harness coding baseline. Passing controls are not model scores.
The coding loop remains ahead of documents, research and media. No public release,
model acquisition or owner-ready declaration is authorized by this decision.

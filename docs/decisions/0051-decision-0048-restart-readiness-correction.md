# Decision 0051: Decision 0048 Restart-Readiness Correction

| Field | Value |
|---|---|
| Status | Accepted corrective product-planning decision |
| Date | 2026-09-06 |
| Scope | Corrects Decision 0048 story identity, baseline, dependency-registration, applicability, and blocker-selection defects before unattended execution resumes |
| Corrects | Decision 0048 items 5, 6, 9, 10, Verification, and its September 6 baseline wording |
| Preserves | Story 25.2, every stable identifier and completed artifact, Decisions 0021 and 0040 through 0048, the full preserved plan opened by Decision 0047, exact genuine blockers, additions-only history, evidence binding, and current product truth |
| Does not authorize | Reopening or closing a genuinely blocked row; treating unknown work as unblocked or external; evidence substitution; model activation; network egress during normal question-answering; signing, publication, purchase, hosted execution, secret access, or a release claim |

## Context

Decision 0048 intentionally deferred executable story numbering to its
implementing batch. Its proposed Story 25.2 collides with the accepted Incident
Tabletop and Release Support Readiness story. It also repeated a blocker-audit
claim whose sprint-paragraph classifier did not prove row-level externality,
mixed two checkbox denominators, named Decision 0047 where the interface text
meant Decision 0048, and retained one obsolete "frozen plan" phrase after
Decision 0047 opened the complete preserved plan.

The owner directed the bounded restart-readiness implementation batch on
2026-09-06 after receiving the revised audit and completion prompt. This
decision records the precise corrective governance required by that direction;
it does not infer approval for any external or release action.

## Decision

1. Story 25.2 remains **Incident Tabletop and Release Support Readiness**.
   **Story 25.3** is the **v1.0-preview Release Gate**. Every current preview-gate
   reference uses Story 25.3; Decision 0048's proposed Story 25.2 text remains
   historical and is explicitly corrected rather than silently rewritten.
2. Stories **76.2**, **76.3**, **77.2**, and **25.3** are registered additions
   in `TASKS.md`, `IMPLEMENTATION-PLAN.md`, and the machine-checked release
   applicability graph. They retain the Universal Story Definition of Done.
3. The audited `b62eb543` detailed-item baseline is **4,041 done / 1,107 open /
   5,148 total**. The all-checkbox baseline, including headings, is **4,094 done
   / 1,484 open / 5,578 total**. These denominators are never mixed, and later
   counts do not replace this historical baseline.
4. Decision 0048 item 6(g) means the **remainder of the complete preserved plan
   in numbered order**, not the remainder of a frozen plan. The full plan opened
   by Decision 0047 remains authoritative.
5. Decision 0048's assertion that all 1,484 open rows were externally blocked is
   retained as historical context, not accepted as a row-level proof. Exact
   genuine blocked dispositions and their history remain binding. Each open row
   is now classified from its own prerequisite record and dependency path as
   `local`, `dependency`, `external`, or `unknown`. Unknown is neither external
   nor executable and cannot be bypassed.
6. The v1.0-preview dependency selector fails closed on missing stories,
   unknown dependency identities, cycles, missing applicability, unknown rows,
   and externally blocked prerequisites. It may select only a local leaf whose
   exact dependencies are satisfied, in Decision 0048 critical-path order.
7. Release applicability is explicit and separate:
   `v1.0-preview-windows` covers the hardened standalone Windows preview;
   `v1.0-full-ga` retains the complete later GA scope; and
   `retained-platforms` preserves Apple Silicon macOS, Fedora, Ubuntu, and the
   secondary Visual Studio Code surface without making them preview substitutes.
8. "Read-only" forbids writes to user-selected source documents. It does not
   forbid authorized encrypted application state required for sessions,
   receipts, evidence, recovery, configuration, or locally assembled
   diagnostics.
9. Normal preview question-answering is offline and uses no credential or cloud
   dependency. Model acquisition, signed update retrieval, and diagnostic
   transmission are three distinct network phases, each fail-closed and
   separately consented. Diagnostic transmission requires explicit consent for
   each report after local assembly and redaction.
10. Unattended execution remains stopped until the registered graph, semantic
    blocker register, governance checks, and current readiness/quality evidence
    all pass on one exact source revision.

## Required verification

- Task-graph tests remove each of Stories 25.3, 76.2, 76.3, and 77.2; introduce
  an unknown dependency, cycle, or applicability drift; and require rejection.
- Blocker-register tests prove an unrelated sprint blocker cannot classify a
  local row, transitive dependency paths are retained, unknown is distinct and
  non-executable, and the newly registered first local row is selectable.
- Current documentation uses Story 25.3 for the preview gate and separately
  states preview, full-GA, and retained-platform applicability.
- Current status remains `scaffolded`, with no enabled model, supported
  platform, released package, or release approval.

## Approval record

On 2026-09-06, the owner supplied the revised full-project completion prompt and
explicitly directed implementation and verification of its required
restart-readiness batch under the prompt's governance and approval requirements.
This record is limited to the corrective local work above and carries none of
the external authorities excluded in this decision.

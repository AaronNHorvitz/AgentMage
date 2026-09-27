# Decision 0088: Amendment Coverage in Work Selection

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0052, 0054, 0061 and 0081; current owner restart |
| Scope | Existing repository blocker register and work selector |

## Finding

The existing selector recognizes heading and list checkboxes, but omits all twenty
AMR package/component rows in TASKS.md. Its current result covers 1,602 earlier open
rows and zero AMR rows. The accepted amendment therefore cannot be assessed from
that result. Decision 0052 already prohibits treating unassessed work as external.
This finding does not establish that any omitted capability is ready or complete.

## Decision

Extend the existing derived register to include the AMR tables. TASKS.md remains
the only completion ledger; no second scheduler, runtime owner or grant is added.
Preserve all earlier numeric identities, dependency classifications, blockers and
Decision 0061 coding priority. Keep the old-format and AMR denominators explicit.
Bind the complete task source and amendment authorities, preserving prior inputs.

Parse only the closed package and component table forms. Refuse malformed or
duplicate AMR identities and require every currently accepted AMR identity in the
production build. Preserve the exact row bytes, location and dependency text.
Record AMR references only from the dependency column; mentions in deliverables
do not become prerequisites. Missing referenced identities remain unresolved.

AMR dependency prose can require a source contract, native boundary, exact profile
or owner artifact without requiring completion of an entire parent package.
Recognizing an identifier cannot resolve that distinction. Classify these rows as
requiring assessment, even when a referenced row is checked. Neither local-work
nor external-blocker words in a table description establish a disposition.
Retain referenced identities for inspection without presenting them as resolved
completion-gate dependencies. Future structured assessments require their own
explicit source and verification scope; this change supplies none.

Coding additions and their exact prerequisites retain precedence. Thereafter,
unassessed AMR work precedes unrelated older-plan implementation. Assess a package
and its numbered components together, in amendment order, before the next package.
Preserve the full older plan and its remaining unknown, dependency and external
rows. Neither an omitted row nor an unknown disposition supports an external-only
or completion declaration.

## Verification and limits

Exercise package/component coverage, exact source binding, duplicate/malformed
refusal, missing references, qualified and checked prerequisites, misleading
execution/blocker words, coding precedence and assessment before unrelated work.
Retain all existing selector tests. Regenerate the derived register only after the
parser and tests are complete. These are planning-tool checks, not native, model,
independent, human or release acceptance. Runtime sources and capability status
remain unchanged.

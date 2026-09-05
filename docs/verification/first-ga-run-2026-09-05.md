# First GA Unattended Run — 2026-09-05

Continuation of [`first-ga-run-2026-09-04.md`](first-ga-run-2026-09-04.md). Current Git state at
the date boundary was branch `build/agentmage-ga`, checkpoint `0a54afe9`, three commits ahead of
upstream while the sole worker was completing the Batch 151 full gate. Batch numbering continues
from the prior ledger.

## Batch 151 — Sprint 148 financial documents and matching

### Completed

- Closed 20 local Sprint 148 rows: five source-preserving financial-document kinds; seven
  field kinds with exact source hash, page/region coordinates, parser identity/version,
  confidence, and unresolved ambiguity; explainable transaction-match candidates; exact and
  semantic duplicates with preserved versions; and seven classified privacy-copy kinds covering
  encryption, minimization, retention, redaction, export, backup, deletion, and zero residue.
  Corpus cases: 6,720; silent merges, authority, hostile-file escapes, privacy residue, and
  citation loss: 0 each. Batch closures: 20. Cumulative closures: 1,816. Promotions: 0.
- Commits: `4bbc3d86` (kernel contract, corpus, tests, closures, traceability and supply chain),
  `1f466a2e` (Sprint 148 report), and `0a54afe9` (14 downstream bound artifacts). Including this
  log: 4; commits/item: 0.20. Review pins advanced: 0; complete `REVIEWED_PATHS` intersection
  across 27 changed or regenerated paths and all 20 pin-bearing gates: empty.

### Validation and blockers

- Passed Clippy with warnings denied, 4 focused Rust tests, 8 focused Python tests, the
  6,720-case AT-FDOC-001 corpus, Sprint evidence, dependency, traceability, supply-chain, Story
  1.2, and Story 3.1/Sprint 3 configuration chains. Supply-chain builds: 1; evidence regeneration
  passes: 1. Full gate: 701.671 seconds, stopping only at retained Story 6.1 Podman after every
  preceding gate passed. Self-recovery: 2 — rebuilt the traceability report after truthful TASKS
  closure changed its input, then rebuilt the Story 3.1 security map after its component reports
  changed before rebuilding the Story and Sprint aggregates in dependency order.
- Sprint 148 remains `UPSTREAM-SPRINTS-140-142-143-BLOCKED` and
  `BLOCKED_EXTERNAL(platform=native supported financial-document, OCR, archive, malware,
  encrypted-storage, backup, and export environments with representative documents,
  artifact=untouched native extraction, coordinate, parser, matching, duplicate, hostile-file,
  privacy-copy, backup, restore, deletion, reviewer, and removal evidence, action=provision exact
  native environments and representative receipt, invoice, reimbursement, statement, and tax
  documents, then run AT-FDOC-001 and RV-33 native campaigns, credential=applicable
  encrypted-storage, backup, and export destination credentials, payment=host, OCR, storage, or
  provider costs if applicable)`; `substitution_set=empty`. Full-chain blocker remains `blocked:
  host change required — run npm run -s docs:check outside the restricted filesystem sandbox with
  the current user's /run/user/1000/libpod writable`; `substitution_set=empty`.

Exact next action: Sprint 149 statistical outlier and potential-fraud indicators. Cumulative
closures: 1,816; next checkpoint: 1,825.

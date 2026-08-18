# Sprint 29 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 29 |
| Local deterministic retrieval core | Pass |
| Semantic components | Absent |
| Filesystem, network, process, or write authority | Absent |
| Sprint result | Blocked |

## Verified Locally

- Strict source-document and query contracts bind approved roots, canonical paths, dates, file
  types, authority classes, hashes, ranges, historical/denied state, and fixed budgets.
- Literal case-insensitive term and phrase matching covers title, field, tag, link, date, task,
  heading, metadata, and body fragments without regular-expression or instruction execution.
- A fixed score table prefers current handoffs, canonical Markdown, direct evidence, and recently
  verified sources. Stable path/range/digest tie-breaking is independent of input order.
- Context is byte-bounded and exact-text deduplicated while preserving the highest-ranked
  content-bound citation.
- Current contradictions, stale sources, denied sources, missing evidence, and exhausted context
  produce explicit states rather than inferred answers.
- The synthesis envelope and deterministic renderer reject evidence-state changes, invented or
  duplicate citations, uncited nonblocked answers, and proposed prose for unknown/blocked input.
- Query-injection strings remain literal, obvious secret candidates never enter results, and
  unrelated-workspace canaries remain excluded. Empty and oversized queries, invalid dates and
  limits, traversal-like paths, historical supersession, and composed/decomposed Unicode behavior
  are explicit and covered.
- The versioned three-question fixture corpus has exact expected top-one paths and citation sets.
  This is a contract fixture, not a claim of production retrieval quality.
- Canonical parsed snapshots and integrity-checked rebuilt SQLite rows now feed the same labeled
  retrieval evaluator. All three fixture questions produce the same citations, evidence states,
  and extractive rendered answers through both paths.
- Missing expected evidence is retained as an explicit citation-set blind spot. Stale indexes,
  malformed expected citations, and duplicate expected citations fail closed before an answer.
- Rebuilt-index projection reads emit content-free receipts with false source-mutation,
  external-process, and network-effect markers.
- All semantic-component flags remain false, and the strict-local audit finds no undeclared
  network path.

## Open Evidence

Sprint 28 remains blocked, and independent Sprint 29 review is absent. The retained parity path is
deterministic and extractive; it does not claim native-interface execution, model-generated
synthesis, production retrieval quality, or a release gate.

Sprint 29 therefore remains blocked despite passing its locally executable deterministic
retrieval scope. The machine-readable record is retained at
[`local-evidence-report.json`](../../artifacts/sprints/sprint-29/local-evidence-report.json).

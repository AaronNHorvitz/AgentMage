# Sprint 30 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 30 |
| Local policy/lifecycle/comparison contracts | Pass |
| Real approved semantic profiles | Absent |
| Real-hardware benefit benchmark | Absent |
| Semantic release behavior | Disabled |
| Sprint result | Blocked |

## Verified Locally

- Exact embedding and optional reranker manifests require hash-bound role-specific approval,
  approved lifecycle state, fixed local runtime, artifact/tokenizer digests, resource bounds, and
  passing origin and license policy outcomes.
- Per-workspace opt-in binds exact roots, files, fields, hashes, byte/record ceilings, storage
  protection, lifecycle disclosures, policy, and accepted benchmark evidence.
- Content-addressed records bind source path/hash/range, branch, model/reranker, tokenizer,
  chunker, schema, policy, and complete opt-in identity.
- Atomic rebuild, stale omission, deterministic fixed-point query, inspect, delete, clear, and
  rebuild controls mutate no source and expose no network contract.
- Instruction-like content remains inert. Secret-bearing, oversized, out-of-scope, undisclosed,
  duplicate, missing-vector, wrong-dimension, and cross-workspace inputs fail closed without
  changing the prior index.
- The comparison gate reports structural, lexical, semantic, and hybrid precision, recall, top-one,
  citation-set, latency, memory, and uncertainty metrics under one corpus/hardware/result identity.
- Grounding, concept-gain, code-symbol, latency, memory, or uncertainty failure retains
  deterministic-only release behavior.
- All model and vector data used by the unit suite are synthetic contract fixtures. They are not
  model-quality or release evidence.

## Open Evidence

Sprint 29 remains blocked. No real embedding or reranking artifact has completed model admission,
no local runtime has generated vectors for a representative corpus, and no real-hardware
four-mode benchmark or independent review exists. Application integration is also absent.

Sprint 30 therefore remains blocked and semantic retrieval remains disabled for release. The
machine-readable record is retained at
`artifacts/sprints/sprint-30/local-evidence-report.json` when generated from a committed revision.

# Evidence-State Assignment

## Scope

Sprint 20 adds a platform-neutral, kernel-owned classification boundary for material claims. It
does not resolve citations against live files, persist an answer ledger, chain audit receipts, or
grant authority. Those concerns remain in Sprint 21 and later integration work.

The shared contract exposes exactly four user-visible states:

| State | Required provenance | Meaning |
|---|---|---|
| `observed` | Successful authority-bound Observe receipt and its exact content-addressed sources | A deterministic tool directly returned the fact. |
| `derived` | Registered method identity, immutable version and implementation digest, and exact Observed assignment IDs | A deterministic method computed the value from observations. |
| `inferred` | Supporting citations, exact model run, complete model/runtime manifest observation, and response digest | A model interpreted evidence; the state does not imply proof. |
| `unknown_blocked` | Closed reason, stable content-free detail code, and any available evidence identities | Stronger classification is unavailable or unsafe. |

`Unknown/Blocked` has eight closed reasons: unavailable, denied, failed, conflict, stale evidence,
unsupported parsing, unverifiable data, and scope exclusion. Free-form model confidence is not a
state and cannot enter the wire contract.

## Validation Boundary

`EvidenceStateAssigner` owns one bounded task-local assignment map. An assignment is terminal and
its identity cannot be reused. It validates state-specific provenance before insertion:

- Observed accepts only `workspace_read` or `database_read` receipts in the Observe authority
  class. The receipt must be successful, tool-call bound, internally hash-valid, error-free, and
  source-identical to the claim's current subject and revision.
- Derived accepts only an exact entry in `DeterministicMethodRegistry`. Every input must already
  exist in the same task and have the Observed state; inferred or blocked inputs cannot be promoted.
- Inferred requires at least one valid citation and binds the exact model run, profile, artifact,
  tokenizer, template, codec, adapter, runtime build, platform, architecture, and response digest.
- Unknown/Blocked permits no evidence when none exists, but any supplied references must remain
  bounded, content-addressed, and uniquely identified.

All assignment and provenance contracts reject unknown fields. Identifiers, collections, text,
digests, and versions have explicit bounds. Duplicate sources, duplicate inputs, cross-task claims,
non-observe receipts, incomplete manifests, and malformed reason codes fail closed.

## Authority and Completion

Evidence assignments implement `NonAuthoritativeArtifact` as a sealed Claim Record. Offering one
as execution authority always returns the existing descriptive-authority denial. No assignment
constructor can issue or consume a grant, start a worker, access a path, contact a network, or mark
the existing completion ledger verified.

The current implementation intentionally does not attach these assignments to final rendered
answers. Sprint 21 must add current citation resolution, stale-source detection, the answer-claim
ledger, reason rendering, append-only receipt chaining, and the external keyed integrity anchor
before the evidence story can pass its complete gate.

## Traceability

| Requirement | Contract and implementation | Focused verification |
|---|---|---|
| `S-019-I01`, `AM-EVD-002` | `MaterialClaimEvidenceState`, `MaterialClaimEvidenceStateKind` | Constructs and compares exactly four state kinds. |
| `S-019-I02`, `AM-EVD-001` | `ObservedClaimProvenance`, `assign_observed` | Valid receipt, every non-success outcome, non-observe, missing-tool, tampering, source and revision cases. |
| `S-019-I03` | `DeterministicMethodIdentity`, `DeterministicMethodRegistry`, `assign_derived` | Registered, changed, missing, duplicate, and non-Observed input cases. |
| `S-019-I04` | `InferenceRuntimeProvenance`, `InferredClaimProvenance`, `assign_inferred` | Missing citation and model, manifest, runtime, and response identity mutations. |
| `S-019-I05` | `UnknownBlockedReason`, `UnknownBlockedClaimProvenance`, `assign_unknown_blocked` | Every closed reason, malformed detail, duplicate evidence, and empty-evidence cases. |
| `SR-AI-003`, `SR-AI-007` | Sealed non-authoritative assignment and closed state payload | Authority rejection and unknown-field/model-confidence injection. |
| `SR-AI-010`, `SR-AI-011` | Exact inference provenance without raw response content | Model/runtime identity mutation and reproducibility checks. |
| `SR-OPS-001` through `SR-OPS-005`, `SR-TST-010` | Source-bound local evidence report | Hashed committed sources, command-output digests, explicit blockers, and recomputed summary. |

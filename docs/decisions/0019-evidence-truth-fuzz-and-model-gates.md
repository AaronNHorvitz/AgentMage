# Decision 0019: Evidence Truth and Model Gates

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-12 |
| Scope | Historical evidence, current applicability, traceability, platform lanes, review provenance, shared evidence primitives, concurrency, and model activation truth |
| Resolves | `RM-019` through `RM-023`, the concurrency portion of `RM-024`, and `RM-025` |
| Defers | Real fuzz-engine execution in `RM-024` to separately approved manual security validation |
| Preserves | Decisions 0001 through 0018, immutable retained artifacts, strict-local operation, and fail-closed release claims |

## Context

AgentMage retains source-bound evidence from earlier implementation increments.
Many of those records remain valid statements about the exact source and policy
that they reviewed, but their generators compare them with the present working
tree and therefore label them stale or fail before their historical validity can
be established. The traceability generator has the opposite defect: it does not
inspect accepted artifacts at all and reports empty evidence paths even when a
gate names an exact acceptance-test identity.

The existing story and sprint gates also combine shared and Linux results with a
retained macOS blocker. That accurately prevented an old cross-platform claim,
but it no longer expresses the first-GA platform composition in Decision 0008.
Some generated reviews use independence language for aggregation performed by
the same repository automation. Existing fuzzing defines a strong registry,
policy, corpus, result schema, and seeded synthetic pipeline, but no real fuzz
engine currently executes a product parser. Real fuzz execution is intentionally
deferred from this stabilization phase because it must run as a separately
approved manual security-validation activity. The grant race tests additionally
place synchronization in the test harness around an otherwise exclusive mutable
issuer. Finally, the current status model correctly records no enabled model,
while roadmap and historical language can still be mistaken for an active
Gemma runtime.

## Decision

### Historical Evidence

1. A retained artifact is immutable evidence about a recorded source state. Its
   historical validity is evaluated only against its recorded commit, tree,
   artifact bytes, owned inputs, policy identities, signatures, and bounded
   environment fields.
2. A later policy, source, task, or documentation change cannot invalidate an
   otherwise replayable historical record. It can change only that record's
   separately computed current applicability.
3. A record lacking enough source identity to replay is classified
   `unverifiable-legacy`; it is neither silently repaired nor treated as current.
4. Missing Git objects, changed historical blobs, invalid signatures, unsafe
   paths, and malformed bindings fail historical validation with content-free
   diagnostics.

### Current Applicability and Supersession

1. `evidence/catalog.json` is the current, versioned catalog of accepted evidence
   bindings. Each record names one exact artifact, its immutable historical
   identity, exact requirement or acceptance-test claims, owned inputs,
   transitive evidence dependencies, disposition, platform lanes, review
   provenance when applicable, and any explicit supersession relationship.
2. `evidence/current/applicability-report.json` is a generated current view. It
   never replaces or modifies a retained artifact.
3. Current applicability is one of `current`, `stale`, `blocked`, `superseded`,
   `historical-only`, or `rejected`. Historical validity is reported separately.
   Traceability uses `absent` when no valid catalog record exists and `produced`
   only for valid evidence that is intentionally not evaluated as a current
   completion claim.
4. A changed owned input makes the owning record stale. Staleness propagates
   through the catalog dependency graph. An unrelated change has no effect.
5. Supersession is an explicit acyclic edge. The old and new artifact bytes stay
   immutable, and only the new eligible record may be selected as current.
   Duplicate live claims for one exact artifact or ambiguous current claims for
   one requirement fail closed.

### Artifact-Discovering Traceability

1. Traceability accepts evidence only through the validated catalog. Finding an
   identifier in arbitrary JSON or prose does not create a binding.
2. A catalog claim must name an existing registry requirement, and the artifact
   must contain that exact identifier in an allowlisted structured field. A
   wrong requirement, forged path, hash mismatch, unsafe path, missing artifact,
   duplicate claim, or invalid historical binding is excluded and reported.
3. The traceability report records every accepted artifact path and its separate
   historical-validity and current-applicability states. Evidence cannot promote
   the planned lifecycle or product-completion status in the requirement
   registry by itself.
4. Required current-release evidence with no accepted record is `absent` and
   `missing-required`. Evidence outside the selected release remains `absent`
   and `expected-outside-release`; those conditions are never conflated.

### Platform Lanes

1. Gate results are recorded independently for `shared`, `fedora-x86_64`,
   `ubuntu-x86_64`, `windows-x86_64`, and `macos-arm64-retained`.
2. A named milestone lists its exact required lanes. A lane may be `pass`,
   `block`, `unsupported`, or `not-applicable`; skipped, stale, missing, or
   substituted evidence cannot become `pass`.
3. The first-GA composition requires shared, Fedora, Ubuntu, and Windows lanes.
   Retained macOS work has its own milestone and does not erase Linux progress or
   become satisfied by Linux evidence.
4. A composed gate reports every lane, every blocker, and the reason a lane was
   included or excluded. A missing required lane blocks only milestones that
   require it.

### Review Provenance

1. Review provenance uses the closed classes `deterministic-self-check`,
   `automated-aggregation`, `independent-automated-analysis`, `native-execution`,
   `independent-human-review`, and `external-review`.
2. Repository automation that builds or checks its own report is never a human
   review. Aggregating separately generated inputs is not independent analysis
   unless the record identifies a distinct implementation, process identity,
   inputs, and execution.
3. A blocked review may retain nonzero findings, owners, remediation state, and
   a later re-review link. Findings are never forced to zero to fit a passing
   shape.
4. A control requiring human or external review remains unsatisfied until a
   provenance-valid record of that exact class exists.

### Shared Evidence Primitives

1. A small standard-library Python module owns canonical JSON, SHA-256 hashing,
   bounded regular-file reads, repository-relative path validation, atomic
   writes, Git source identity, historical blob replay, artifact discovery,
   deterministic report rendering, and redacted diagnostics.
2. New Phase 10 evidence tools and the traceability generator use these
   primitives. Existing historical generators are not bulk rewritten, and their
   retained output is never reformatted as a side effect of consolidation.
3. Reads reject symbolic links, path escape, non-regular files, and oversized
   input. Writes use a same-directory temporary regular file, synchronization,
   fixed mode, atomic replacement, and cleanup on failure.

### Concurrency and Deferred Manual Fuzzing

1. Phase 10 does not add or execute a real fuzz engine, and it makes no real
   fuzzing, sanitizer, or coverage claim. Existing synthetic pipeline artifacts
   retain their explicit synthetic provenance.
2. Real product-boundary fuzzing remains an open `RM-024` security-validation
   task. It requires separate user approval and manual execution outside the
   ordinary autonomous development loop before any result may be accepted.
3. A future manual run must bind exact toolchain, harness, product-source,
   corpus, dictionary, sanitizer, resource-limit, and result identities. It must
   not replace or relabel historical synthetic evidence.
4. A product-owned synchronized grant-admission boundary performs the contested
   consume or cancellation transition. Concurrency tests launch unsynchronized
   callers behind a start barrier and do not wrap the product operation in a
   test-owned mutex.
5. One valid single-use grant admits at most one consume. Concurrent cancellation
   and consumption produce one terminal state, no replay, and no second effect.

### Model Catalog and Runtime Truth

1. The current baseline has zero enabled model profiles. Both evaluated Gemma
   candidates remain visible only as rejected and disabled catalog records.
2. The VS Code `secure-local-read` provider is a deterministic product workflow,
   not a Gemma profile and not model inference. It must not be labeled with a
   rejected model identity.
3. A future model can enter the picker only through one admitted profile that
   binds the model artifact, tokenizer, conversion or quantization, runtime,
   platform, license, lineage, and exact hashes. Mutation of any bound item
   disables activation until a new admission succeeds.
4. Rejected, blocked, missing, mutated, unsupported, or offline-unavailable
   selections stop visibly. Automatic fallback or substitution is prohibited.

## Verification

Phase 10 verification must include:

- historical replay, unavailable source, later-policy change, unrelated change,
  direct and transitive staleness, signature mutation, explicit supersession,
  and supersession-cycle tests;
- exact `AT-AUTH-001` discovery plus duplicate, wrong-ID, forged-path,
  missing-artifact, rejected, stale, and outside-release tests;
- mixed platform lanes, unsupported and retained lanes, milestone composition,
  and evidence-substitution denial;
- every review class, self-review mislabeling, missing reviewer, reused process,
  blocked findings, re-review, and human-review gate denial;
- bounded-read, symbolic-link, path-escape, oversize, atomic-failure,
  deterministic-order, and historical-output compatibility tests;
- explicit denial of real fuzzing, sanitizer, or coverage claims from synthetic
  records, with the deferred manual security-validation task remaining open;
- duplicate consume and cancellation races with product-owned synchronization;
  and
- rejected, blocked, valid-admission, artifact/runtime mutation, unsupported
  platform, fallback refusal, disabled picker, and offline-runtime model tests.

## Consequences

- Retained evidence can remain historically trustworthy without pretending to
  describe the current tree.
- Current gates become smaller and more candid: they identify exactly what is
  current, stale, blocked, absent, or outside a release.
- macOS no longer obscures Linux work and cannot be satisfied by it.
- Review labels describe who or what actually performed the review.
- Real product-boundary fuzzing remains a visible, separately approved manual
  security-validation task and is not represented as Phase 10 completion.
- AgentMage continues to expose no local model until a complete model and
  runtime admission passes.

## Approval Record

The user approved entry into Phase 10 after accepting Decision 0018 and the
Phase 9 local commit on 2026-08-11. On 2026-08-12, the user explicitly deferred
real fuzz execution to a separate manual security-validation task, approved
completion of the amended phase, and authorized committing and pushing its
verified candidate. This acceptance does not authorize entry into Phase 11.

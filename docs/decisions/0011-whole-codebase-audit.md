# Decision 0011: Whole-Codebase Audit as a First-GA Capability

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-11 |
| Scope | Comprehensive, read-only, resumable, evidence-based repository audit |
| Amends | Decision 0010 and the implementation scope of Sprints 157-166 |

## Context

AgentMage already plans repository reading, codebase understanding, search, testing, and coding
assistance. Those capabilities do not by themselves prove that a repository too large for one
model context has been comprehensively evaluated. A sequence of independent chunks can omit files,
lose contradictions, compound summary errors, or mistake retrieval relevance for architectural
coverage.

A serious codebase audit must know what it did and did not inspect, preserve exact source evidence,
reconcile conclusions across modules, resume after interruption, invalidate stale conclusions, and
prove that a read-only request did not modify the repository. These controls belong in product
architecture rather than in a user prompt.

## Decision

1. AgentMage adds `CODEBASE-AUDIT.md` as the normative first-GA whole-codebase audit architecture.
2. The repository index and evidence ledger are persistent local project memory. Model context is
   bounded working memory and is never treated as complete repository state.
3. Every audit starts from an exact repository, revision, working-tree, scope, parser, model,
   policy, command, resource, retention, and output identity.
4. Every in-scope path receives an explicit terminal disposition. Generated, vendored, binary,
   ignored, untracked, linked, archived, submodule, worktree, large, inaccessible, and unsupported
   content cannot disappear from coverage accounting.
5. Deterministic parsers, language services, build metadata, source-control records, and exact
   search construct the canonical structural index. Embeddings may rank retrieval but are never the
   source of truth for identity, coverage, dependency, authority, or evidence.
6. Semantic analysis operates on coherent bounded work packets and creates source-pinned evidence
   cards. Model observations remain provisional until deterministic references and reconciliation
   support them.
7. Cross-module reconciliation, contradiction retention, reverse-dependency invalidation, and
   risk-directed review are mandatory before a report can claim whole-codebase coverage.
8. Read-only behavior is enforced outside the model. The canonical repository receives no write
   grant; commands that may write run only in a disposable copy-on-write workspace; before-and-after
   identity proves whether the audit changed anything.
9. Secret detection and classification occur before model exposure. Raw sensitive values never
   enter model context, evidence cards, findings, checkpoints, reports, logs, diagnostics, or
   exports.
10. Checkpoints bind exact source and analysis identities. Resume revalidates them, and changed
    inputs invalidate all transitively dependent records before reuse.
11. Every finding includes immutable evidence, severity, confidence, impact, counterevidence,
    uncertainty, and recommendation. Every final report publishes complete coverage and gap totals.
12. The capability is implemented through added stories in Sprints 157, 159, 161, 163, and 165 and
    independently qualified in Sprint 166. Existing sprint identities and the Sprint 166 final GA
    gate remain stable.
13. `AM-GAD-004` and `AT-GA-004` extend, rather than erase, the Decision 0010 release evidence.
    Sprint 166 cannot close `G-GA` without whole-codebase audit evidence.

## Consequences

- Repository size is a throughput and storage problem rather than a reason to fabricate a larger
  model memory.
- Audits can take substantial time, pause, resume, and update incrementally without losing exact
  coverage state.
- The product can distinguish a quick or targeted review from a comprehensive audit truthfully.
- Builds and tests can inform a read-only audit without writing to the canonical source tree.
- Architectural drift and mid-development pivots are reported as observed structure or supported
  historical conclusions according to the evidence actually included.
- Smaller approved local models can contribute bounded analysis while deterministic indexing and
  reconciliation carry repository-wide state.

## Verification

- Canonical documents link `CODEBASE-AUDIT.md` and preserve the same first-GA scope.
- The inventory appends stable product requirements, acceptance tests, and construction checks.
- `TASKS.md` adds detailed stories without renumbering or deleting any sprint.
- `SECURITY-REVIEW.md` adds repository-census, read-only, secret, parser, checkpoint,
  reconciliation, report, resource, and removal controls and reviewer protocols.
- `RUNTIME-BOUNDARIES.md` declares every audit worker, IPC edge, path authority, scratch store,
  checkpoint, model packet, and cleanup owner.
- Generated registry, policy, traceability, Markdown, Mermaid, link, identifier, and additions-only
  checks pass after deliberate review and baseline extension.

# Decision 0053: Owner-Delegated Linux Desktop Demo and Governance Correction

| Field | Value |
|---|---|
| Status | Accepted owner-delegated milestone decision |
| Date | 2026-09-15 |
| Scope | Locally runnable Fedora Kinoite document-question-answering demo; decision identity and G1/G2 planning correction |
| Authority | Repository owner's explicit implementation and governance delegation on 2026-09-15 |
| Preserves | Business Source License Decision 0051, historical evidence, stable requirements, production roadmap and release gates, whole-file evidence bindings |

## Owner direction

The owner instructed: "This instruction prioritizes a usable Linux desktop demo
over completing the entire production roadmap. It supersedes conflicting
repository-level sequencing requirements for this milestone only." The owner
also explicitly authorized resolving the duplicate Decision 0051 identity and
G1/G2 planning registration mismatch, ordinary architecture and model choices,
local implementation and testing, and pushing a dedicated demo branch.

## Decision

1. Business Source License Decision **0051** retains its filename, identity,
   license parameters, original acceptance and historical meaning unchanged.
   The restart-readiness correction originally also numbered 0051 is assigned
   **0052**, at `0052-decision-0048-restart-readiness-correction.md`. Current
   references and validators use 0052. Historical immutable reports retain
   their recorded names and hashes; their restart-readiness 0051 references
   resolve through the correction note in 0052. This is an authorized identity
   correction, not a revision of either decision's historical substance.
2. Accept the owner's 2026-09-08 G1/G2 normative additions through the explicit
   **post-0053** planning snapshot. Preserve the historical post-0040 snapshot
   with 31 statements; the accepted current snapshot has **53 statements** and
   **294 stable requirements**, without new stable IDs, release epics or sprints.
   Record exact appended/superseded statement hashes and requirement mappings
   rather than replacing historical snapshots or weakening comparison checks.
   The registration's identity blocker is resolved by this owner direction;
   served-context contracts, native qualification, and automatic compaction
   remain separate incomplete implementation gates.
3. Execute a short dependency-ordered Linux demo checklist independently of
   production Windows packaging, enterprise features, and full-GA sequencing.
   Prefer the existing host/kernel application path and a maintainable loopback
   browser shell if practical. Demo evidence is not production release evidence
   and does not close unrelated requirements or promote rejected model profiles.
4. Demonstrate selected-folder read-only ingestion for explicit supported
   formats, truthful input dispositions, real local-model answers with actual
   source citations, bounded follow-ups, cancellation, error recovery, and
   stop/restart. Use only synthetic documents and repository fixtures in tests.
   Prevent traversal/symlink escape and treat document content as untrusted data.
5. Bind application and model endpoints to loopback and protect privileged
   browser endpoints against unintended access. Application inference is local
   only, with no cloud fallback. Inspect hardware, existing models and rejected
   evaluations before selecting an exact documented model/runtime configuration.
   Public trustworthy downloads are authorized after provenance, terms, disk
   and fit checks. No purchase, access bypass, private-document upload, system
   security changes, reboot, or destructive system change is authorized.
6. Enforce actual input/output context budgets. Until securely implemented and
   verified, automatic compaction is disabled with a clear new-conversation
   workflow. Preserve existing application-owned storage protections and make
   no unverified encryption claim.
7. Batch source changes before supply-chain and affected evidence regeneration.
   Do not remove hash inputs, disable checks, or synthesize passing reports.
   Maintain `docs/DEMO-PROGRESS.md`, document exact launch/stop/smoke commands in
   `docs/LOCAL-TESTING.md`, and commit/push coherent inspected milestones.

## Acceptance and reporting

The demo requires actual on-machine build/launch, automated interface use, real
model inference through AgentMage, known-fact multi-document grounding/citation
checks and honest insufficient-evidence answers. Also verify input boundaries,
unsupported inputs, overflow, cancellation, model-unavailable recovery, scoped
external-network denial without host VPN/firewall changes, and a successful
interaction after stop/restart. Record actual latency/resource observations and
relevant regression results. Screenshots, mocks, and standalone model-server
responses cannot substitute for the application workflow.

Production `lifecycle_status`, `verification_status`, and release qualification
remain accurate and incomplete unless independently demonstrated. This decision
does not declare the demo complete; the exact acceptance evidence and final
branch/commit determine its result. An unavoidable external prerequisite must be
reported precisely with preserved work and the minimum owner action.

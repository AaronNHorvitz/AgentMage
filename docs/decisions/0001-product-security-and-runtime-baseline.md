# Decision 0001: Product Security and Runtime Baseline

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-10 |
| Scope | Planning baseline additions and clarifications before product implementation |
| Supersedes | Any implication that the Docker or native Linux adapter is the sole v0.1 path |
| Superseded in part by | [Decision 0008](./0008-first-ga-delivery-system-and-windows.md), which reclassifies v0.x as internal milestones and appends the first-GA delivery/Windows scope |

## Context

The initial planning baseline established strong product-security controls but left several required artifacts and producing tasks implicit. It also described Docker Model Runner and native `llama.cpp` inconsistently across documents. The baseline is now version-controlled, so corrective work must be recorded and traceable instead of silently rewriting accepted identifiers.

An external planning audit remains outside this repository. AgentMage incorporates only independently evaluated product findings; no external remediation text becomes project authority by being copied into the backlog.

## Decision

1. AgentMage is distributed under the Apache License 2.0.
2. `MODEL-PROVENANCE-POLICY.md` defines model-origin, lineage, artifact, runtime, and admission evidence.
3. Gemma 4 E4B remains the initial candidate. Gemma 4 12B Unified is the named, disabled fallback candidate if E4B fails a mandatory quality or tool-calling threshold.
4. Native `llama.cpp` and Docker Model Runner implement the same `LocalModelRuntime` contract and use the same admitted Gemma profile.
5. Native `llama.cpp` is the Fedora and Ubuntu security reference. Docker Model Runner is a supported compatibility adapter whose unauthenticated local API, Docker privilege model, socket exposure, and zero-egress behavior require additional gates.
6. A model or adapter is enabled only after its own evidence passes. AgentMage never switches adapters or models silently.
7. `SECURITY.md` governs public vulnerability reporting, supported versions, signed manual patch delivery, local emergency disablement, and end of support without adding runtime update checks.
8. `RUNTIME-BOUNDARIES.md` records privileges, processes, sockets, lifecycle, trust boundaries, and classified data flows.
9. v0.1 receives explicit producing stories for continuous fuzzing, patch and vulnerability response, incident tabletop exercises, accessibility, native-Chat diagnostics, and handoff disclosure warnings.
10. Each `RV-*` protocol has an owning sprint for first execution. The release sprint re-runs proven protocols and assembles evidence; it does not discover every control for the first time.
11. Documentation validation runs in continuous integration for Markdown, Mermaid, links, secret patterns, stable identifiers, and cross-document facts.
12. Existing stable identifiers remain unchanged. New stories, tasks, artifacts, and criteria are appended; wording corrections that narrow risk or remove contradiction cite this decision.

## Consequences

- Docker Model Runner can match a Docker-based Gemma development environment without becoming a hidden dependency or weakening the native security reference.
- The Docker profile may remain blocked while the native profile passes, but v0.1 cannot advertise Docker support until that profile passes. A later scope decision may defer the adapter without weakening or misrepresenting the native release.
- Model admission work moves before implementation depends on a model assumption.
- Product release work grows, but final verification becomes incremental and reproducible.
- New policy documents remain subordinate to the existing product, inventory, security, implementation, and task authorities.

## Verification

- Cross-document validation finds no unresolved model, runtime, platform, policy, or protocol contradiction.
- Every new policy is linked from README and its governing documents.
- Every new story has numbered tasks, Given/When/Then criteria, sprint criteria, and mapped `SR-*`/`RV-*` evidence.
- The documentation workflow passes from a clean checkout.

# Decision 0008: First-GA Delivery System and Windows

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-11 |
| Scope | First supported public release, connected delivery capabilities, and supported platforms |
| Supersedes | The prior public-release meaning of v0.1; the deferral of Windows 11; the placement of all hosted writes after the first supported release; the rule that final product closure occurs at Sprint 102 |

## Context

The original roadmap deliberately began with a very small read-only local assistant. That remains the correct implementation order, but it is no longer the intended boundary of the first supported public release. AgentMage is now intended to become a local-first software-delivery control plane that can understand and safely operate across source control, work planning, continuous integration, artifacts, deployments, infrastructure, observability, incidents, security findings, service catalogs, and releases.

The product must also support Windows 11 in its first supported public release. Apple Silicon work remains valuable and retained, but unavailable Mac hardware must not prevent the Linux and Windows first-GA program from completing.

Provider feature sets are not identical and change over time. A truthful claim of complete integration therefore needs a versioned capability matrix, not an unbounded claim that every endpoint or administrative operation is supported.

## Decision

1. v0.1 through v0.7 and the inherited v1+ roadmap are **internal capability milestones**. They are not supported public production releases.
2. The first supported public release is **AgentMage v1.0 GA**. It cannot ship until the new delivery-system and Windows gates appended after Sprint 102 are complete.
3. Fedora and Ubuntu are first-GA reference platforms. Windows 11 is a first-GA supported platform with its own package, runtime boundary, path contract, secret-store integration, process confinement, clean-install evidence, and release manifest.
4. Apple Silicon macOS remains a retained post-GA platform lane. Existing Mac requirements and evidence are preserved, but Mac completion is no longer a dependency of v1.0 GA. No Linux or Windows evidence may be represented as Mac evidence.
5. Native Visual Studio Code Chat remains the first and required user interface. LaTeX-style inline and display mathematics in Markdown are required authoring and rendering capabilities.
6. GitHub.com and user-approved GitHub Enterprise Server instances receive full repository, issue, pull-request, review, check, workflow, release, commit, and approval-gated push capability within the published matrix.
7. The delivery architecture is provider-neutral. Jira, Azure DevOps, GitLab, Jenkins, artifact registries, deployment systems, infrastructure tools, observability systems, incident systems, security tools, and service catalogs use one versioned adapter contract and conformance suite.
8. Every adapter publishes host types, tested versions, object coverage, read operations, write operations, executable operations, unsupported operations, required scopes, event modes, limits, and degradation behavior. “Full” means complete support for that published matrix and nothing broader.
9. Read, local write, remote write, execute, deploy, secrets, and administration are independent capability classes. An approval or credential for one class never implies another.
10. Every external mutation requires a current exact preview, fresh remote preconditions, a single-use grant, an idempotency key where the provider supports one or an equivalent reconciliation strategy, verified postconditions, and one immutable receipt.
11. CI execution, deployment, infrastructure application, secret changes, policy administration, branch or repository administration, force operations, destructive operations, and production promotion each require separate policy and approval gates. They cannot be hidden inside a generic connector write.
12. OpenTelemetry is the provider-neutral observability foundation. Vendor adapters consume or correlate the same typed telemetry and delivery graph rather than defining separate product truths.
13. The strict-local product remains complete and testable when every connected capability pack is absent. Connected adapters cannot become startup dependencies for local workspace work.
14. Sprints 101 and 102 remain historically stable inherited-scope checkpoints. Their former final-product meaning is superseded. The final v1.0 decision occurs only at the appended `G-GA` gate.
15. Extreme verification is release-blocking: provider contract mutation, hostile content, credential confusion, cross-tenant access, webhook replay, network partition, rate limiting, version skew, duplicate delivery, partial effect, crash recovery, rollback, resource exhaustion, and clean-platform testing all require current raw evidence.

## Consequences

- The roadmap grows substantially, but implementation still proceeds from the existing local kernel and authority foundations.
- Completed work remains valid unless a changed contract makes its evidence stale; it is never erased or represented as proof of a new provider or platform.
- Windows work becomes a release dependency. Mac tasks retain their identities and `BLOCKED-MACOS` truth state but move outside the v1.0 GA dependency chain.
- Connected capability packs may be removed without damaging the strict-local core or canonical local state.
- Provider support can expand without adding provider conditionals to the kernel.
- A release cannot claim complete delivery support unless its exact provider/version/capability matrix and conformance evidence ship with it.

## Verification

- Canonical documents identify v0.x as internal milestones and v1.0 as the first supported public release.
- The first-GA matrix identifies Fedora, Ubuntu, and Windows 11 as required and Apple Silicon macOS as retained post-GA work.
- The inventory contains additive stable requirements and tests for the delivery graph, adapter contract, connected systems, Windows boundary, and extreme verification.
- `TASKS.md` preserves Sprints 0-102 and appends delivery, Windows, and final-GA gates.
- Documentation validation checks the new decision, required architecture documents, sprint count, platform boundary, capability classes, and first-GA language.

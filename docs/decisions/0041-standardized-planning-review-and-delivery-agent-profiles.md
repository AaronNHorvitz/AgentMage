# Decision 0041: Standardized Planning, Review, and Delivery Agent Profiles

| Field | Value |
|---|---|
| Status | Accepted additive architecture and planning refinement |
| Date | 2026-08-17 |
| Scope | Standardized planning, issue, bug, review, CI/CD, release, operations, and maintenance role profiles |
| Adds | Profile identities `AG-01` through `AG-49`, one architecture reference, and Stories 92.2, 95.2, 107.2, and 125.2 |
| Preserves | All existing requirements, sprint identities, completed work, shared-runtime boundaries, provider gates, and release blockers |
| Does not authorize | Immediate implementation, out-of-order work, agent-owned credentials or policy, autonomous external effects, self-approval, automatic merge, release, deployment, or a support claim |

## Context

AgentMage already plans bounded agent definitions, synthetic validation, child
authority, multi-agent coordination, GitHub and work-management adapters, CI/CD,
release, incident, security, and evidence capabilities. Those plans describe the
enabling mechanisms but do not provide one canonical catalog of the specialist
roles used to turn ideas, issues, bugs, pull requests, builds, releases, and
incidents into reviewable work.

Without a catalog, later implementation could create overlapping agents with
private model loops, provider clients, memory, permissions, or completion rules.
It could also model deterministic controls such as credential delivery, signing,
policy decisions, or deployment execution as language-model judgment. Both
outcomes would conflict with the existing kernel and shared-runtime architecture.

The requested profiles are a decomposition of already accepted capabilities
under `CR-P2-MAG`, `AM-GHE-001`, `AM-WRK-001`, `AM-CIC-001`, `AM-REL-001`,
`AM-INC-001`, `AM-SEC-003`, and their existing gates. They do not create a new
provider, authority class, release, or capability family.

## Decision

1. Standardize `AG-01` through `AG-49` as versioned declarative role profiles.
   A profile describes purpose, accepted work, prohibited work, model needs,
   requested tools, maximum authority, inputs, outputs, budgets, evidence,
   completion, and stop conditions. A profile has no authority by itself.
2. Every role submits a bounded work packet through the shared caller-neutral
   runtime. No role receives a private model loop, tool router, permission
   engine, journal, artifact store, session store, credential path, or provider
   client.
3. Deterministic workflows own ordering, dependencies, leases, retries,
   idempotency, gates, cancellation, and side-effect state. Agent roles supply
   bounded synthesis or judgment only where deterministic processing is
   insufficient.
4. Policy and approval, credential brokerage, signing and key custody, evidence
   and provenance retention, artifact verification, and merge/deployment
   actuation remain deterministic kernel or platform services. They are not
   agent roles and cannot be replaced by model conclusions.
5. Read-only analysis and local drafts may operate at high configured autonomy.
   Controlled local writes require exact workspace, worktree, preimage, and
   grant boundaries. Remote writes, CI execution, merge, release, deployment,
   rollback, notification, and administrative actions remain separate provider
   effects under the existing Autonomy and capability-class policies.
6. Review profiles execute independently from implementation profiles. Their
   context records the reviewed source and evidence rather than inheriting an
   implementer's conclusion. A review coordinator preserves dissent, severity,
   evidence, and unresolved findings instead of averaging them into approval.
7. Story 92.2 owns the declarative profile catalog and compatibility templates.
   Story 95.2 owns deterministic local coordination and proposal synthesis over
   enabled profiles. Story 107.2 owns provider-backed issue, bug, pull-request,
   Agile-planning, and task-reconciliation workflows after the GitHub and work-
   management adapters are available. Story 125.2 owns all-profile lifecycle
   conformance after the complete delivery and operations adapter matrix exists.
8. `M-HARNESS-MVP` and its runtime-hardening dependencies remain earlier work.
   This decision records future composition and does not move profile execution
   ahead of the single-session coding harness.
9. Profile and workflow tests cover definition mutation, authority narrowing,
   role substitution, hidden effects, prompt injection, stale evidence,
   duplicate events, cancellation, disagreement preservation, provider identity
   confusion, uncertain results, and false completion.
10. This decision adds no stable `AM-*`, `AT-*`, or `CR-*` identity and no sprint.
    The role catalog and appended stories refine existing accepted requirements,
    so the 241-requirement and 169-sprint planning baselines remain unchanged.

## Consequences

- Issue authoring, Agile planning, bug handling, pull-request review, CI
  remediation, release, and operations gain named reusable roles without
  multiplying execution engines.
- GitHub, Jira, and Azure Boards remain provider adapters rather than agent-owned
  integrations.
- A role can be disabled, replaced, or evaluated without changing the runtime or
  granting a new capability.
- The roadmap becomes larger at the story and task level, but the implementation
  dependency order and release identities do not change.
- Current product truth remains pre-alpha with no enabled end-user role catalog,
  provider-backed planning workflow, or autonomous CI/CD agent team.

## Approval Record

On 2026-08-17, the user explicitly approved adding the complete standardized
planning, issue, bug, pull-request review, CI/CD, release, operations, and
maintenance role catalog to the architecture and plan.

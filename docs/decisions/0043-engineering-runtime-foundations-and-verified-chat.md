# Decision 0043: Engineering Runtime Foundations and Verified Chat

| Field | Value |
|---|---|
| Status | Accepted additive architecture and planning refinement |
| Date | 2026-08-22 |
| Scope | Rust-owned Engineering Runtime, Verified Chat, native Chat compatibility, observability, capability registry, deterministic completion, and multi-agent extension points |
| Adds | Normative Engineering Runtime and Capability Registry specifications, one canonical Verified Chat architecture, new stable requirements and tests, Foundational Runtime Epic F3, and dependency-ordered work in existing sprints |
| Preserves | Decisions 0012, 0027, 0041, and 0042; 17 release epics; 169 sprint identities; stable identifiers; completed work and evidence; current status truth; native tool registration; authenticated IPC; and additions-only history |
| Does not authorize | Broad production implementation, private VS Code APIs, a TypeScript policy kernel, model-owned tools or completion, ambient workspace access, automatic external effects, unsupported platform claims, or release closure |

## Context

AgentMage already has typed Rust contracts, deterministic grants, a model-neutral
candidate boundary, runtime events, durable artifacts, recovery primitives,
native tools, platform adapters, and thin clients. Decision 0042 made universal
artifact ingestion and verified workflow execution shared runtime concerns.
Those foundations do not yet define the complete engineering harness expected
by a user who needs persistent work, exact tool observations, inspectable
execution, reliable chat, reusable capabilities, and later multi-agent teams.

The native VS Code Language Model Chat Provider is useful compatibility, but it
does not control every attachment and lifecycle transition needed to promise
AgentMage's strongest guarantees. A canonical AgentMage-owned chat surface is
therefore required without turning the extension or webview into a second
kernel.

## Decision

1. `ENGINEERING-RUNTIME.md` is the normative subsystem authority for the
   Engineering Runtime. It composes Decisions 0041 and 0042 and owns the common
   lifecycle from admitted input through verified terminal result.
2. The Rust host remains the only policy, authority, persistence, tool,
   verification, and completion engine. Models are untrusted planners. The
   TypeScript extension is a transport and presentation adapter. The webview is
   disposable and reconstructs state from authenticated host events.
3. AgentMage Verified Chat is the canonical reliable VS Code experience. Its
   Ask, Plan, and Agent modes differ only in allowed workflow and effect policy;
   none grants authority by interface choice. Stable VS Code APIs and native
   editor, diff, terminal, test, diagnostic, Git, and notebook surfaces are
   reused where applicable.
4. Native `@agentmage` Chat Participant and Language Model Chat Provider paths
   remain compatibility surfaces. They must disclose unresolved references,
   normalized-part loss, lifecycle limitations, and version support. They may
   never claim the stronger Verified Chat guarantee without matching evidence.
5. Every admitted tool call produces exactly one closed `ToolObservation`.
   Large output is content-addressed and range-readable. A missing or uncertain
   observation prevents workflow advancement and completion.
6. Persistent tasks belong to a host supervisor, not a UI view or model turn.
   Durable state, ceilings, reconciliation, cancellation, pause, resume, and
   terminal diagnostics remain deterministic and verifier-owned.
7. `ENGINEERING-CAPABILITY-REGISTRY.md` defines versioned executable capability
   manifests. A manifest can request but cannot mint authority. Prompt-only
   descriptions cannot become enabled capabilities.
8. Multi-agent execution is an extension of the proven single-agent runtime.
   It may begin only after context, workflow, observation, persistence,
   verification, and resource gates pass. Each pod uses bounded authority,
   leases, budgets, an isolated worktree where applicable, and serialized
   integration.
9. The roadmap uses a hybrid additive mechanism: Foundational Runtime Epics F3
   and F4 describe cross-cutting outcomes, while implementation is assigned to
   new stories inside existing incomplete sprints. No release epic or sprint is
   inserted or renumbered.
10. Schemas and conformance fixtures may be added in this planning pass. They
    are design evidence only and do not promote any component to implemented,
    integrated, supported, or shipped.

## Consequences

- AgentMage becomes a universal engineering harness in target architecture,
  not merely a model endpoint or chat provider.
- Verified Chat can provide complete runtime state without depending on private
  VS Code internals.
- Native Chat remains available where its documented capability envelope is
  sufficient.
- Capability and agent catalogs share one runtime instead of creating private
  loops, credentials, dispatchers, or completion rules.
- Current product truth remains pre-alpha: Verified Chat, the complete
  Engineering Runtime, the Capability Registry, and multi-agent execution are
  planned or designed, not operational product claims.

## Approval Record

On 2026-08-22, the user explicitly instructed AgentMage to execute the complete
Engineering Runtime governance, architecture, requirements, security,
implementation-planning, task, registry, validation, and audit assignment while
preserving additions-only governance and current status truth.

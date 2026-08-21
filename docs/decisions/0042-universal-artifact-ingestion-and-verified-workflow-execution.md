# Decision 0042: Universal Artifact Ingestion and Verified Workflow Execution

| Field | Value |
|---|---|
| Status | Accepted additive architecture and planning refinement |
| Date | 2026-08-21 |
| Scope | Universal artifact ingestion and context preparation; verified workflow execution and recovery |
| Adds | Two cross-cutting foundational runtime epics, one architecture specification, and dependency-ordered additions to existing owning stories and sprints |
| Preserves | All stable requirements, 17 release epics, 169 sprint identities, completed work and evidence, native tool registrations, platform IPC decisions, release blockers, and additions-only history |
| Does not authorize | Production implementation in this decision, private Visual Studio Code API use, silent attachment recovery claims, a second execution engine, automatic destructive retry, unsupported model or platform claims, or release closure |

## Context

AgentMage already contains or plans a candidate-neutral model boundary, typed
agent states, verifier-only completion, a common tool registry, capability
grants, an encrypted operational store, an append-only runtime event journal,
content-addressed runtime artifacts, restart reconciliation, a reusable runtime
coordinator, native Visual Studio Code and command-line clients, and optional
later MCP integration. Those foundations must be extended rather than replaced.

Two remaining product gaps cut across those existing boundaries:

1. The model cannot reliably reason over pasted text, files, office documents,
   logs, and other attachments unless accessible bytes are resolved, extracted,
   classified, budgeted, and made retrievable before inference.
2. A model proposal is not proof that a tool ran or that a workflow completed.
   Runtime-owned preflight, execution evidence, verification, retry policy,
   recovery, and terminal diagnostics are required for dependable operation.

Both gaps are foundational services used by later capability packs. Treating
them as optional end-stage features would cause each shell, model adapter, or
capability pack to invent incompatible ingestion and execution behavior.

## Decision

### 1. Two cross-cutting foundational runtime epics

AgentMage establishes two first-class cross-cutting runtime epics:

- **FRE-INGEST - Universal Artifact Ingestion and Context Preparation** owns
  accessible-reference resolution, artifact admission, type detection,
  extraction, canonicalization, provenance, caching, chunking, indexing,
  retrieval, context budgeting, manifests, and format adapters.
- **FRE-WORKFLOW - Verified Workflow Execution and Recovery** owns persisted
  workflow plans and transitions, tool-call validation, deterministic
  preflight, executor-authored evidence, postcondition verification,
  side-effect and retry policy, approvals, no-progress detection, crash-safe
  resume, exactly-once safeguards, and terminal diagnostics.

These are first-class foundational runtime epics, not additional release
increments. Their `FRE-*` identities and count are validated separately from
the 17 numeric release epics. Their work is distributed into existing
dependency-owning sprints; the accepted 17 release-epic and 169-sprint
identities remain unchanged.

### 2. Existing Rust boundaries remain authoritative

The implementation extends the existing Rust workspace:

- `kernel/contracts` owns versioned, path-free artifact, context, workflow,
  evidence, verification, retry, and diagnostic contracts.
- `kernel/engine` owns artifact admission and orchestration, context budgeting,
  workflow supervision, policy, persistence, verification, recovery, and
  evaluation integration.
- `capabilities/knowledge` owns format-specific extraction implementations
  behind contracts that carry no policy or path authority.
- `platforms/*` owns restricted file handles, process execution, operating-
  system storage protection, sandboxing, and platform-specific IPC.
- `shells/host` owns the installed Rust host process and caller-neutral runtime
  transport and injects admitted capability implementations into kernel-owned
  service ports, preserving the rule that the kernel cannot import capability
  packages.
- `shells/vscode` remains a thin TypeScript adapter for stable VS Code APIs,
  accessible-reference collection, progress, approvals, cancellation, and
  rendering.

No second TypeScript workflow engine, document-processing engine, model loop,
or artifact store is introduced.

The current machine-readable VS Code contract prohibits all extension
workspace reads. That remains the implementation boundary until Story 1.2
narrows it through a reviewed contract change to request-bound resolution of
references explicitly supplied to the AgentMage participant. Ambient browsing,
independent path selection, and policy-bypassing reads remain prohibited.

### 3. Existing local transport is retained

The extension and Rust host continue to use the existing versioned framed
protocol over platform-native authenticated IPC: mode `0600` Unix-domain
sockets on Linux, authenticated App Group IPC on macOS, and access-controlled
named pipes on Windows. Loopback HTTP is not added merely for convenience.

The protocol is extended with bounded artifact-ingress frames, chunk streaming,
flow control, cancellation, progress, context manifests, workflow events,
approval challenges, resume bindings, and terminal diagnostics. Remote SSH,
WSL, and Dev Container placement must be resolved through explicit capability
negotiation and tests rather than an ambient network listener.

### 4. Visual Studio Code ingress is honest about its limits

The supported ingestion path is an AgentMage-owned chat participant or custom-
agent session that accounts for every reference exposed by the stable public
API for that request and attempts to resolve only content reachable through
documented APIs and current authority. Stable references can still contain
opaque or unavailable values; those remain visible unsupported or unavailable
states rather than a promise of bytes. The language-model provider remains a
model-picker and lossy compatibility surface and can process only parts
delivered to it.

An HTTP inference proxy cannot reconstruct omitted bytes. An MCP server cannot
read an ephemeral attachment without a path, URI, resource, handle, or staged
artifact. Proposed or private VS Code APIs are never required for the supported
path; experiments using them require a feature flag, version guard, fallback,
and separate non-support claim.

### 5. Models propose; the runtime establishes truth

Model output remains untrusted. Only the runtime may establish that an action
was admitted, approved, executed, observed, verified, retried, deferred, or
completed. Every success resolves to current executor or verifier evidence.

Destructive and external effects are never automatically retried. Unknown
effect classes fail toward approval, and uncertain outcomes are reconciled
before any new attempt. Model-specific policy may adjust prompt structure,
planning horizon, and repair budgets, but may not weaken authority, approval,
idempotency, evidence, or completion requirements.

### 6. Existing format and workflow work is composed, not duplicated

Existing text, Word, PDF, spreadsheet, runtime-artifact, context, event,
coordinator, tool, MCP, coding-harness, and recovery work becomes an adapter or
consumer of the two shared services. Format-specific rendering and generation
remain in their existing document sprints. Native tools remain registered
directly; MCP remains an optional later adapter behind the same dispatcher.

Source bytes remain memory-only by default. Explicit policy-approved resume or
retention may persist encrypted payloads through the existing runtime artifact
backend under separate logical source manifests. Absolute paths and original
URIs remain protected metadata and never enter model-visible manifests.

### 7. Evaluation is a release gate

Both epics share deterministic mock transcripts, malformed-stream fixtures,
corrupt and oversized artifacts, fault injection, process interruption, model-
provider matrices, security cases, and baseline-versus-runtime comparisons.
Release evidence measures verified completion, false completion, unsafe action,
silent failure, duplicate effects, provenance accuracy, context compliance,
recovery, latency, and resource use. Collected metrics do not substitute for
declared pass thresholds.

## Consequences

- Pasted text and documents receive one source-preserving artifact path instead
  of shell- or provider-specific preprocessing.
- Interactive coding, future workflows, and capability packs use one persisted
  execution truth rather than private retry loops.
- Rich format support may land in later owning sprints while the common
  artifact contract and plain-text/log path remain foundational.
- The roadmap gains stories and tasks but no renamed sprint, erased task,
  reopened completed evidence, or new release identity.
- Current product truth remains pre-alpha. Universal ingestion, the supported
  participant accounting path, and the complete verified workflow supervisor are planned,
  not shipped.
- Sprint 50 may close only the text/log plus verified-workflow
  `M-FOUNDATIONAL-RUNTIME-CORE`; required structured-document adapters close
  `M-FOUNDATIONAL-RUNTIME` no earlier than Sprint 62, while optional MCP remains
  later and non-blocking.

## Approval Record

On 2026-08-21, the user explicitly directed AgentMage to make Universal
Artifact Ingestion and Context Preparation and Verified Workflow Execution and
Recovery first-class runtime epics and to reconcile the architecture,
development plan, and task plan without implementing the complete production
runtime in this change.

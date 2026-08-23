# Decision 0045: Mandatory Verified Chat and Team Runtime

| Field | Value |
|---|---|
| Status | Accepted additive supersession |
| Date | 2026-08-23 |
| Scope | Canonical own-tab Verified Chat, persistent Rust Engineering Runtime, local and remote Model Gateway, and real Team execution |
| Supersedes | Only the optional, planning-only, or deferred interpretations of Decisions 0043 and 0044 |
| Preserves | All stable identifiers, additions-only history, completed evidence, deny-first authority, current status truth, strict-local completeness, and human-gated default-branch promotion |
| Does not authorize | Private VS Code APIs, model-owned authority or completion, silent fallback, unqualified endpoints, default-branch promotion, release publication, or unsupported status claims |

## Context

Decisions 0043 and 0044 established the Engineering Runtime, Verified Chat,
capability registry, and model-gateway architecture. Their accepted text also
described broad implementation as unauthorized by that planning pass and made
multi-agent execution conditional on later foundations. That wording can be
misread as allowing the first Engineering Runtime release to omit the canonical
chat surface, durable runtime, remote-route implementation, or Team mode.

The user has now explicitly directed implementation of the complete production
path. Accepted history must not be rewritten, so this decision supersedes only
that ambiguity.

## Decision

1. **AgentMage Verified Chat is the canonical AgentMage VS Code interface.** It
   must open in its own first-class editor-area tab and have a dedicated
   AgentMage Activity Bar entry. A sidebar, native Chat participant, language
   model provider, or terminal command alone cannot satisfy this requirement.
2. Verified Chat owns its composer, exact paste capture, attachment tray,
   context inspection, transcript, approvals, task controls, model and route
   selection, tool observations, verification cards, and Team projections.
3. Native `@agentmage` and Language Model Chat Provider surfaces are
   compatibility-only clients of the same Rust host. They fail closed when
   VS Code cannot provide exact, resolved, and verifiable context.
4. The Rust host, durable supervisor, artifact service, context service,
   workflow engine, tool-observation boundary, deterministic verifier, event
   journal, capability registry, model gateway, and Team coordinator are
   mandatory release-blocking functionality for the first supported
   Engineering Runtime release.
5. Remote routes are optional for a user to configure at runtime, but qualified
   `remote_private` and `remote_managed` route implementations, disclosure
   controls, credential references, hostile-endpoint tests, and live-gate
   procedures are mandatory product functionality. `strict_local` remains a
   complete offline profile.
6. Team mode must execute real concurrent workers under deterministic
   dependency, path, test-resource, worktree, branch, budget, review,
   correction, and serialized-integration controls. Schemas, fake workers, or
   sequential demonstrations cannot satisfy its integrated gate.
7. The qualified worker target is one coordinator and up to five isolated
   implementation workers. Default-branch promotion remains human-gated.
8. Planning, schemas, compilation, unit tests, mock UI, and fake endpoints are
   component evidence only. Integrated status requires the exact
   context-to-model-to-tool-to-verifier-to-resume path and the relevant live or
   platform qualification evidence.
9. Every component and campaign retains truthful lifecycle and verification
   status. Unavailable credentials, model artifacts, signing identities,
   hardware, external endpoints, and independent reviewers remain explicit
   blockers and cannot be inferred as passed.

## Consequences

- The implementation order must build the single-agent reliability spine
  before Team execution, while keeping every mandatory capability in the same
  release boundary.
- TypeScript and webview code remain disposable clients; closing a tab cannot
  cancel, complete, or erase Rust-owned work.
- No local-to-remote or remote-to-remote fallback occurs without an explicit
  policy, disclosure impact review, current qualification, and visible route
  decision.
- Existing completed work remains complete only for its proved historical
  scope. New release-blocking stories remain open until their current evidence
  passes.

## Approval Record

On 2026-08-23, the user explicitly directed execution of the Mandatory Verified
Chat and Full Engineering Runtime implementation, including the own-tab VS Code
surface, persistent Rust runtime, local and remote gateway, and real
multi-agent Team mode, while prohibiting unauthorized pushes and false status
promotion.

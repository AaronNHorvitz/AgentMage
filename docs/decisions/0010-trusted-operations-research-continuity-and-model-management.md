# Decision 0010: Trusted Operations, Research, Continuity, and Model Management

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-11 |
| Scope | First-GA shell authority, public research, credentials, continuity, and approved-model management; post-GA experimental models |
| Supersedes | The rule that final v1.0 closure occurs at Sprint 156 and the blanket interpretation of deferred unrestricted shell access |

## Context

Decisions 0008 and 0009 expanded AgentMage into a local-first delivery and productivity system.
Several capabilities necessary for practical agent work were present only as dispersed or later
roadmap items: command execution, public research, credential handling, backup and restore, and model
installation. Their existing descriptions did not provide one release-level architecture or an
explicit owner-controlled path to full host-user shell authority.

The product must be usable by people who should not need to install models or configure every
provider manually. It must also preserve work when a device fails, allow current information to be
researched with citations, and store service credentials without revealing them to a model.

An unrestricted shell creates a materially different risk from a workspace-confined command
runner. Cloud backup creates a materially different write path from read-only Cloud Observer.
Arbitrary model experimentation creates a materially different trust state from approved-model use.
Those differences must remain visible and testable instead of being hidden under broad automation.

## Decision

1. AgentMage adds `TRUSTED-OPERATIONS.md` as the normative architecture for command authority,
   public research, credential brokering, local and cloud continuity, approved-model management,
   and the Experimental Model Lab.
2. v1.0 GA includes a fully capable local command surface with Disabled, Inspect, Workspace
   Autonomous, Connected Operations, and Owner / Unrestricted Session levels intersected with the
   global Autonomy Center and all narrower policies.
3. Owner / Unrestricted Session runs with host-user authority only after a direct authenticated user
   activation. It is time bounded, visibly indicated, panic-stoppable, unschedulable, non-inheritable,
   non-renewable by a model, and revoked on lock, logout, restart, expiry, policy change, or incident.
4. The prior `DEFER unrestricted shell access` checklist statement remains historical. It continues
   to prohibit an ambient, default, silent, unattended, or model-activated unrestricted shell; it no
   longer prohibits the explicit owner session defined here.
5. v1.0 GA includes public Internet search and retrieval with freshness controls, claim-level
   citations, bounded downloads, hostile-content handling, and strict separation from authenticated
   browser effects and private-data disclosure.
6. v1.0 GA includes an operating-system-backed credential broker. Models, prompts, chat logs,
   process arguments, ordinary logs, diagnostics, exports, and backups never receive raw secrets.
7. v1.0 GA includes encrypted local snapshots and optional client-side-encrypted cloud continuity.
   Live operational databases, model stores, locks, indexes, queues, sockets, and temporary data do
   not run from cloud-synchronized storage.
8. Cloud backup is a narrowly writable Continuity capability with one exact destination namespace.
   It does not broaden the read-only Cloud Observer pack or grant general cloud-write authority.
9. v1.0 GA includes a chat-guided approved-model catalog and installer. A user can ask AgentMage to
   compare compatible approved profiles and install, import, verify, activate, roll back, or remove
   one through a deterministic, separately isolated workflow.
10. Meta Muse Glimmer is added as a candidate, not an approved profile. Its open-source or
    open-weight status and every required license, provenance, artifact, runtime, resource, quality,
    security, and platform fact remain unclaimed until first-party evidence and admission tests pass.
11. Muse Glimmer admission may produce `PASS`, `BLOCKED`, or `REJECTED`; a non-pass does not block
    first GA because Gemma remains the reference model.
12. A post-GA Experimental Model Lab may import and evaluate user-selected unapproved artifacts in a
    disposable, no-network, no-credential, no-tool, no-workspace-write sandbox. It retains the
    non-Chinese and non-Chinese-derived model rule unless a later accepted decision changes it.
13. Experimental results never create an approved catalog entry. Promotion requires the complete
    normal model-admission process and independent evidence.
14. Sprints 157 through 165 implement trusted operations. Sprint 166 is the superseding final v1.0
    GA decision gate. Sprints 167 and 168 implement the post-GA Experimental Model Lab and do not
    block v1.0 GA.

## Consequences

- AgentMage remains useful as a serious coding agent while its default command authority stays
  bounded and understandable.
- An informed device owner can deliberately grant broad host-user shell authority without implying
  that the mode is confined against commands executed within it.
- Current web evidence, connected credentials, continuity objects, and model artifacts each use a
  separate worker and capability class rather than sharing ambient network or secret access.
- Cloud backup can write encrypted objects without creating an infrastructure-management path.
- Nontechnical users gain a safe model-installation conversation while approval remains tied to
  deterministic manifests and checks.
- Arbitrary models can be explored later without weakening the supported model boundary.
- Sprint 156 remains a stable expanded-productivity checkpoint and no longer closes `G-GA`.

## Verification

- Canonical documents link `TRUSTED-OPERATIONS.md` and preserve this decision's first-GA and
  post-GA boundaries.
- The inventory appends stable first-GA and post-GA requirements, acceptance tests, and construction
  checklists without changing prior identifiers.
- `TASKS.md` preserves Sprints 0 through 156, appends Sprints 157 through 168, and assigns final
  first-GA closure only to Sprint 166.
- The security guide adds command-authority, owner-session, research, credential, continuity,
  model-manager, Muse-candidate, experimental-lab, and removal protocols.
- Runtime documentation declares every added worker, socket, privilege, credential, storage, and
  network edge.
- Generated registry, policy, traceability, Markdown, Mermaid, link, identifier, and additions-only
  checks pass after deliberate review and baseline extension.

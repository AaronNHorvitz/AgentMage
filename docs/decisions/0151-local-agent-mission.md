# Decision 0151: Local Agent Mission

| Field | Value |
| --- | --- |
| Status | Accepted under explicit owner instruction |
| Date | 2026-10-25 |
| Authority | Owner mission and lane start, 2026-10-25 |
| Scope | Linux local use, Fedora Kinoite first; no public release |

## Context

The owner needs an installable local agent for conversation, workspace tasks,
document questions with citations, bounded research and repository coding help.
The preserved programme and its evidence remain useful engineering inputs, but
its unfinished rows do not block this mission. Existing component tests do not
demonstrate an integrated application or a real model run.

## Decision

1. Make T-301 through T-310 in the new active queue at the top of TASKS.md the
   delivery order. Preserve the old ledger and its checkboxes. This instruction
   supersedes earlier scope freezes, stop markers and work selection rules;
   respect any new operator STOP. Documents (including PDF) and research precede
   repository coding help. Engineering team coordination is deferred.
2. Use a local browser UI served by the Rust runtime, with packaged offline assets,
   one interface with contextual detail, keyboard operation and light/dark themes.
   This removes the embedded webview dependency from the Linux mission. The
   existing client and host components may be reused where their boundaries fit.
   This is an implementation choice, not a claim that the UI already exists.
3. Expose every capability through a versioned local HTTP API, also used by the UI:
   a local API other programs can use. Before exposing consequential operations,
   require loopback binding, authenticated clients, browser origin checks, request
   limits and permission enforcement by the runtime. A browser page cannot grant
   itself authority. Specify and test these protections with implementation.
4. Configure a local OpenAI-style endpoint and optional key, detect or request it
   on first run, test it and stream chat. Do not bundle a model or a provider key.
   Keep credentials out of logs and committed evidence. Real model checks use only
   the lane's existing model files and reserved GPU slot; fixtures stay labeled.
5. Preserve the product's permission model. Workspace file access is scoped;
   commands run only through its confined tool; writes outside scope, commands
   and off-machine data need the defined authorization. Effects, decisions and
   refusals are visible and recorded. Runs must resume and replay from records.
6. Start documents with a simple built-in local index. A configurable memory
   service with a versioned HTTP API and MCP is optional. Keep exact source spans,
   re-import/removal semantics and synthetic citation tests. Research stays offline
   unless enabled for the run, using a configured search provider and strict
   time/page/byte budgets. Coding uses an isolated worktree and confirmed apply.
7. Provide a user-level Kinoite install, user service, launcher, uninstall and an
   owner exercise guide. No root, layered packages, hosted service, other operating
   systems, paid actions, accounts or model acquisition belong to this mission.
8. Establish a minutes-scale per-change gate for format, lint and touched-component
   unit/integration tests. Preserve the existing long documentation gate unchanged
   for one acceptance run at T-310. Do not regenerate supply-chain or evidence
   outputs per source edit; batch regeneration without narrowing hash inputs.
   Publish observed measurements and failures without turning old programme rows
   into new prerequisites. Never weaken a test or threshold to obtain a pass.
9. Close a row only with an independent review of its exact commit, committed under
   docs/reviews/, with `Verdict: PASS` and no open high or medium finding. Commit
   sanitized row evidence under docs/evidence/. Every review request declares
   `Tier: core` for runtime, tools, permissions, sandbox, file/network access,
   prompts or credentials; UI, documentation and packaging alone use `plugin`.
   Fix high/medium findings and request re-review. After three non-PASS reviews or
   three failures of an approach, retain the original open row and add a sub-row
   for remaining findings, continue dependency-ready work, and return by T-309.
   Low findings become backlog notes. Builder self-review is not independent review.
10. Codex builds; a separate read-only reviewer session reviews. At a review handoff,
    write the exact commit and reproduction commands to the private review request
    and end the builder session. Prepare owner testing after T-308 verification
    and T-310 independent PASS; public release awaits owner testing and authority.

## Consequences and truth

This record authorizes work, not completion. It promotes no status-model entry and
does not close any historical gate. Preserve runtime, licensing, IP, resource and
independent-review boundaries. New evidence must distinguish fixtures, real
backend observations, unexercised paths and environmental failures.

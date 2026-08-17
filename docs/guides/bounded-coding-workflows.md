# Bounded Coding Workflows

## Current Availability

The coding contracts, declarative skill pack, reusable coordinator, native coding catalog, MVP
fixture groups, and durable journal, artifact, checkpoint, and continuation primitives are
source-level pre-alpha evidence. AgentMage does not yet expose an authenticated, admitted-model
coding workflow through native Chat or the local CLI. The sequence below defines the intended review
path and stop conditions; it is not a supported-product claim.

The interactive harness remains a planned thin client of the source-level reusable kernel runtime
coordinator. It does not own a model loop, tool router, approval system, journal, session store, or
artifact store. Built-in local coding tools register directly through the common tool dispatcher;
MCP remains an optional later adapter for reviewed external tools.

The earliest useful `M-HARNESS-MVP` path includes one admitted local model, one approved repository
and owned worktree, exploration/read/search, patch and controlled creation, bounded commands,
targeted tests, Git status/diff/log/show, exact approvals, streaming/cancellation, runtime events,
bounded output with explicit truncation, current receipts, and a final evidence-backed summary.
Persistent session resume, complete durable-journal and content-addressed-artifact lifecycle, remote
Git, commit, push, advanced indexing, routing, MCP, full conversation-library behavior, workflow
orchestration, and multiple agents remain later work and do not become hidden prerequisites for
that milestone.

## Repository Comprehension

1. Select one exact repository, owned worktree, branch, revision, and approved root.
2. Inspect the deterministic repository map, parser coverage, entry points, dependencies, symbols,
   relationships, tests, configuration, and exact citations.
3. Use lexical-only coverage for unsupported languages and keep unavailable structure unknown.
4. Choose the smallest applicable declarative skill and inspect its identity, hash, purpose,
   advisory evidence, completion criteria, scope, and permanent authority denial.
5. Stop on stale sources, incomplete coverage represented as complete, unresolved path identity,
   missing citations, or repository text presented as authority.

## Plan and Change

Follow the [change-plan review](./reviewing-change-plans.md) before implementation. A plan must bind
the requested behavior, current evidence, target citations, impact, alternatives, tests, risks,
rollback, and explicit exclusions. Apply only an exact-preimage shadow proposal reviewed through
the [structured-change guide](./reviewing-structured-code-changes.md).

An owned worktree protects repository-state isolation; it does not replace the platform sandbox.
Stop on a dirty or divergent base, unowned worktree, collision, stale preimage, changed plan,
unsupported structural claim, generated-output mismatch, unrelated file, dependency upgrade,
migration, broad refactor, or project instruction that attempts to select commands or authority.

## Commands and Validation

Formatting, linting, type checks, tests, builds, package checks, and scans require separate exact
trusted command templates. Review the executable, literal arguments, environment, directory,
network denial, process limits, output limits, parser, expected artifacts, and repository snapshot
using the [validation guide](./reviewing-validation-results.md).

Cancelled, timed-out, crashed, truncated, malformed, skipped-only, zero-test, sensitive-output,
partial, stale, and unrun checks are not passes. A failed required check blocks completion and local
commit. Repository configuration and model output are untrusted inputs and cannot mutate a
registered command.

## Review and Local Commit

Review the complete diff, purpose assignment, validation evidence, findings, unresolved risk,
rollback, and excluded unrelated changes. A review packet explains the state but grants nothing.
Use the [local-commit guide](./reviewing-and-creating-local-commits.md) only after the exact candidate
tree, signer, parent, message, task branch, user index, preservation manifest, and approval are
current.

A local commit does not authorize push, pull-request publication, review submission, merge,
release, deployment, dependency upgrade, migration, amend, reset, clean, discard, force operation,
or history rewrite. Those operations remain absent from the v0.4 release boundary.

## Rollback, Recovery, and Troubleshooting

Rollback is a fresh reversed request against current bytes, not a reusable prior grant. Preserve a
later user edit as a conflict. On cancellation, crash, uncertain apply, validation failure, branch
movement, or signature mismatch, retain the content-free transaction and recovery identities and
reconcile before another attempt. Never hide cleanup through reset, discard, force, implicit stash,
or deletion of an unowned worktree.

The current [`agent` CLI](./local-command-line-interface.md) returns
`client.transport.failed` for operational commands because authenticated product transport is not
composed. Treat that as an unavailable dependency, not as a coding failure and not as permission to
use a hidden fallback.

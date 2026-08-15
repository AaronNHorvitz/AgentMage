# Repository Worktree Boundary

## Status

Sprint 42 implements a closed repository-planning contract and a Fedora-local
worktree/CAS adapter. The implementation is not registered in a product profile.
Remote clone/fetch plans are representable, but the current Linux executor denies
every network invocation until authenticated transport, credential brokering, and
complete process-tree containment are implemented and reviewed.

This document describes current code. It makes no GitHub publication, release,
cross-platform, or independent-review claim.

## Components

| Component | Responsibility | Authority |
|---|---|---|
| [`repository_safety.rs`](../../kernel/engine/src/repository_safety.rs) | Canonical remotes and refs, currentness, minimized manifests, fixed Git plans, worktree ownership, collision checks, postcondition reconciliation, receipts | Plans and validates; no native path, process, credential, or socket access |
| [`repository_safety.rs`](../../platforms/linux/src/repository_safety.rs) | Pinned Git identity, owned-path verification, bounded observations, fixed guest-path mapping, Fedora-local worktree/CAS execution | Local process and admitted repository paths only after a kernel permit |
| [`repository-preservation-manifest.schema.json`](../../schemas/runtime/repository-preservation-manifest.schema.json) | Closed manifest interchange format | Data contract only |
| [`repository-operation-plan.schema.json`](../../schemas/runtime/repository-operation-plan.schema.json) | Closed operation preview and invocation format | Data contract only |
| [`repository-operation-receipt.schema.json`](../../schemas/runtime/repository-operation-receipt.schema.json) | Terminal authority/plan/postcondition binding | Data contract only |
| [`worktree-ownership.schema.json`](../../schemas/runtime/worktree-ownership.schema.json) | Task ownership, retention, handoff, cleanup, and disposition | Data contract only |

## Authority Flow

```mermaid
flowchart LR
    Request[Repository request] --> Plan[Kernel exact plan]
    Plan --> Preview[Plan and manifest preview]
    Preview --> Grant[Exact single-use grant]
    Grant --> Driver[Kernel effect driver]
    Driver --> Permit[Nonforgeable launch permit]
    Permit --> Linux[Linux repository adapter]
    Linux --> Post[Fresh terminal manifest]
    Post --> Reconcile[Kernel reconciliation]
    Reconcile --> Receipt[Authority-bound receipt]
```

The model, repository, Git configuration, remote output, and user interface cannot
construct a `RepositoryLaunchPermit`. The kernel driver requires exact operation,
held-object, serialized-plan, argument-hash, and consumed-grant agreement before it
creates one permit. A cancellation observed before launch returns a cancelled,
unchanged manifest pair and never calls the platform executor.

## Representable Operations

The Sprint 42 type has exactly five variants:

- `clone`: initialize one no-checkout owned bare repository and fetch one full branch ref into `refs/agentmage/fetch/<transaction>/source`.
- `fetch`: fetch one full branch ref into the same transaction namespace.
- `worktree_create`: create one `refs/heads/agentmage/tasks/<task>` branch and one owned worktree from an immutable object.
- `worktree_remove`: remove one proven clean, process-free, recovery-retained owned worktree without force.
- `branch_fast_forward`: prove ancestry and compare-and-swap one AgentMage task branch from one expected old object to one exact new object.

There is no generic command, pull, merge, rebase, reset, clean, discard, stash,
tag, note, remote-configuration, push, force, mirror, deletion, arbitrary ref, or
user-branch fast-forward variant. The fixed mutation corpus verifies these remain
unrepresentable.

## Hardened Invocation

Every planned invocation is a literal argument vector after a pinned Git executable.
The fixed prefix disables hooks, filesystem monitors, prompts, credential helpers,
pagers, editors, external diff, automatic maintenance, garbage collection,
commit-graph writes, recursive submodules, and Large File Storage smudge. Protocols
default to denied; one plan may admit only its selected HTTPS or SSH protocol.

The environment is replaced rather than inherited. It contains only fixed locale,
noninteractive, no-global-config, no-system-config, no-LFS-smudge, and empty-home
settings. Git, SSH, askpass, proxy, tracing, object-directory, alternate-object,
index, namespace, pager, editor, and credential variables cannot cross from the
caller.

The Linux adapter verifies and holds a root-owned, non-writable Git executable and
revalidates both the held descriptor bytes and launch path before each operation.
Only `/repo`, `/owned/worktree`, and `/owned/repository` tokens can be mapped by the
adapter. Unknown or embedded path tokens fail before spawn.

## Preservation Manifest

The collector retains hashes, counts, identities, and booleans rather than raw path
names, ignored-file bytes, config values, credentials, hook content, or user content.
It covers:

- repository/common-directory identity, ownership, object format, `HEAD`, branch, upstream, detached state, and index;
- staged/unstaged/untracked/ignored dispositions;
- user branches, remote refs, tags, notes, stash, replacement refs, reflogs, AgentMage refs, and worktrees;
- submodules, Large File Storage declarations, object database, remotes, config, hooks, attributes/drivers, locks, and in-progress operations;
- shallow, partial, alternate-object, active-hook, executable-filter/driver, alias, URL-rewrite, helper, pager/editor/signer, and protocol hazards.

The manifest is collected before preview by the host, immediately before launch by
the Linux adapter, and after every result. The kernel permits only operation-owned
field changes. Fetch may change quarantined object and AgentMage transaction-ref
identity. Worktree lifecycle may change AgentMage refs and the worktree registry.
Fast-forward may change AgentMage refs only. Failed, denied, cancelled, or timed-out
attempts must return a byte-equivalent manifest.

## Worktree Lifecycle

An ownership record binds task, source object, AgentMage task branch, owner, grants,
owned-file digest, live process count, resource-budget identity, retention, recovery,
cleanliness, and disposition. Registry updates are compare-and-swap operations bound
to the prior record digest. Duplicate worktree path or branch identity is refused.

Automatic removal requires all of the following:

- disposition is `cleanup_eligible`;
- the worktree is proven clean;
- no attributed process remains;
- a recovery artifact is retained;
- path, branch, source, task, and owner identities still match.

A handoff changes disposition to `handed_off`; it does not silently transfer cleanup
authority. Interrupted or uncertain work changes disposition to `recovery_required`
and remains on disk.

Worktrees isolate task changes and reduce checkout/concurrency collisions. They are
not the operating-system security sandbox. Process, filesystem, network, and secret
containment remain responsibilities of the platform sandbox and kernel grants.

## Current Local Evidence

Fedora fixture tests execute worktree create/remove and compare-and-swap against real
Git repositories. They compare active `HEAD`, symbolic branch, index bytes, and task
worktree disposition; stale old-object updates fail without changing the target ref.
The collector detects untracked state and executable local configuration. A fixed
44-case corpus rejects hostile remotes, refs, and operation arguments. A deterministic
10,000-case manifest campaign changes one protected state field per case and records
zero accepted fetch reconciliations.

## Known Gaps

- The Linux executor does not launch clone/fetch or any other networked invocation.
- The current direct local process runner can terminate the Git parent but does not yet prove complete hostile descendant-tree containment.
- Clone success needs a distinct empty-destination-to-repository postcondition contract; current generic reconciliation refuses a successful clone claim.
- Platform-observed CPU, memory, task, descriptor, and byte budgets are incomplete.
- Native Ubuntu, macOS, and Windows execution evidence is absent.
- A full hostile hooks/filters/drivers/alternates/Unicode/case/lock/interruption campaign is absent.
- Product-profile registration, trusted-package execution, credential brokering, independent review, and deferred manual fuzzing are absent.

These gaps keep Sprint 42 and every dependent release gate blocked.

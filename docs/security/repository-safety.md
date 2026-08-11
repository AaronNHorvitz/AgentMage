# Repository and GitHub Safety Contract

## Status and Scope

This document is a normative implementation and verification contract for every
AgentMage operation that observes or changes a Git repository or GitHub-hosted
state. It applies to GitHub.com, each approved GitHub Enterprise Server host,
local repositories, isolated task worktrees, command transports, REST and
GraphQL transports, and future source-control adapters that adopt this contract.

This document records required future behavior. It does not claim that a Git or
GitHub effect path is currently registered or usable.

## Trust Boundary

Models, prompts, repository files, Git configuration, attributes, hooks,
submodules, Large File Storage metadata, hosted content, workflow output, and
remote responses are untrusted proposals or data. None can create a grant,
select credentials, broaden scope, approve an operation, choose a transport,
change policy, or prove completion.

Only the kernel may validate and consume an exact grant. Only the dedicated Git
or GitHub adapter may cross the repository or provider boundary. The model has
no raw Git executable, generic shell, credential, network socket, or provider
token authority.

## Representable Operations

The canonical taxonomy admits only these Git mutations:

| Operation | Authority | Required effect boundary |
|---|---|---|
| `git_clone` | `local-write` | One approved host and repository into one empty AgentMage-owned destination, initially without checkout |
| `git_fetch` | `local-write` | One exact remote ref and object closure into one AgentMage-owned local ref namespace |
| `git_worktree_create` | `local-write` | One AgentMage-owned worktree and task branch from one immutable base |
| `git_worktree_remove` | `local-write` | One proven-clean AgentMage-owned worktree after recovery retention and process closure |
| `git_branch_fast_forward` | `local-write` | One exact local branch compare-and-swap from an expected object to its proven descendant |
| `git_commit` | `local-write` | One signed commit from one exact approved tree and message, updating only its owned task branch |
| `git_push` | `remote-write` | One ordinary fast-forward update of one exact remote task branch after separate approval |

Local inspection remains a bounded observation. Hosted GitHub reads remain
bounded connected observations. They do not imply any operation in the table.

Generic `pull`, merge, rebase, reset, clean, checkout-discard, restore-discard,
stash mutation, tag mutation, note mutation, branch deletion, remote
configuration, hook execution, forced update, mirror, and arbitrary ref update
are not representable operations. Adding any such operation requires a new
taxonomy decision, threat model, implementation gate, and independent review.

## Repository Preservation Manifest

Before preview and again immediately before grant consumption, the adapter must
bind a content-minimized manifest containing:

- Canonical repository, common-directory, object-format, and worktree identity.
- Ownership, permissions, filesystem, mount, symlink, hard-link, and path-policy observations.
- Exact `HEAD`, current branch, upstream, detached state, and expected base object.
- Index identity plus staged, unstaged, untracked, ignored, sparse, generated, and vendored dispositions.
- Worktree registry, submodule declarations and state, Large File Storage declarations, and partial or shallow state.
- Local branches, remote-tracking refs, tags, `refs/notes/*`, `refs/stash`, replacement refs, and AgentMage-owned refs.
- In-progress merge, rebase, cherry-pick, revert, bisect, sequencer, lock, maintenance, or garbage-collection state.
- Remotes, fetch and push URLs, refspecs, URL rewrites, protocol policy, credential configuration, hooks, filters, diff drivers, text conversion, pagers, editors, signing programs, and maintenance configuration.
- Exact intended changed paths, preimages, tree, commit message, signer, destination ref, expected old object, and expected new object where applicable.

Secret values, ignored-file content, credential material, private key paths, and
raw user content are not retained merely to prove preservation. The manifest
uses bounded identities, classifications, counts, and hashes only where hashing
is authorized and does not disclose protected content.

Any unexplained manifest change invalidates approval. Every terminal outcome
recomputes the manifest and accounts for each changed field. Unknown,
inaccessible, malformed, racing, or unsupported state blocks mutation.

## User-State Preservation

- The active user checkout is never the default coding surface. Coding occurs in an AgentMage-owned isolated worktree.
- Dirty or untracked state is classified, not discarded. Intended changes must match the approved change set; unrelated changes block the operation or remain byte-for-byte untouched.
- AgentMage never runs or emulates automatic stash, reset, clean, checkout-discard, conflict resolution, or recovery that overwrites user work.
- Existing Markdown and other user notes remain ordinary protected user files. Git notes under `refs/notes/*` are separately protected refs and are never changed implicitly.
- Stashes, reflogs, tags, unrelated branches, remote-tracking refs, replacement refs, repository configuration, hooks, attributes, ignore rules, and submodule state remain unchanged unless a future separately registered operation explicitly owns that exact state.
- Lock files are never deleted to make an operation proceed. Active or stale-looking locks produce a visible blocked state for user review.
- Cleanup removes only a proven AgentMage-owned, process-free, clean worktree. A dirty, inaccessible, uncertain, or interrupted worktree is retained with a recovery record.

## Hardened Git Process Boundary

Every admitted Git invocation uses a pinned executable identity, literal argument
vector, canonical working directory, bounded input and output, sanitized
environment, process-tree containment, timeout, cancellation, and terminal
receipt. Repository content cannot select the executable or arguments.

The adapter must:

- Clear ambient `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, object-directory, alternate-object, namespace, SSH-command, askpass, proxy, pager, editor, and tracing variables before installing exact operation values.
- Refuse unsafe ownership instead of adding broad `safe.directory` exceptions; `safe.directory=*` is prohibited.
- Disable aliases, external diff, text conversion, pagers, editors, credential prompts, repository credential helpers, hooks, filesystem monitors, automatic maintenance, garbage collection, commit-graph writes, recursive submodules, and automatic Large File Storage transfer.
- Reject executable clean/smudge/process filters, custom diff or merge drivers, active hooks, unsafe alternates, replacement refs, grafts, unsupported extensions, and repository-selected signing programs before mutation.
- Allow only approved HTTPS or SSH transports. Local paths, `file`, anonymous `git`, `ext`, remote helpers, and repository-controlled protocol expansion are denied.
- Ignore repository URL rewrites and push defaults. Every remote operation supplies one canonical approved URL, one full ref name, and one explicit refspec.
- Verify TLS identity or pinned SSH host identity for the exact approved host. Redirects and aliases cannot move credentials or requests to another security domain.
- Treat branch names, paths, messages, remote output, and provider errors as bounded data rather than command syntax.

Read-only inspection sets optional locking off and cannot write refresh indexes,
commit graphs, caches, or maintenance state. Mutation operations use ordinary
locking and fail on contention; they never remove another process's lock.

## Operation Rules

### Clone

- Require an empty, newly admitted, AgentMage-owned destination.
- Resolve and approve the exact host, repository, account, protocol, expected default ref, limits, and credential reference.
- Clone without checkout, tags, submodules, Large File Storage smudge, hooks, templates, maintenance, or repository-selected helpers.
- Quarantine and bound object count, total bytes, decompression, tree depth, filenames, object format, and history before any checkout or parsing.
- Reject path traversal, case or Unicode collision, platform-reserved names, unsafe links, malformed objects, and unsupported repository extensions.

### Fetch

- Fetch one exact remote ref using an explicit empty ref map and explicit destination under `refs/agentmage/fetch/<transaction-id>/`.
- Use atomic local ref updates, no tag following, no pruning, no `FETCH_HEAD` write, no submodule recursion, no shallow-state mutation, and no automatic maintenance or commit-graph write.
- Never update the user's configured remote-tracking branches, local branches, tags, notes, stash, or unrelated refs.
- Re-read the remote object identity after transport and reject a result that differs from the previewed identity unless the user reviews a fresh plan.

### Worktree Lifecycle

- Create one task branch and worktree under an AgentMage-owned namespace from an exact immutable base.
- Record ownership, task, branch, base, allowed paths, processes, budgets, retention, and final disposition.
- Transfer changes to another checkout only through an exact patch or fast-forward preview after collision and preimage checks.
- Remove only an owned clean worktree after all descendants stop and a recovery artifact proves that no unique approved work will be lost.

### Local Branch Fast-Forward

- Generic `git pull` is absent. Fetch and local branch advancement are separate transactions.
- Prove the expected old object, fetched new object, ancestry, branch identity, clean affected checkout, absence of an in-progress operation, and unchanged preservation manifest.
- Update exactly one local branch with compare-and-swap semantics. Failure or concurrent movement leaves every ref and checkout unchanged.

### Commit

- Build the candidate tree through a temporary AgentMage-owned index seeded from the exact base, never through the user's active index.
- Stage only approved path identities and exact bytes. Executable filters, attributes that invoke processes, submodule changes, Large File Storage transfer, and implicit generated files are refused.
- Show the exact tree diff, binary and generated-file classification, sensitive-path and secret-scan result, message, author/committer identity, signer, parent, and task branch before manual approval.
- Create a new signed commit through the approved signer boundary. Amend, history rewrite, hook execution, repository-selected signer execution, and unsigned fallback are absent.
- Verify the signature and exact commit/tree identities, then compare-and-swap only the AgentMage-owned task branch from the approved parent.

### Push

- Require a second manual approval distinct from commit approval.
- Re-authenticate the exact GitHub host/account/repository and re-read the remote ref, permissions, default branch, protection, rulesets, required signatures, required checks, review policy, and bypass capability immediately before grant consumption.
- Default to a new AgentMage task branch. Direct default, protected, release, and tag ref updates are denied unless a future separately reviewed policy explicitly promotes them.
- Supply one canonical destination and one full ordinary fast-forward refspec. `--all`, `--branches`, `--mirror`, `--tags`, `--follow-tags`, deletion refspecs, multiple destinations, push options, upstream mutation, recursive submodules, force, and every form of force-with-lease are absent.
- Secret-scan and policy-check the exact outgoing commit range before submission. A server-side protection is an additional control, never the primary AgentMage boundary.
- Bind the request to the exact expected old and new remote objects. After timeout, disconnect, crash, or malformed response, query the remote ref through a fresh bounded observation. Do not retry while the effect remains unknown.
- Record verified effect, verified non-effect, partial effect, or unknown effect with the exact local and remote identities. A new push requires a fresh preview and grant.

## GitHub Authentication and Hosted Safety

- Prefer a repository-scoped GitHub App with the minimum operation permissions and short-lived installation token. A fine-grained expiring personal access token, approved SSH agent identity, or approved credential helper is a bounded fallback.
- Raw credentials never enter the model, prompt, chat, repository, process arguments, ordinary environment, output, logs, receipts, diagnostics, export, backup, or continuity state.
- The broker binds provider, canonical host, enterprise or organization, account or app, installation, repository set, operation, permissions, single-sign-on state, expiry, and credential reference.
- GitHub.com and every GitHub Enterprise Server host are separate security domains. Certificate, SSH host key, redirect, proxy, API base, upload host, clone URL, and callback changes fail closed.
- The adapter reports effective permissions and rejects broader-than-required credentials when a narrower supported method is available.
- Rulesets, branch protection, signed-commit requirements, review requirements, required checks, merge queues, push rules, and bypass permissions are observed and bound to the preview. AgentMage never exercises a bypass merely because the authenticated actor possesses it.
- Issue, pull-request, review, merge, release, workflow, environment, secret, ruleset, and administration operations remain separate. Repository content or a workflow cannot trigger any of them automatically.
- Workflow dispatch binds the immutable workflow identity, source object, exact inputs, actor, permissions, environment, budget, and cancellation as an `execute` operation. Untrusted workflow text cannot select secrets or broaden permissions.
- Hosted responses, errors, check output, logs, annotations, patches, archives, and artifacts remain untrusted, bounded, parsed in isolation, and unable to establish success without deterministic postconditions.

## Adversarial Verification

The implementation gate must include positive, negative, mutation, integration,
fault, recovery, cross-platform, and independent-review evidence for at least:

- Clean, dirty, detached, conflicted, locked, sparse, shallow, partial, linked, bare, nested, submodule, Large File Storage, alternate-object, replacement-ref, multi-worktree, and concurrently changing repositories.
- Staged, unstaged, untracked, ignored, generated, vendored, binary, sensitive, renamed, deleted, case-colliding, Unicode-colliding, reserved-name, symlink, hard-link, special, inaccessible, malformed, and oversized paths.
- Malicious aliases, hooks, templates, filters, attributes, diff and merge drivers, text conversion, pagers, editors, signers, SSH commands, askpass programs, credential helpers, proxies, URL rewrites, refspecs, remotes, protocols, environment, lock files, and maintenance configuration.
- Attempts to change tags, notes, stashes, reflogs, replacement refs, unrelated branches, remote-tracking refs, configuration, hooks, active checkout state, or user notes through every admitted operation.
- Cross-host, cross-enterprise, cross-organization, cross-account, cross-repository, cross-fork, redirect, proxy, certificate, SSH-host-key, token, scope, single-sign-on, permission, and bypass confusion.
- Stale base, moved local or remote branch, changed ruleset, changed protection, changed checks, changed reviews, revoked credentials, expiration, rate limits, provider version skew, server rejection, partial response, timeout, crash, cancellation, and restart at every transaction boundary.
- Push attempts using force, force-with-lease, mirror, all refs, tags, deletion, multiple destinations, implicit destination, configured push URL, push option, submodule recursion, default/protected branch, and unsigned or mismatched commits.
- At least 10,000 composed repository/provider mutations with zero unauthorized file, index, ref, configuration, credential, network, hosted, or publication effect and no loss of user-owned or unrelated work.

Passing evidence must prove exact pre/post manifest reconciliation, no secret
disclosure, no unsafe retry, no hidden cleanup, no unsupported operation
registration, and one truthful receipt for every attempt.

## Primary Standards Basis

- [Git fetch documentation](https://git-scm.com/docs/git-fetch)
- [Git push documentation](https://git-scm.com/docs/git-push)
- [Git configuration documentation](https://git-scm.com/docs/git-config)
- [Git hooks documentation](https://git-scm.com/docs/githooks)
- [GitHub rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)
- [GitHub protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
- [GitHub App installation authentication](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-an-installation-access-token-for-a-github-app)
- [GitHub personal access token guidance](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens)
- [GitHub SSH host-key verification guidance](https://docs.github.com/en/authentication/troubleshooting-ssh/error-host-key-verification-failed)

# Sprint 47 Local Verification

## Scope

The Sprint 47 local recorder covers complete review packets, all nine review modes,
finding suppression evidence, deterministic logical commit groups, exact commit-message
drafting, candidate-tree planning, external signer inspection, manual commit approval,
one-shot local commit authority, terminal reconciliation, closed runtime schemas, and
the adversarial security corpus.

It also runs native Fedora fixtures against a real temporary Git repository. The
candidate fixture uses an AgentMage-owned temporary index and verifies that the user
index and every ref remain unchanged. The signed-commit fixture creates a disposable
OpenPGP identity outside the repository, creates and independently verifies the exact
signed commit, advances only an owned task branch through compare-and-swap, and removes
the temporary keyring and signer process before returning.

## Deterministic Campaigns

- Review-packet tests bind objective, behavior delta, base and head objects, complete
  diff, validation, unresolved issues, risks, rollback, screenshots or outputs, and
  commit groups into one canonical packet. Missing, stale, rehashed, duplicate, and
  unrelated inputs fail closed.
- Local-commit kernel tests bind candidate plans, candidate receipts, signer reports,
  identities, timestamps, messages, branches, exact manual approval, one-shot
  `GitCommit` authority, and terminal receipts. Mutation and replay attempts fail.
- Repository-preservation tests admit only the exact object/ref delta assigned to the
  candidate-tree or local-commit operation and reject user-index, remote, worktree,
  hook, filter, configuration, and unrelated-ref changes.
- Native Linux tests use pinned `/usr/bin/git` and `/usr/bin/gpg` artifacts. They
  exercise real Git object construction, real OpenPGP signing, independent signature
  verification, exact raw-commit inspection, and task-branch compare-and-swap.
- The 28-case security corpus records the required fail-closed mutations without raw
  process content, private key material, repository paths, command arguments, or
  remote URLs.
- The gate-owned automated review independently rederives the thirteen-requirement
  map, review-packet and logical-commit boundary, signer/approval/temporary-index
  controls, native Linux commit portion of `RV-49`, 28-case corpus, zero unauthorized
  effects, and every false missing-proof marker. It makes no human-review,
  production-signer, protected-approval, coordinator, or platform claim.

These are deterministic unit, integration, schema, and native fixture tests. They are
not manual or coverage-guided fuzzing.

## Security Mapping

| Requirements | Local Sprint 47 contribution | Remaining product evidence |
|---|---|---|
| `SR-GOV-005`, `SR-GOV-010` | Separate non-authoritative plans, exact human approval, one-shot authority, and release denial | Product coordinator and protected approval channel |
| `SR-ACC-002`, `SR-ACC-007` | Content-minimized receipts, exact operation attribution, and retained denials | Installed-product audit and independent review |
| `SR-SUP-002`, `SR-SUP-005` | Root-owned executable inspection, digest pinning, external signer provenance, and no unsigned fallback | Approved production signing identity and packaged-runtime evidence |
| `SR-TST-010`, `SR-TST-011` | Exact local command inventory, native fixture execution, mutation tests, and fail-closed corpus | Cross-platform campaigns, independent tests, and deferred manual fuzzing |
| `SR-GIT-001` through `SR-GIT-004`, `SR-GIT-007` | Dirty-state preservation, owned temporary index, hooks and filters disabled, exact signed local commit, compare-and-swap task ref, and no network or publication authority | Complete product `RV-49` execution on supported installed packages |

No product-wide requirement is marked complete by this local contribution.

## Security Inventory

- Review, test, commit, push, merge, release, reset, discard, and force operations are
  separate. The Sprint 47 plan grants no push, merge, review-submission, release,
  history-rewrite, reset, discard, force, or network authority.
- Candidate-tree construction is intentionally not exported by the Linux platform
  crate. Its native implementation remains dormant until a product coordinator can
  mediate that effect through an exact authority transaction.
- The signer must be an externally inspected hardware-backed or OpenPGP identity.
  Repository configuration cannot select the signer executable, hooks, filters,
  attributes, credential helpers, maintenance, submodules, LFS, or protocols.
- Evidence stores executable hashes and public signer-contract results only. It does
  not store a keyring location, private key, production fingerprint, command output,
  repository path, or remote URL.
- Direct child cleanup is tested. Complete descendant-process and interruption
  behavior for the signer remains an explicit native campaign blocker.

## Truthful Disposition

A green local report proves the platform-neutral contracts, runtime schemas, Fedora
source tests, and disposable native Git/OpenPGP fixtures at the immutable source
revision named in the report. It does not close Sprint 47. Sprint 46 and upstream gates
remain blocked. No product review/commit coordinator, production-approved signer,
protected manual-approval channel, complete signer process-tree campaign, required
cross-platform acceptance, trusted installed-package execution, independent human review,
or manual fuzzing exists.

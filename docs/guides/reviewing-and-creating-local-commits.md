# Reviewing and Creating Local Commits

## Current Availability

The review and signed-commit boundary is source-level pre-alpha evidence. It is
not registered in the Visual Studio Code shell or any supported product profile.
The steps below define the intended review contract and the evidence a future UI
must show; they are not a claim that users can complete the workflow today.

## Review the Packet

Before candidate-tree construction, verify that one local review packet shows:

- The requested objective, behavior delta, exact base object, and repository.
- Every changed path and its complete diff.
- Validation results and every requested check that was not run.
- Visible findings, suppressed-finding evidence, residual risk, and rollback.
- Logical commit groups and a separate list of excluded unrelated changes.

Stop when a file, diff, test, unresolved issue, rollback step, or unrelated change
is missing or stale. A packet is explanatory evidence only and cannot authorize a
commit.

## Review the Candidate Tree

Candidate-tree evidence must name the exact parent, tree, temporary-index
identity, unchanged user-index identity, and every approved path, postimage, blob,
and file mode. The terminal receipt must show that only Git objects changed and
that no ref changed.

Any changed staged state, unexpected generated output, unsupported file mode,
filter result, or missing path requires a new candidate build and a new preview.

## Review the Signer

The signer report must identify either an approved hardware-backed signer or an
approved OpenPGP identity inspected outside the repository. Confirm the public
fingerprint through a trusted channel. Never approve a repository-selected signer
program, an unknown keyring, or an unsigned fallback.

Private keys and keyring paths must not appear in the packet, chat, logs,
diagnostics, exports, or receipts.

## Approve the Exact Commit

The final manual preview binds all of these values together:

- Candidate tree and parent object.
- Full commit message.
- Author and committer identities, timestamp, and timezone.
- Pinned signer and public fingerprint.
- Exact AgentMage task branch and expected old object.
- Current preservation manifest and unchanged user index.

Approval expires after at most ten minutes and cannot survive any changed field.
It does not authorize a push, pull request, merge, release, deployment, amend,
reset, discard, force operation, or history rewrite.

## Verify the Receipt

A successful local-commit receipt must show:

- One verified signed commit with the exact tree, parent, message, identities,
  signer, and task branch.
- A compare-and-swap branch update from the approved parent to that commit.
- An unchanged user index, unchanged remotes, and disabled repository execution
  surfaces.
- No network effect and no publication authority.
- Exact before and after preservation-manifest identities.

Failure, timeout, cancellation, stale state, signature mismatch, branch movement,
or uncertain cleanup is not success. Preserve the owned worktree and evidence for
inspection; do not retry or clean up by reset, discard, force, or hidden stash.

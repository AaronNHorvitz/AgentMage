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

The pinned fingerprint identifies the exact key that must sign, whether a primary
key or a signing subkey. A primary-key selection does not delegate to a newer
subkey. If that exact key cannot sign, creation fails without trying another key.

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

## Dormant Linux candidate construction

The candidate builder now dispatches its fixed Git steps through the offline
worker supervisor. It exposes source read-only, exact selected metadata writable,
and a new private candidate directory writable. Other owned worktrees and scratch
siblings are not projected. Its temporary-index identity names `index` inside
`candidate-index-<build-id>`; stale flat-index plans and occupied directories refuse.
Postimages are sealed memory input, and each resulting blob must match those
exact bytes. A single thirty-second deadline covers the candidate sequence.

Incomplete supervision, changed directory identities or unknown scratch contents
prevent successful cleanup. Such scratch is retained for explicit recovery;
reusing the same build identity does not replace it. The user index, source and
refs must still satisfy the existing complete preservation reconciliation.

This adapter remains dormant: there is no API or coordinator activation and a
review packet still grants no build authority. The direct signing runner also
remains inactive pending its own confinement and exact approval composition.
Synthetic tests and mount experiments do not establish native admission or model
acceptance; the lane's executable trust and user-bus refusals remain unchanged.

## Dormant signed-commit I/O

Branch lookup, raw commit reads and the final task-branch compare-and-swap now
use the same closed offline supervisor. An immutable kernel-verified plan binds
the held roots, branch, expected parent and returned commit object. These three
steps run in order within one thirty-second deadline, including time spent
signing between them. An error ends the session. Source stays read-only; exact
selected metadata and the owned management root retain their existing writable
projections. No direct fallback runs these three operations.

The content verifier requires exact singleton headers and message bytes. An
additional parent, identity, tree or other header refuses verification. Signature
headers are bound to the object format: `gpgsig` for SHA-1 and `gpgsig-sha256`
for SHA-256. The other header, duplicate headers and mixed object lengths refuse.
Signature verification requires one complete pinned fingerprint in a GnuPG status record;
a substring in prose cannot suffice, and failure/revocation/expiry records refuse.
Signed stdout and stderr remain separate, both signer configuration keys are
pinned, and replacement objects are disabled. Sealed memory input replaces the
parent's blocking message-pipe write.
Message checks preserve the exact approved stdin bytes, including the presence
or absence of final newlines. A failed raw read or content check stops before
signature verification. After worker dispatch or signing begins, failures report
failed execution and unverified cleanup. Last-observation manifest fields in a
failed result do not establish preserved postflight state or a successful receipt.

External signer inspection and signed creation remain direct subprocesses in
this dormant adapter. Their complete confinement and descendant cleanup are
still prerequisites for activation. Their output capture
now uses bounded nonblocking reads with the same four-MiB ceiling on each stream.
Cancellation and the original deadline remain observable after the immediate
child exits, including when another process holds an inherited output pipe.
Signer inspection shares one thirty-second deadline across listing and export;
signed creation retains the original commit-session deadline.
Failure discards partial output and allows at most half a second to kill and
reap the exact owned child. An unreaped child remains retained and refuses later
direct capture attempts; a busy or poisoned capture owner also refuses.
A retained child reports `linux.local_commit.cleanup.uncertain` through both
inspection and signing callers. Signed creation with that
failure reports `linux.git.commit.signer_cleanup_uncertain`, failed execution
and unverified cleanup. It cannot produce a successful commit receipt or
proceed to the task-branch update. Other capture failures keep their existing
redacted error classes. No diagnostic includes child output or keyring paths.

This bounds capture and immediate-child teardown, not descendant lifetime.
An open-pipe failure closes the local readers without proving that the remaining
writer died. No capture result establishes sandbox admission or whole-unit
cleanup. Actual local child/pipe tests, synthetic worker results, parser fixtures
and installed-Git message tests establish component behavior only. Disposable
signatures traverse the production direct signing capture method; key listing
and export traverse the production inspection helper. Verification uses the
public-only payload reconstruction and fixed GnuPG options directly in these
cryptographic component tests. Those tests supply synthetic private artifact fields. They
do not pass the product's executable trust checks or exercise approval and
preservation composition. Native workflow, signer confinement, independent reviewer/artifact provenance, exact
approval composition and measured model acceptance remain open.

## Offline signature verification

The dormant adapter now verifies signatures through a standalone GnuPG worker
in the existing offline supervisor. It no longer launches Git verification
directly. The worker receives exactly three sealed public-material descriptors:
the inspected public-key export, detached signature and exact signed content.
The public-key bytes must match the pinned inspection digest. Signature extraction
first checks the entire raw commit against the approved plan, including the
object-format-specific signature header and all message bytes.

No repository, private keyring, host home, agent socket or user configuration is
projected. The worker has a private temporary home, fixed options disabling option
files, agent startup and key retrieval, and the existing socket-denying seccomp
policy. The verified executables, original commit deadline, resource limits,
bounded output and whole-unit descendant cleanup remain under the same supervisor.
Only a successful complete result with no stdout and the exact pinned signing
fingerprint can permit the next task-branch step. A verification failure or
uncertain cleanup cannot update the task branch or produce a successful receipt.

Synthetic supervisor tests prove result validation and launch construction, not
native admission. Disposable installed-Git signatures test both object formats,
exact primary/subkey selection and message reconstruction against the worker's
public-only GnuPG options. The complete native workflow remains unverified in the
lane. Signer inspection and signed creation still need confinement before any
activation; this change does not grant commit or push authority.

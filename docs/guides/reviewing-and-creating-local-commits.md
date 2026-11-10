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

Setup opens the candidate directory relative to the held parent, without
following symlinks, before capturing the worker scope. The captured directory
must match that descriptor's device and inode; both scopes are revalidated
before returning the session. A failed setup removes only an empty directory
that still matches the held identity and private permissions. Unknown entries,
replaced roots or directories, changed permissions, or an unsuccessful initial
descriptor open retain scratch. This is conservative rollback, not an atomic
transaction against other processes running as the same user.

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

Signed creation remains a direct subprocess in this dormant adapter. Its
complete confinement, usable private-key access and descendant cleanup are
still prerequisites for activation. Its output capture
now uses bounded nonblocking reads with the same four-MiB ceiling on each stream.
Cancellation and the original deadline remain observable after the immediate
child exits, including when another process holds an inherited output pipe.
Public identity inspection uses the offline worker described below; signed
creation retains the original commit-session deadline.
Failure discards partial output and allows at most half a second to kill and
reap the exact owned child. An unreaped child remains retained and refuses later
direct capture attempts; a busy or poisoned capture owner also refuses.
A retained child reports `linux.local_commit.cleanup.uncertain` through both
the test-only direct inspection helper and production signing caller. Signed creation with that
failure reports `linux.git.commit.signer_cleanup_uncertain`, failed execution
and unverified cleanup. It cannot produce a successful commit receipt or
proceed to the task-branch update. Other capture failures keep their existing
redacted error classes. No diagnostic includes child output or keyring paths.

This bounds capture and immediate-child teardown, not descendant lifetime.
An open-pipe failure closes the local readers without proving that the remaining
writer died. No capture result establishes sandbox admission or whole-unit
cleanup. Actual local child/pipe tests, synthetic worker results, parser fixtures
and installed-Git message tests establish component behavior only. Disposable
signatures traverse the production direct signing capture method; their direct
key listing and export now use a test-only fixture helper. Verification uses the
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
Machine status uses stdout exclusively (`--status-fd=1`); stderr contains bounded
diagnostics and cannot supply a verification verdict. Every stdout line must be
a complete machine-status record. Both streams must have consistent lengths and
digests. Only a successful complete result with the exact pinned signing
fingerprint can permit the next task-branch step. A verification failure or
uncertain cleanup cannot update the task branch or produce a successful receipt.

Synthetic supervisor tests prove result validation and launch construction, not
native admission. Disposable installed-Git signatures test both object formats,
exact primary/subkey selection and message reconstruction against the worker's
public-only GnuPG options. The complete native workflow remains unverified in the
lane. Signed creation still needs confinement and usable private-key access
before any activation; this change does not grant commit or push authority.

## Offline public identity inspection

The dormant signer constructor pins public identity through offline listing and
export workers. Only a bounded, sealed snapshot of `pubring.kbx` (or
`pubring.gpg` when the first name is absent) enters either worker. Capture holds
the approved private keyring directory and opens the public database relative
to it without following symlinks. Special files, unsafe ownership or modes,
empty files, changed metadata and databases larger than four MiB refuse capture.
This is a bounded public-file snapshot, not an atomic transaction against
concurrent same-user edits. Homes containing `public-keys.d` or `common.conf`
are refused, including dangling symlinks and empty configuration. This
conservatively excludes common configuration instead of interpreting it, so a
legacy database left after keybox-daemon migration cannot qualify through this
capture path. Markers are rechecked before and after sealing.

Neither worker receives the live keyring directory, private keys, option files,
trust database or host agent sockets. Fixed GnuPG options prevent option-file
loading, agent startup, key retrieval and use of a default keyring. Listing must
identify exactly the selected fingerprint on its own public key/subkey record,
with that key's own signing capability, no revocation, disablement or expiry,
and a current primary key. Creation must be an unsigned decimal timestamp;
expiry may be empty or decimal zero for no expiry. A selected identity or its
primary created in the future, or expiring before creation, is refused.
Designated revoker records may precede a primary fingerprint; the revoker
fingerprint never qualifies the signer. A fingerprint in a user ID or diagnostic
cannot qualify. Export follows only a successful eligible listing, over the same sealed
snapshot. The inspection digest binds the snapshot, listing, public export and
fingerprint. Bounded stderr remains diagnostic only.

The existing supervisor requires root-owned executable admission and whole-unit
cleanup for each worker, with the original thirty-second total deadline, resource
bounds, output checks and socket-denying filter. No direct inspection fallback
exists in production. `inspect_cancellable` uses the caller's cancellation token
through public capture and both workers, including checks between bounded
64-KiB reads and before returning a pin. The legacy synchronous constructor
delegates with an internal token. Cancellation returns no signer; an individual
filesystem operation is not made interruptible by these checks.

A public pin does **not** establish private-key possession or signing availability.
Usable private signing and confined signed creation remain prerequisites for
activation. A disposable real-GnuPG component test exercises the fixed public-only
options without an agent; synthetic worker tests cover the dispatch and refusal
contracts. Neither establishes native sandbox admission or a complete approved
commit workflow. Candidate construction, independent reviewer/artifact authority,
approval composition, approved commit/push and model measurements remain open.

## Dormant socket-free private signing primitive

Public inspection now also retains the keygrip attached to the selected public
fingerprint. It comes from the sealed listing, whose digest is already part of
the inspection evidence. A missing, malformed or duplicate selected keygrip
refuses private use without disqualifying the public identity.

A separate internal primitive captures only that keygrip's private file through
held owner-private directories. It refuses links, special files, unsafe modes,
empty or oversized input, changed metadata and directory replacement. The limit
is 128 KiB, with cancellation checkpoints around 16-KiB reads and sealing. The
temporary heap buffer is wiped. This does not guarantee atomic capture against
same-user edits, locked memory or erasure of kernel storage.

A foreground agent receives the sealed selected file and fixed commands through
the existing supervisor. The socket-denying policy stays unchanged. There is no
host socket, live private directory, repository or user configuration projection,
no shell, no pinentry program, no smartcard daemon and no external passphrase
cache. Protected or externally backed keys can fail; there is no host fallback.
Exactly two payloads, held installed artifacts, resource limits, separate output
bounds, cancellation, deadlines and whole-unit cleanup remain mandatory.

The primitive returns only an unverified signature expression after a complete,
phase-ordered successful response. Errors, inquiries, partial output, mismatched
stream digests, unsuccessful exit or uncertain cleanup refuse the result.
This is not yet an OpenPGP commit signature or an activated capability. Packet
construction, verification, native admission and approval composition remain
open; the existing signed-creation adapter remains dormant. Disposable agent
fixtures prove the primitive's protocol only, not a complete approved workflow.

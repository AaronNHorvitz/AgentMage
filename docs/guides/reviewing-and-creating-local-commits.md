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

Both kernel commit planners and the live-owner candidate planner refuse every
unresolved medium, high or critical finding in the complete normalized review.
Low confidence and duplicate suppression change presentation; they do not resolve
a finding. The existing packet count describes visible high/critical findings
only and is not an admission decision. Informational and low findings remain
inspectable, including their suppression evidence. Packet integrity is checked
before this policy is applied. The live-owner path refuses a blocked packet
before held-root inspection, source reads or offline Git observation.

This check cannot authenticate a reviewer or prove that a caller supplied all
findings. Reviewer and artifact provenance, separate exact approval and native
admission remain prerequisites for executable candidate/commit composition.

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

## Commit approval timing in the internal driver

The internal signed-commit driver takes a trusted host clock rather than a saved
construction timestamp. It samples that clock at construction and again at
dispatch, before issuing its executor permit. The complete review packet and
manual approval must still match, and both the approval and consumed grant must
be live at dispatch. A missing clock, rollback before construction, time before
grant consumption, or either exclusive expiry refuses without invoking the
executor. Cancellation retains its separate refusal. Post-effect reconciliation
uses the checked launch timestamp; this does not renew or replay authority.

This driver is still dormant library composition. Its component tests use real
kernel grant consumption with synthetic held objects, consent and executor
results; they establish no native commit, trusted approval channel or model
result. Full original-owner approval composition, trusted checker and artifact
integration, recovery, native admission and approved push remain open.

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
review packet still grants no build authority. Signed creation uses the confined
composition described below; the old direct runner is test-only.
Synthetic tests and mount experiments do not establish native admission or model
acceptance; the lane's executable trust and user-bus refusals remain unchanged.

## Dormant signed-commit I/O

Branch lookup, verified commit storage, raw commit reads and the final
task-branch compare-and-swap use the same closed offline supervisor. An immutable
kernel-verified plan binds the held roots, branch, expected parent and exact
commit object. These four steps run in order within one thirty-second deadline,
including time spent signing between them. An error ends the session. Source
stays read-only; exact selected metadata and the owned management root retain their existing writable
projections. No direct fallback runs these operations.

The content verifier requires exact singleton headers and message bytes. An
additional parent, identity, tree or other header refuses verification. Signature
headers are bound to the object format: `gpgsig` for SHA-1 and `gpgsig-sha256`
for SHA-256. The other header, duplicate headers and mixed object lengths refuse.
Signature verification requires one complete pinned fingerprint in a GnuPG status record;
a substring in prose cannot suffice, and failure/revocation/expiry records refuse.
Stdout and stderr remain separate and replacement objects are disabled. Signed
bytes enter Git on sealed memory input; Git never launches a signing program.
Message checks preserve the exact approved stdin bytes, including the presence
or absence of final newlines. A failed raw read or content check stops before
signature verification. After worker dispatch or signing begins, failures report
failed execution and unverified cleanup. Last-observation manifest fields in a
failed result do not establish preserved postflight state or a successful receipt.

The historical direct signed-creation runner now exists only in test builds.
Its bounded immediate-child capture, inherited-pipe refusal and retained-child
cleanup diagnostics remain component controls. They do not establish descendant
termination, sandbox admission or approval authority. Disposable direct-signing
fixtures use synthetic artifact fields, rather than the production trust and
approval boundary. The dormant executor instead uses the socket-free signing
and storage composition below. Native workflow admission, original independent
reviewer/artifact provenance, exact product approval composition, durable
recovery and measured model acceptance remain open.

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
lane. The later confined signing/storage composition below still needs native
admission and complete approval provenance before activation; no commit or push
authority is granted by verification.

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
Native admission of usable private signing and the composed signed-creation
workflow remains a prerequisite for activation. A disposable real-GnuPG
component test exercises the fixed public-only options without an agent; synthetic worker tests cover the dispatch and refusal
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

Signature data escapes require exactly two ASCII hexadecimal digits after
percent; signed, incomplete and cross-record escapes refuse. Uppercase and
lowercase hex preserve the same bytes, including encoded NUL, line breaks and
percent. Decoded bytes remain unverified until packet assembly and independent
signature verification succeed.

The primitive returns only an unverified signature expression after a complete,
phase-ordered successful response. Errors, inquiries, partial output, mismatched
stream digests, unsuccessful exit or uncertain cleanup refuse the result.
This primitive alone is not an OpenPGP commit signature or an activated
capability. The assembler and executor composition below add packet construction
and verification; native admission and complete product approval composition
remain open. The signed-creation adapter remains dormant. Disposable agent
fixtures prove the primitive's protocol only, not a complete approved workflow.

## Dormant RSA signature assembly

An internal assembler now prepares exact unsigned commit bytes from a verified
plan, preserving every message byte and using the correct signature header for
each Git object format. It constructs a v4 binary-document signature packet
with SHA-256 and RSA only. Unsupported algorithms and malformed signature
expressions refuse assembly; no fallback is selected. Modulus-width unsigned
RSA values may start with zero octets: the bounded parser strips those octets
before forming the minimal packet integer and still refuses an all-zero value.
The approved timestamp must fit the signature's unsigned 32-bit time field. Author and committer
identities cannot contain angle brackets, names cannot have leading/trailing
whitespace and email identities cannot contain whitespace. Invalid identities
are refused before signing; accepted bytes are never silently normalized.

The dormant composition sends the complete signature digest to the socket-free
primitive, assembles armor, checks the reconstructed signed content, and calls
the public-only offline verifier before returning an immutable verified commit.
Full-fingerprint verification supplies identity; the packet's issuer key ID is advisory. No
product route or permission is added. Disposable RSA primary/subkey signatures
and tamper rejection are component evidence, not a native approved coding workflow.

## Confined storage in the dormant executor

The existing dormant executor now composes that verified commit with the closed
Git session. Direct signed creation is compiled only for disposable tests. The
internal verified value binds one complete plan; saved JSON or caller-supplied
bytes cannot replace it, and it grants no approval authority.

The session checks the task branch's expected parent, writes the sealed signed
bytes without filters, requires the exact locally calculated commit identifier,
and reads back the same object byte for byte. Only then can the original exact
task-branch compare-and-swap follow. Another public-only signature verification
runs between readback and the ref update. One existing deadline and the same
held roots cover the session; the Git worker has no signer, private keyring,
credential or network projection. Failed, cancelled, incomplete or inconsistent
results terminate the session. There is no direct retry or signing fallback.

A stored object may remain unreachable after failure. Ref updates use the
expected-parent comparison, and the original kernel permit, preflight and final
preservation reconciliation remain required. No rollback or successful receipt
is inferred from a partial effect. This is internal library composition without
an HTTP, CLI or MCP commit command. Native artifact admission, complete product
approval/reviewer/artifact provenance, durable recovery and measured coding
acceptance remain open, as do T-303/T-306 and approved push.

## Executor sequence coverage

The dormant executor checks cancellation before and after preservation preflight.
It checks the original token and commit I/O deadline between every subsequent
stage and after postflight. A stop prevents the next stage or successful receipt,
including when the task branch has already changed. It does not imply rollback.
Preservation collection retains its existing bounded workers; these checkpoints
do not interrupt an individual collection or impose the I/O deadline within it.

A private execution seam now exercises the dormant adapter's shared sequence:
branch observation, independently verified signing, sealed object write, exact
readback, content and signature verification, task-branch compare-and-swap and
postflight observation. Every failed stage stops without retry; no failure
returns a commit identity or verified cleanup. The native wrapper still uses
the original kernel permit, preflight checks, closed I/O owner and one deadline.
Only module-private tests can supply synthetic outcomes. These tests do not
establish native admission, authenticated reviewer or artifact provenance,
product approval composition, durable recovery, model scores or approved push.

## Bind the review read selection

An internal caller obtains consent for complete original-repository and owned
worktree inspection, including metadata and offline Git reads, using the two
root identities displayed by the existing apply inspection. It then requests
`consented_commit_review` with those exact identities. The constructor refuses
mismatches before filesystem inspection; a matching selection still requires
held-root revalidation. The returned borrowed view exposes original write
projection and inert candidate planning. Neither operation can be called
unscoped from outside the repository owner.

Each use retains the existing root and source checks. Replacement directories
are refused and retained. The borrow prevents effects or worktree replacement
through the same owner while the view is live; it cannot prevent external
filesystem changes. Root digests bind selection, not consent authentication,
independent review, artifact provenance or permission to execute a candidate.
No product route or new execution authority follows from this view.

The earlier worktree edit selection now follows the same correlation rule:
`consented_created_sources` requires the worktree root displayed by
`inspect_apply` and the exact relative paths covered by read consent. A wrong
root refuses before path inspection or source reads. The uncorrelated selector
is internal to the owner. This operation reads only selected worktree sources;
it does not inspect the original checkout or Git metadata and needs no consent
for those separate reads. Existing file-count, byte, path and native-resolution
bounds still apply, and model disclosure requires separate authority.

A returned source scope retains its original directory descriptor and exact
path allowlist. It may still read a displaced original directory; it cannot
adopt a replacement at the same pathname. Preparing an edit rechecks the live
owner and refuses a foreign, replaced or stale worktree. Selection supplies no
write authority: the complete edit preview and separate exact approval remain
required. Root digests correlate the choice and do not authenticate consent.
The existing trusted host flows derive that root from their continuously retained
native owner under their prior-read-consent contract. They do not introduce a
new person-choice challenge or independently authenticate the prior consent.

## Original-owner checker connection

A trusted host can now call `verify_candidate_tree` on the existing selected
`LinuxCodingCommitReview` view with its explicitly installed
`LinuxCodingCandidateReviewPort`. There is no default accepting implementation.
Select this implementation outside model/tool input. It must independently check
the exact complete packet and task criteria, all visible and suppressed findings,
the complete original test history and trusted capture/scanner evidence for every
artifact claim. An empty findings list, matching digests or a test pass is
insufficient. The interface cannot authenticate the installed checker itself.

The checker receives borrowed references to the exact packet and checked inert
plan, the original native Git-creation record and original validation records.
A rejected or unavailable review stops immediately. A successful review repeats
all original-owner checks and compares the complete Git observations across the
checker call, including linked metadata and task inventory absent from the plan.
Cancellation is checked around observations and review. The host must bound
checker execution; an individual filesystem call or blocking checker cannot be
interrupted by these checkpoints.

The returned `LinuxCodingCheckedCandidate` is a read-only process value borrowing
the original owner and packet. It has no public constructor, cloning or receipt
restoration. Its plan is still inert and can become stale after return. No
executor accepts it and it authorizes no Git effect. The old `plan_candidate_tree`
method remains explanatory evidence only. No API, MCP or terminal route is added;
installation of a real independent checker, artifact provenance, native admission
and complete product approval composition remain open.

## Retain the complete packet through commit preparation

`record_reviewed_manual_commit_approval` and
`verify_reviewed_manual_commit_approval` require the complete typed packet.
They check packet integrity, all visible and suppressed material findings and
the exact packet, repository, parent, logical group and proposed message against
the commit plan. Rehashing a caller-created plan cannot replace these checks.
The older manual-receipt primitives check integrity and exact approval fields
only; they do not inspect a review packet or authenticate person consent.

`PreparedLocalCommit::new` now requires and owns that complete packet. It
cannot be prepared from a bare packet hash. The dormant driver repeats the
complete check at construction and immediately before passing its launch
permit to the executor. The request bytes still contain the exact plan and
approval digest; the plan's packet digest binds the retained packet. Debug
formatting omits the packet and its content. Existing expiry, held-object,
consumed-grant, exact request, cancellation and preservation checks remain.

This supplies packet admission at the dormant kernel boundary, not independent
reviewer authentication or a product commit command. It does not authenticate
candidate construction, signer capture, artifact capture/scanning, the original
owner or the manual approval channel. Those remain mandatory for the eventual
product composition. No API, MCP, CLI or native execution is activated, and
there is no commit or push authority from these values alone.

## Exact source pairs for the trusted checker

`LinuxCodingCandidateReviewEvidence::sources()` provides an ordered source pair
for every file in the complete packet, including files outside the selected
logical commit group. Each pair exposes its original operation, relative path,
immutable-base bytes, current owned-worktree bytes and candidate Git mode.
An absent base is `None`; an existing empty base is `Some(&[])`. New files retain
their separately approved mode. No source bytes appear in debug output or a
serialized receipt, and source pairs have no public constructor.

The base bytes come from the existing bounded offline Git collector, with its
original object-identity, path and size checks. Current source reads use the
existing held worktree scope. Both contents must match their exact packet hashes
before the checker is called. All complete pre/post observations also compare
the retained base bytes; successful review still needs the original full source,
mode, validation and ownership rechecks. This is bounded observation, not an
atomic lease against concurrent filesystem changes.

These bytes are untrusted local input. An installed checker must obtain separate
authority before sending them to any model or service; the two-root read selection
does not grant disclosure. This supplies review material, not an independent
checker implementation, artifact/scanner provenance, person approval or a Git
effect. The checked result remains inert, and T-303/T-306 remain open.

## Packet-bound result reconciliation

`reconcile_local_commit` takes `PreparedLocalCommit`, retaining the complete
packet and exact manual receipt through result reconciliation. The former raw
plan-and-approval signature is removed. It rechecks packet integrity, exhaustive
finding severity admission, exact packet/plan bindings and approval at the
checked launch time before inspecting the result. Adding a severity requires an
explicit admission decision at compilation. Existing postcondition checks still
require exact signed content, task-branch compare-and-swap, unchanged user index
and remotes, preservation, disabled repository execution surfaces and cleanup.

The dormant driver supplies its own prepared value and checked launch time. It
returns no success receipt for an unproved platform result. A post-effect check
does not renew authority or authorize retry. Public callers still supply their
own process observations; this function cannot authenticate a checker, signer,
executor, artifact scanner or approval channel. A serialized receipt and its
public fields do not establish those provenances or grant effect authority.

The Linux component tests now pass complete synthetic packet input through this
same interface. Their synthetic packet, creation, validation, consent and process
observations remain component fixtures. Empty-base source coverage separately
asserts an existing empty file is `Some(&[])`, and refuses an absent base before
calling the checker. Its internal held-target fixture does not admit empty files
through the ordinary source selector, which still refuses empty content. Neither
observation establishes native admission or a real model coding result. T-303/T-306 remain open.

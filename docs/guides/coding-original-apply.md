# Apply a reviewed worktree change to the original checkout

The existing local coding repository owner can prepare a complete diff from its
successfully edited worktree and apply it to the original checkout after a new,
exact approval. This is a local API other programs can use, with a terminal
client over the same routes. T-303 and T-306 remain open. See
[Decision 0156](../decisions/0156-original-checkout-apply.md) and
[creation transfer](../decisions/0160-created-file-original-transfer.md).

Start `serve-coding`, approve [worktree creation](coding-worktrees.md), and make
separately approved [owned edits](coding-owned-edits.md). A path becomes eligible
only after the existing native writer reports a committed worktree edit. Read
selection, a proposal, a caller-supplied receipt and an arbitrary checkout path
cannot establish eligibility. Nothing is automatically applied to the original.

## Terminal workflow

Each invocation starts with
`agentmage-code <loopback-ip:port> <private-auth-directory>`:

```sh
agentmage-code <loopback-ip:port> <private-auth-directory> original
agentmage-code <loopback-ip:port> <private-auth-directory> original-prepare <original-repository> <created-worktree> '<task>' <paths...>
agentmage-code <loopback-ip:port> <private-auth-directory> original
agentmage-code <loopback-ip:port> <private-auth-directory> original-apply <original-repository>
```

Preparation verifies both chosen roots against the owner's held identities,
shows the paths and task, and requires `read <original-root-digest>
<worktree-root-digest>` as one complete line in the controlling terminal. The
request sends those exact two displayed identities. The server refuses
`code.apply.read-scope` if either differs from its current held pair, before
inspecting either root or reading a source. A different control client creating
a worktree during the prompt cannot retarget that read consent. Inspect the
current pair and give fresh consent after a mismatch; do not automatically retry.
Preparation reads the selected worktree files and original preimages for edits.
For verified creations, it instead observes bounded original parent-directory
names and an absent destination, binding each observed parent identity. The terminal
read confirmation discloses these observations. Preparation writes nothing.
The complete preview carries both root identities, creation receipt digest,
complete kernel diff or creation contents and mode, plan/change-set and preview
digests, and expiry.

Application displays the complete escaped preview and requires
`apply-original <outer-preview-digest>`. The client verifies the original root
before and after consent. For a creation-only preview, the confirmation
explicitly describes creating absent files with the displayed complete contents
and destination modes, refusal of existing destinations, and the requirement
that parent directories already exist. The preview and approval digest remain
unchanged; this wording grants no additional operation. It sends the decision
once. Piped input, an incorrect
answer or a substituted root cannot approve a write. `original-deny` consumes
the exact pending preview without writing. An error or lost reply never causes
an automatic resend. Inspect the latest result and disk before another action.

## Versioned local API

Every route requires the control token and the existing loopback, origin and
HTTP framing protections. Requests are exact objects: duplicate, positional and
unknown fields refuse. A model never receives a control credential or chooses
its own approval.

| Route | Body and result |
| --- | --- |
| `GET /v1/code/repository/apply` | Pending complete preview, latest process result, root identities, eligible paths and `available_changes` (`path`, `kind`, `creation_mode`) |
| `POST /v1/code/repository/apply/prepare` | `paths`, `task`, `approve_read: true`, `expected_repository_root_sha256`, `expected_worktree_root_sha256`; inert complete original-checkout diff |
| `POST /v1/code/repository/apply/decide` | `preview_sha256`, `approve`; consumes one exact decision |

Both expected identities are required strings obtained from inspection and bound
to the caller's read confirmation. They are digests, never filesystem paths.
Old preparation bodies without them refuse rather than silently selecting the
current roots. The body accepts no root path, patch, model answer or creation
receipt. The owner selects both roots from its successful creation; an expected
identity can only restrict that selection. A control token authenticates
the trusted caller; it is not a kernel grant. Only the separate approved decision
can issue and consume the existing exact native write grant.

## Drift, bounds and recovery

Up to sixteen successfully edited paths retain their first preimage and most
recent committed postimage across repeated edits. Preparation requires the
original to equal that first preimage and the worktree to equal the last
postimage. Existing selected-file and aggregate byte limits apply. This can
refuse an original with pre-existing changes or a base that differs from its
current checkout. It does not merge human work. Choose a fresh worktree or
resolve the changes yourself before requesting a new diff.

The owner rechecks creation provenance, both held roots and selected worktree
postimages at decision time. The native writer rechecks every original preimage
before any replacement and reconciles partial failures. Symlinks, hardlinks,
mount crossings, metadata and deletion retain their existing refusals.
Unrelated original files and the Git index are outside the write scope.

A new file is eligible only after creation through the live coordinator's
kernel-authorized native creation driver, followed by successful terminal
publication. A manually added or merely untracked file is ineligible. Later
successful edits preserve this absent-before provenance and the native creation
mode. The existing API and terminal commands then show its complete escaped
contents and mode in a controlled-filesystem preview. Original destinations must
remain absent, including empty files, directories and case aliases. Parents must
already exist. The native transaction revalidates all parents, names and absence
before dispatch and uses its existing no-overwrite creation and reconciliation.
The normal UTF-8, nonempty, NUL-free and 128 KiB aggregate source limits apply.

Choose either edits or creations in one preview; a mixed selection refuses
`code.apply.mixed-kinds`. This prevents a partial two-transaction application.
Use `available_changes` to distinguish the kinds, then prepare and approve them
separately. Creation previews list `project-tests` as an unexecuted follow-up;
its result entry explicitly requires a separate command grant. The transfer
executes no project test. Creations and edits share the sixteen-path lineage
limit. A transferred creation becomes an ordinary existing-file edit on its next
successful worktree change.

A foreign digest leaves the preview pending. Denial, expiry, clock rollback,
matching drift refusal and dispatch consume it. A later worktree edit dispatch
invalidates pending original approval. An original transfer dispatch invalidates
an older pending worktree edit. Successful transfer removes the selected paths'
eligibility; the next successful worktree edit starts a fresh chain. A worktree
write without a committed result removes that dispatch's paths from eligibility.

The bounded native write cannot be interrupted midway by API cancellation.
Results and ownership are process-only; the write result contains native
transaction receipts, but this path does not claim a durable coding-run journal.
Restart restores no preview, ownership, lineage or approval. A lost response or
uncertain result requires manual disk inspection, never replay.

The native Git trust blocker still prevents actual worktree creation in this
lane. Tests with synthetic creation provenance exercise real file writes and
approval boundaries, not native Git creation or a real-model coding workflow.
`tested` remains false: applying a diff does not prove tests passed. Commit,
push, durable recovery, configured-endpoint composition and measured parent
acceptance remain open. No public release is authorized.

An explicit [endpoint review service](coding-runtime-api.md#review-the-original-diff-in-the-endpoint-service)
also exposes these three routes after exact terminal release of its strict
coding run. It retains the same original repository and recorded successful edit
lineage; it does not adopt a worktree path or enable other manual tools. The
read consent and exact apply decision described above are still required.

Candidate review also requires complete creation publication. Once native
creation dispatch begins, this live owner refuses candidate review until its
original checked creation proof is published. A failed, uncertain, unsupported
or unpublished creation leaves review unavailable for the lifetime of that
owner, including later edits, successful creations and runtime handoffs. The
diagnostic is `code.commit.creation-provenance`. Inspect the retained files and
use a fresh service and approved worktree; restarting cannot adopt the old one.
This conservative refusal prevents an unrecorded creation from appearing as an
ordinary existing-file edit. It neither removes files nor changes the separate
original-transfer permissions.

The same refusal applies when a later edit retires published creation history
without restoring its complete checked proof. The guard starts before native
dispatch, including edits to other paths that share that history. Failed,
unpublished or rejected completions leave it set across later successful edits,
creations and handoffs. A committed edit can restore creation review only when
the preceding proof was retained and the combined history is complete. Denial,
expiry, stale decisions and abandoned previews do not retire the existing proof.

The internal candidate planner also checks every reviewed first preimage against
the exact immutable creation commit. Dirty starting content or an existing
untracked file cannot become an ordinary base edit. A creation must be absent
in the base; an empty base file counts as occupied. The diagnostic is
`code.commit.base-preimage`. Selected symlinks, submodules, directories and
descendants of non-directory entries refuse. These base reads use the existing
offline linked worker, with replacement objects disabled and unchanged trust,
output limits and per-command deadlines. The final Git observation repeats them
before releasing the inert plan. This is internal library plumbing; no candidate
executor or new terminal/API entry point is available. Reviewer/artifact
provenance and separate Git approvals still remain necessary before execution.

## Continue from a terminal coding run

For an endpoint service explicitly started with original review enabled, use:

```sh
agentmage-code <loopback-ip:port> <private-auth-directory> endpoint-original-apply \
  300 <run-id> <request-digest> <absolute-original-root> <absolute-worktree-root> \
  'Apply the reviewed changes' src/example.py
```

The time allowance, from 1 to 3600 seconds, covers the HTTP operations and
terminal confirmations together. The command first checks the exact retained
controlled-write request, collects its complete terminal event chain and emits
the record to stdout. Keep that output if needed; the command creates no record
file. Its output consumer must keep draining stdout. Record verification checks
the canonical chain, not authenticity or referenced payload contents. A running
or faulted prefix refuses before release. Use the separate inspection commands
for faulted runs; this command cannot restore execution.

Before collecting the record or offering release, the command checks only root
directory metadata. Missing roots, non-directories, symlinked roots or ancestors,
and non-canonical spellings refuse while the run remains held. These checks do
not establish source eligibility or protect against later filesystem changes;
the existing held-root checks still apply before reads and writes.
Pass canonical root spellings, obtainable with `realpath`; an alias to a home
directory can refuse even when it names the intended checkout.

To retain the record in a private file before release, use the saved variant:

```sh
agentmage-code <loopback-ip:port> <private-auth-directory> endpoint-original-apply-saved \
  <absolute-private-record-directory> 300 <run-id> <request-digest> \
  <absolute-original-root> <absolute-worktree-root> \
  'Apply the reviewed changes' src/example.py
```

The existing canonical private directory must have mode 0700. This variant adds
one exact save confirmation before the three decisions below. It creates
`<request-digest>.json` with mode 0600, never replaces an existing name, and
synchronizes the record and directory before offering release. A declined or
failed save stops while the run remains held. Declining release after saving
keeps both the file and run. Inspect the file; another save to that same name
refuses. Use ordinary release or a fresh private directory after inspection.
See [record retention](runtime-endpoint-runs.md#export-and-replay-a-terminal-event-record)
for plaintext, partial-file and offline replay limits. Saved records restore
no execution or approvals, and a cancelled record remains cancelled after a
separately approved successful original transfer.

Three independent terminal decisions follow: release the displayed run, read
the displayed paths from both held roots, and apply the complete original diff.
Release drops the old model ports and event record. Ordinary services discard
repository ownership at release, so use the explicit review service. The command
checks the locally displayed root identities before and after read consent, then
requires the same complete pending preview at the exact apply decision. Source
drift and the original native refusals still apply. There is no automatic retry,
cancellation, cleanup or new model call. After a lost reply, inspect state and
disk; a write may have completed even though the command reports an error.

An explicit HTTP 409 `code.endpoint.original-review-unavailable` refusal reports
`released: true` and `original_review_available: false`: the terminal run is gone
but no repository was published for review. The terminal displays this refusal
and stops. Inspect state and disk rather than repeating release or approval;
the service cannot reconstruct ownership from the exported record.

Declining release keeps the old record. Declining reads or the diff after
release leaves the original owner available for separate inspection; an inert
preview may remain pending. Inspect and resolve it with the ordinary commands:

```sh
agentmage-code <loopback-ip:port> <private-auth-directory> original
```

After a declined read, use `original-prepare` from the terminal workflow above
for fresh two-root read consent, then inspect the resulting preview. After a
declined apply, the existing preview can be inspected and resolved directly.
Use `original-apply <absolute-original-root>` for fresh exact diff approval, or
`original-deny` to consume the preview without writing. Denial retains eligible
changes for a later separately prepared preview. Each decision is consumed once;
none of these commands restores the released run, its events or model ports.
A lost reply still requires disk and state inspection before any new action.

Interrupting a confirmation or exhausting the allowance sends
no later approval. A previously dispatched native write remains non-interruptible.
Applying recorded lineage after cancellation or failure establishes no test pass
or task success. Worktree bytes, unrelated original files and the index remain
under the existing native protections. Commit, push and durable recovery remain
open work.

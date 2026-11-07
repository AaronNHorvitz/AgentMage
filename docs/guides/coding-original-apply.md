# Apply a reviewed worktree change to the original checkout

The existing local coding repository owner can prepare a complete diff from its
successfully edited worktree and apply it to the original checkout after a new,
exact approval. This is a local API other programs can use, with a terminal
client over the same routes. T-303 and T-306 remain open. See
[Decision 0156](../decisions/0156-original-checkout-apply.md).

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
<worktree-root-digest>` as one complete line in the controlling terminal. It
reads only those existing files on both sides and writes nothing. The complete
preview carries both root identities, creation receipt digest, kernel diff,
change-set and preview digests, and expiry.

Application displays the complete escaped preview and requires
`apply-original <outer-preview-digest>`. The client verifies the original root
before and after consent and sends the decision once. Piped input, an incorrect
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
| `GET /v1/code/repository/apply` | Pending complete preview, latest process result, root identities and eligible paths |
| `POST /v1/code/repository/apply/prepare` | `paths`, `task`, `approve_read: true`; inert complete original-checkout diff |
| `POST /v1/code/repository/apply/decide` | `preview_sha256`, `approve`; consumes one exact decision |

The body accepts no root, patch, model answer or creation receipt. The owner
selects both roots from its successful creation. A control token authenticates
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
mount crossings, metadata, new files and deletion retain their existing refusals.
Unrelated original files and the Git index are outside the write scope.

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
push, durable recovery, coordinator composition and measured parent acceptance
remain open. No public release is authorized.

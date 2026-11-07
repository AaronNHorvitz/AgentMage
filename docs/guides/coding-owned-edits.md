# Edits in the service-created worktree

The local API can prepare a strict proposal as a diff in the worktree held after
this service's successful native creation. A separate exact approval writes it
through the existing kernel write grant and native atomic transaction. It cannot
adopt caller paths or receipts. This connects [worktree ownership](coding-worktrees.md)
and [reviewed writes](coding-writes.md); T-303 and T-306 remain open.

Positive native worktree creation remains unmeasured in this lane: the existing
Git executable-trust check refuses before creation. Native-owner component tests
inject explicitly synthetic creation provenance, then write real fixture files.
That proves the edit boundary, not actual Git creation or a working coding agent.

## API and terminal

All routes require the control token, including inspection and preparation.
Ordinary client credentials and model output confer no authority.

| Route | Meaning |
| --- | --- |
| `GET /v1/code/repository/edit` | Process-only pending diff, latest dispatch/result and held worktree identity |
| `POST /v1/code/repository/edit/prepare` | Explicit read consent and strict proposal; returns an inert complete diff |
| `POST /v1/code/repository/edit/decide` | Exact outer preview digest and Boolean decision, consumed once |

The preparation request has these exact fields:

```json
{
  "paths": ["src/calc.py"],
  "expected_sources": [{"path": "src/calc.py", "sha256": "<complete-source-digest>"}],
  "task": "Fix addition",
  "max_message_bytes": 6000,
  "answer": "{\"edits\":[{\"path\":\"src/calc.py\",\"old\":\"return a - b\",\"new\":\"return a + b\"}]}",
  "approve_read": true
}
```

The request carries no root, creation receipt, replacement file content or
execution approval. Only the owner chooses its held created root. The same
strict checker as the selected-workspace route checks every complete source
hash, including unchanged files, before reapplying the exact visible fragments.
Duplicate snapshots, stale hashes, ambiguous fragments, overlaps, unselected
paths, positional objects and unknown fields refuse. The checker validates
edits; it does not attest which model or person authored them.

The outer preview includes `operation: edit-worktree`, the worktree identity,
creation receipt digest, entire kernel `write_preview`, creation/expiry times,
retention and warnings. Its `preview_sha256` hashes that complete body. The
inner kernel preview and change-set digests remain distinct and are included
inside the approved body. A decision sends only:

```json
{"preview_sha256": "<outer-preview-digest>", "approve": true}
```

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> edit-prepare <absolute-worktree> '<strict-change-json>'
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> edit
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> edit-apply <absolute-worktree>
```

Preparation shows the complete escaped request and requires `read <root-sha256>`
on the controlling terminal. Application shows the complete escaped diff and
requires `edit <outer-preview-sha256>`. The terminal verifies the chosen root
before and after consent and sends each decision once. Piped consent, wrong
challenges and root replacement cannot approve effects. `edit-deny` consumes
an exact preview without writing.

For a model proposal, first select exact sources and explicitly approve model
disclosure through [the existing workspace commands](coding-terminal.md). Then:

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> propose-worktree <absolute-worktree> <model> <message-bytes> <output-tokens> '<task>'
```

This uses the unchanged model proposal path. Only its complete checked terminal
proposal can reach owned-edit preparation. The client retains all context hashes
and asks separately for the created worktree read consent. The selected proposal
sources and created sources must have matching complete hashes. Preparation
still writes nothing. Use `edit-apply` for the later independent diff decision.
A cancelled, partial, refused or malformed generation never prepares an edit.

## Scope and retained results

The owner rechecks the held worktree and management directory before selecting
sources, preparing the diff and consuming approval, and again after the native
write transaction. A foreign scope or different creation receipt refuses. The
existing writer checks selected leaf identities and exact preimages around
replacement. Symlinks, mount crossings, hardlinks, Git metadata, new files and
deletions remain refused. Existing selected-file and byte caps apply unchanged.

A successful preparation replaces only the pending edit. Failed preparation
preserves it. Denial consumes no write grant. A matching approved decision takes
the preview before dispatch, even when it later fails. Dispatch invalidates
pending command/validation previews, so an older command approval cannot be
used after the write. The immutable validation definition and spent-attempt
budget survive; writing cannot reset them. A new successfully created worktree
clears the pending edit and latest result.

Results distinguish committed, failed-no-change, restored and uncertain native
writes. `ownership_scope: service-created-worktree` identifies checked ownership;
the inner writer's `isolation_verified` remains false because it does not inspect
Git state. No source correctness, tests or full repository isolation is implied.
`tested` remains false. Replacing a root after dispatch reports uncertainty.

The latest result and pending preview are process-only. The owner records
`dispatching` with unknown effect before invoking the writer, then retains its
result before responding. This is not the canonical durable command journal.
A restart loses edit state and creation ownership and cannot replay it. A lost
reply or crash requires manual inspection; never resend an approval automatically.
The existing bounded write transaction finishes/reconciles after consumption;
API cancellation cannot interrupt it halfway through.

This edit path changes only files in the held worktree. A separate
[original-checkout diff and approval](coding-original-apply.md) can transfer its
committed edits after source checks on both sides. Coordinator composition,
durable coding-run recovery, commit, push and MCP integration remain open. No
model qualification, coding score or parent-row acceptance is established.

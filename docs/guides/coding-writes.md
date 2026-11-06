# Reviewed workspace edits

The local API can turn a strict source proposal into a complete diff, then apply
it after a separate exact approval. Read consent and model disclosure never
authorize writing. This connects the existing kernel shadow-change, write-grant,
policy and native atomic transaction components; it introduces no alternate
filesystem writer. T-303 and T-306 remain open.

This is an explicit selected-workspace edit primitive. **The API does not create
or verify an isolated Git worktree.** Choose a disposable workspace for development
use and review its identity before approving. The [terminal client](coding-terminal.md) connects proposals, previews and typed
approvals over this API. Commands, tests, retries, durable resume/replay, Git
apply/commit/push, MCP and the full task CLI remain separate work. The
native executable trust checks stay enforced; no ownership workaround is enabled.

## Prepare without effects

Retain the `context` event and complete strict JSON answer from a successful
[workspace proposal](coding-workspace.md). Only a terminal checked `proposal`
is eligible; never offer partial or failed generations for approval. An ordinary
client can prepare its diff locally, without any additional model request:

```text
POST /v1/code/workspace/changes
```

```json
{
  "workspace_revision": 1,
  "paths": ["src/calc.py"],
  "expected_sources": [{"path": "src/calc.py", "sha256": "<context-file-digest>"}],
  "task": "Fix add so it adds the numbers",
  "max_message_bytes": 6000,
  "answer": "{\"edits\":[{\"path\":\"src/calc.py\",\"old\":\"return a - b\",\"new\":\"return a + b\"}]}"
}
```

Supply the same task, paths and message-byte ceiling as the proposal request.
`expected_sources` must identify every complete source snapshot in its context,
including sources that were not edited, without duplicates. Current bytes are
read again through the native path adapter and must match those hashes. The
existing deterministic context selector and strict-edit checker run again; the
answer must satisfy the same format, uniqueness, visibility, non-overlap and byte
bounds. The route also accepts independently supplied strict edits: it proves
their validity against these snapshots, not that a particular model authored them.

The native adapter binds each existing target's identity and exact preimage,
builds an inert kernel shadow change and returns its escaped complete before/after
diff. The result includes `preview`, `workspace_revision`, the held root's
`workspace_root_sha256`, `expires_at_epoch_ms`, `applied: false`, `tested: false`
and `isolation_verified: false`. Its plan hash binds the task and root identity.
No write grant is issued during preparation. No source or absolute path is put
in activity records.

There is at most one pending preview. A new valid preparation supersedes it;
invalid preparations preserve it. Scope selection/revocation and every endpoint
settings change drop it. Preview lifetime is two minutes, enforced with both
epoch and monotonic time. It does not survive process restart. Selection's path
and source budgets remain in force; transactions admit at most 16 existing
files and 256 KiB of combined preimages/postimages. New files, deletion, links,
mount crossings and relative `.git` path components are not admitted. UTF-8 and line endings are
checked; programming-language syntax and project tests are not.

## Approve the exact diff

A trusted person-facing control must display the previously chosen root, its
revision and identity, every affected relative path, complete diff and risks.
It must keep the control credential away from the model and ordinary clients.
After the person's explicit decision, send with that **control** credential:

```text
POST /v1/code/workspace/apply
```

```json
{
  "workspace_revision": 1,
  "change_set_sha256": "<preview-change-set-digest>",
  "preview_sha256": "<preview-digest>",
  "approve": true
}
```

The apply request carries no paths, replacements, commands or new scope. Client
credentials get 403. Wrong bindings get 409 without dispatch. `approve: false`
discards the exact pending preview and records `denied`, with no effect. An
accepted approval removes the preview before dispatch, so replay cannot execute
it again. A failure also requires a new preparation and approval; clients must
never retry effects automatically after an ambiguous connection loss.

The kernel derives one exact grant within the selected leaf scopes, checks
policy and current target identity/preimages, consumes that grant once, and
invokes `LinuxAtomicWriteDriver`. That existing driver revalidates paths around
descriptor-relative replacement and reconciles partial failures. No shell or
Git executable is started. Results distinguish `committed`, `failed_no_change`,
`restored` and `uncertain`; kernel transaction errors conservatively report
`effect: "unknown"`. A committed native transaction means verified bytes only;
`tested` and `isolation_verified` remain false. Multi-file application is
reconciled, not a filesystem-wide atomic commit. Existing file modes are retained.

## Cancellation, records and recovery limits

Preparation can be cancelled before its preview is installed. After an apply
decision is consumed, the bounded native transaction finishes and reconciles;
it cannot be interrupted midway. Status reports `cancellable: false` in that
interval and cancel reports `cancellation_requested: false`. Filesystem work is
bounded in bytes, not by a hard wall-clock deadline. A stalled filesystem can
delay completion or shutdown. Model inference retains its existing cancellation.

`GET /v1/code/workspace/changes` returns the pending preview and up to eight
content-free write records. Each has a sequence; `last_sequence` and
`history_truncated` disclose eviction. A dispatch record binds the workspace
revision and exact hashes before any write. Terminal records include the kernel
receipt chain or a fixed failure with unknown effect. Results are retained and
the operation lease is released before the terminal reply, including when the
client disconnects. Scope replacement is refused during a transaction.

**These records are process-only.** A crash loses grants, previews and receipts;
it does not undo writes. This milestone provides no crash-safe audit, recovery
or automatic replay. After a lost response inspect the retained result while
the service still runs. After a crash inspect the actual files and diff manually
before starting a new proposal. Inspect for native-driver
`.agentmage-write-*.tmp` temporary/reconciliation files left by an interrupted
transaction; preserve them until the affected files and outcome are understood.
Durable journal integration remains required
for parent-row acceptance. Undo is a new exact proposal and approval against
current bytes; no implicit reset, restore, stash, checkout or deletion occurs.

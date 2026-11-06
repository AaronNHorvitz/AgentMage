# Approved local worktrees

The local API connects the existing Linux repository executor to exact trusted
approvals and the existing encrypted authority store. This is a non-production
coding-development path. T-303 and T-306 remain open. In this lane, system Git
ownership prevents native admission; the service reports
`linux.repository.git_artifact.invalid` before inspection or creation. Synthetic
platform outcomes establish authority/recovery behavior, not native creation.

## Start with private state

Build the executables and create authentication as in the
[terminal guide](coding-terminal.md). Separately create an empty directory with
mode `0700` on an admitted local filesystem, outside target repositories:

```sh
mkdir -m 700 <private-state-directory>
target/debug/agentmage-local-api serve-coding 127.0.0.1:0 <private-auth-directory> <private-state-directory>
```

Starting `serve-coding` authorizes encrypted state writes there, not repository
commands or effects. Ordinary `serve` preserves prior behavior and refuses the
repository routes with `code.repository.state-required`. Neither command admits
a production platform or establishes installation qualification.

HKDF-SHA256 derives the store key from the control token using a separate
local-coding-state context. Keep that private token unchanged when reopening
state: losing or replacing it prevents decryption. No key file or second database
is introduced. Existing strict-local root inspection, SQLCipher, exclusive owner
and recovery rules apply. Unsafe roots, another live owner and wrong keys refuse
startup. Keep authentication, state and management directories out of model
context. The [API port trust limitation](coding-terminal.md#bounds-and-recovery)
remains required work before an install kit.

## Inspect, then approve creation

Choose a repository with an ordinary `.git` directory and a full immutable base
commit ID. Create another empty mode-`0700` management directory outside the
repository and state. Choose a unique task identifier of at most 64 ASCII
letters, digits, hyphens or underscores:

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> worktree-prepare <absolute-repository> <absolute-management-directory> <task-id> <base-commit>
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> worktree
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> worktree-create <absolute-repository> <absolute-management-directory>
```

Preparation requires a controlling terminal and `inspect <base-commit>` consent
to hardened local Git execution and full-tree reads, including staged, unstaged,
untracked and ignored work. Observations become the content-minimized native
preservation manifest. Inspection creates no branch or worktree and discloses
nothing to a model. Root-owned Git and its hardened environment remain mandatory.
Symlinked roots, linked metadata, overlapping roots and unsafe management
directories refuse. A linked-worktree checkout with a `.git` file is outside this
creation entry point.

The preview contains the immutable base, plan and preservation hashes, root
identities, expiry and `refs/heads/agentmage/tasks/<task-id>`. Separate
`create <preview-sha256>` consent approves creation. The terminal checks both
displayed roots before and after approval. The API revalidates held roots,
metadata, empty destination and the complete preservation manifest. Foreign
digests preserve the preview; denial, expiry and attempted dispatch consume it.
Use `worktree-deny` to deny. Previews expire after two minutes and never return
after restart. Concurrent changes require a new preview and approval.
A failed preparation preserves any older preview; inspect its task and base
before deciding.

Only approved creation durably issues and consumes the exact kernel grant and
checkpoints the authority transaction before the existing executor runs. The
worktree destination is `<absolute-management-directory>/worktree`; native
reconciliation requires preserving the original checkout and index. An existing
destination or task branch is never overwritten. No network, commit, push,
removal or automatic cleanup is authorized.

Creation does not select source. Explicitly select the new worktree through
`select` or `select-model` before proposals or writes. Current write previews
conservatively retain their unverified isolation/test labels: writes are not yet
bound to worktree ownership. Confined tests, failure feedback, retries and durable
coding runs remain open in T-303 and T-306.

## API and recovery

All three routes require the control token and existing local API protections.
Inputs must be objects; unknown, duplicate and positional fields refuse:

| Route | Request |
| --- | --- |
| `GET /v1/code/repository` | No body; process preview and up to 32 canonical authority receipts, newest first |
| `POST /v1/code/repository/prepare` | `root`, `owned_root`, `task_id`, `base_commit`, `approve_inspection: true` |
| `POST /v1/code/repository/decide` | `preview_sha256`, `approve` |

Repository operations serialize with source/model/write operations. They use
existing collector/executor bounds and individual Git timeouts, not a new total
request deadline. Once admitted, inspection and creation are not interruptible
through chat cancellation: status reports `cancellable: false`. An accepted
cancellation before that transition prevents the operation. Slow filesystem
calls can delay shutdown. Crash-safe native-child ownership is not established.

Canonical content-free authority receipts survive restart; complete previews and
specialized repository receipts are not new durable run artifacts. Interrupted
transactions recover conservatively as uncertain, with no restored approval or
automatic retry. This is authority recovery, not coding-run resume/replay.

After a lost reply or interruption, inspect receipts and the actual repository,
task branch and management directory. A refused decision conservatively reports
`effect: unknown`, never permission to resend. A failed preparation reports
`no-worktree-dispatch`, which does not mean no state or inspection activity.
The client sends each consequential request once and requires an observed
successful creation or explicit denial to exit successfully.

# Failure diagnostics to a reviewed repair proposal

`repair-worktree` connects an approved test failure to one model proposal using
the existing [source selection](coding-workspace.md), streaming strict-edit
checker and [owned diff preparation](coding-owned-edits.md). It adds no command
runner, runtime coordinator or automatic loop. T-303 and T-306 remain open.

## Use

First create the worktree through this service, select its exact source paths
with `select-model`, and run the separately approved
[project test definition](coding-validation.md). After a retryable assertion
failure, run:

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> repair-worktree <worktree> <model> 16384 2048 '<original-task>'
```

The controlling terminal displays the exact failed receipt, bounded diagnostics,
held root, selected paths, configured endpoint and settings revision, model and
budgets. Diagnostics may contain sensitive text. Type
`share-failure <feedback-sha256>` to disclose them. This is separate from the
existing selected-source disclosure. The root is checked before and after consent;
the server independently checks current settings, scope and native ownership.

A complete checked answer then requests the existing separate `read <root-sha256>`
consent to prepare an inert diff from current worktree files. This checks every
complete source hash and the exact excerpt visibility used during generation.
It neither writes nor runs a command. Inspect the diff and use `edit-apply`
with its separate exact approval. Prepare and approve the unchanged test
definition again with the exact last failed receipt. No model result can approve
an edit, select a test definition or authorize a rerun.

## Local API

Both routes require the control token. Models and ordinary clients cannot call
them. This is a local API other programs can use.

| Route | Result |
| --- | --- |
| `GET /v1/code/repository/repair` | Inert exact preview for the latest eligible failure in the held service-created worktree |
| `POST /v1/code/repository/repair/proposals` | One streamed proposal after explicit failure-disclosure consent; no write or command |

The POST is a closed object with these required fields:

```json
{
  "workspace_revision": 1,
  "paths": ["calc.py"],
  "model": "configured-model",
  "task": "Fix add so it adds the numbers.",
  "max_message_bytes": 16384,
  "max_tokens": 2048,
  "receipt_sha256": "<latest-failed-receipt-sha256>",
  "feedback_sha256": "<displayed-feedback-sha256>",
  "settings_revision": 1,
  "settings_endpoint": "http://127.0.0.1:43127/v1",
  "approve_failure_disclosure": true
}
```

No caller supplies output, a receipt body, a worktree root or creation proof.
The owner derives failure data from its normalized command result. The existing
kernel rerun planner determines eligibility; unknown, passed, malformed,
uncertain, exhausted and non-retryable results refuse. A caller cannot adopt
another worktree. Source reads must also pass the current workspace selection
and its endpoint-bound model disclosure.

The preview binds the held root, creation receipt, exact failed receipt, test
attempt, output digests and excerpts. Each channel retains at most 1,024 original
bytes: either the whole channel or its first and last 512 bytes. Lossy UTF-8
decoding, omitted bytes and command-capture truncation are explicit. At most
16 failed names of 256 characters are supplied, with total/truncation metadata;
the full normalized result remains bound by its receipt digest. No secret scan
is claimed. Successful results retain no output excerpts.

Diagnostics are JSON in a separate user message. Additional fixed instructions
classify them as untrusted data. They cannot change tools, endpoint or authority.
Their serialized bytes and additional instructions are reserved from the same
2–32 KiB total message ceiling before source selection. At least 2 KiB must remain
for source context or the request refuses. The context event reports the exact
combined `message_bytes`, the source selection budget `source_message_bytes`,
and the exact `repair_feedback`. The terminal validates these before diff
preparation. Output remains capped at 4,096 tokens and 64 KiB of answer text.
Byte bounds do not qualify the backend context size or codec.

## Budgets, recovery and remaining work

Only one admitted model request is allowed per failed receipt. It is consumed
before network admission, including backend errors, invalid answers, cancellation,
disconnects and lost replies. Invalid input, settings/scope mismatch or an
insufficient context budget refuses before consumption. This spends no test
attempt. Edits cannot reset it. A fresh separately approved failed test attempt
can make the next request eligible, within the unchanged three-test-attempt
series. A third failure has no remaining repair/rerun budget.

Preflight encodes the complete three-message request before consuming repair
consent: additional system instructions, selected source context and serialized
diagnostics. Dispatch uses the same message constructor and encoder limits.
Malformed model identities, invalid token reserves and oversized encoded requests
therefore refuse before consumption or network admission.

Generation shares the existing operation lease, status and cancellation. Settings,
scope replacement, writes and commands cannot race it through the API. Source
drift after the snapshot refuses diff preparation rather than overwriting human
work. Never automatically resend a failed request. Bounded excerpts and the
consumption flag are process-only. Restart loses worktree ownership and all these
selections; canonical receipts cannot restore them or replay an action.
Cancelling a repair proposal requires the control token; a client token cannot
consume its single model request by cancelling the operation.

`model_retry_connected: false` remains the legacy declaration that the native
runtime coordinator has no automatic model retry. The additive
`repair_proposal_requires_consent: true` describes this manual proposal path;
`automatic_model_retry` remains false. Native trust checks still refuse positive
worktree/command execution in this lane. Component and synthetic socket/terminal
tests are not an executable native coding campaign or a fresh model score.
[Native command cancellation](coding-commands.md#sandbox-and-results) is now
connected separately; it does not make a cancelled validation repairable.
Automatic repair through the existing host/kernel coordinator, durable coding
runs, cancellation of worktree creation and write effects, compile-failure and
unsupported-runner handling, original-repository apply, approved commit/push,
MCP and measured parent acceptance remain open. Held-out tasks run only at row closes.

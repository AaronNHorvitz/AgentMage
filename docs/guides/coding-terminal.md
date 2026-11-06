# Terminal coding client

`agentmage-code` is a thin terminal client of the
[versioned local API](local-chat-api.md). It connects selected source reads,
streaming [strict proposals](coding-proposals.md), preview preparation and
[exact approved writes](coding-writes.md). It starts no shell, Git, test runner
or model process. T-303 and T-306 remain open: worktree isolation, confined tests,
retries, durable runs, resume/replay, Git operations and MCP are still required.

The separate [approved worktree path](coding-worktrees.md) adds `worktree`,
`worktree-prepare`, `worktree-create` and `worktree-deny` through this same API.
It uses canonical durable authority receipts but has not demonstrated native
creation in this lane. It does not yet bind source writes to worktree ownership
or add project tests, retries or durable coding runs. The separate
[confined command path](coding-commands.md) exposes `command`, `command-prepare`,
`command-run` and `command-deny` with fresh typed execution approval. Native
command execution is not measured in this lane; parsed test results and the
connected retry loop remain open.

Use a disposable workspace. The source-write path edits the explicitly selected
workspace; its approvals are not yet bound to service-created worktree ownership.
Source selection and all approvals are trusted person-facing operations. Never give
this client, its authentication directory or its controlling terminal to a model
or an untrusted command worker.

## Start and choose sources

Build both executables with the pinned toolchain and existing dependencies:

```sh
cargo build --locked --offline -p agentmage-local-api --bins
```

Create the API's private authentication directory and start it as described in
[the API guide](local-chat-api.md). Choose the endpoint and optional key through
its trusted settings controls. The terminal client deliberately has no credential
or endpoint-setting command. It takes a numeric loopback address, including the
port; it does not resolve hostnames, follow redirects or use proxy variables.

For read-only use:

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> select <absolute-workspace-root> src/calc.py tests/test_calc.py
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> read
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> search return
```

The explicit `select` invocation authorizes reading exactly those paths. It
never authorizes model disclosure or writes. Invalid selections preserve the
previous selection. `workspace` shows the selected paths and revision; `revoke`
drops the selection and pending preview. Only the API owns the grant. Neither
selection nor the client stores a persistent workspace configuration.

To additionally consent to model context, use `select-model` with the same root
and paths. The controlling terminal shows the full selection, chosen endpoint
and settings revision. Type the displayed `share <settings-revision>` challenge
exactly. Piped input, a missing terminal, an incorrect response or interruption
cannot approve. The API checks both the displayed revision and exact endpoint while holding
its operation lease; an intervening endpoint change refuses the selection. Later endpoint
changes still revoke model disclosure as before.

To find paths before selecting source, use the metadata-only file map:

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> map <absolute-workspace-root> 'calc tests'
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> map-select <absolute-workspace-root> 'calc tests'
```

Invoking either command authorizes one bounded directory scan of that root.
`map` displays ranked relative paths without reading file content or changing
the current source selection. Ranking matches query terms against path names;
it does not understand file contents or establish semantic relevance. Hidden
objects and generated/dependency trees are omitted. See the
[map contract and limits](coding-workspace.md#metadata-only-file-map).

`map-select` proposes up to sixteen paths with positive path scores. It shows
every selected path, root and endpoint, then requires the same fresh typed
`share` confirmation as `select-model`. Review the selection carefully; use
explicit `select-model` paths when the suggestions are unsuitable. No matches,
decline, invalid source text, or a root/endpoint change refuses selection and
preserves the previous scope. The displayed root identity is checked before
and after confirmation and bound in the selection request. Complete selected
source text becomes available for model context; only `propose` sends it.
Mapping alone authorizes no reads of file content, model disclosure or writes.

## Propose, inspect and approve

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> propose <configured-model> 6000 2048 'Fix add so it adds its arguments'
```

The numbers are the context message-byte ceiling and maximum output tokens.
The API retains its independent source, model and transport limits. Only selected
files enter the existing context selector. Events are printed as JSON values
as they arrive. Only a complete terminal checked proposal can trigger preview
preparation; partial, malformed, truncated or failed output exits unsuccessfully.
The client uses the exact source hashes, task and byte ceiling from that request
and the complete answer. The API reads current sources and checks everything
again. Preparation has no write effects and makes no further model request.
A failure can leave an older pending preview intact; always inspect its intent
and exact diff before choosing an action.

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> review
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> apply <absolute-workspace-root>
```

`apply` fetches the current pending preview and verifies that the supplied root
matches its native path/device/inode identity without reading file content.
The controlling terminal displays the chosen root, full pending preview,
complete escaped before/after text, expiry, exact hashes and risks. Read the
whole review and type `apply <preview-sha256>` exactly. The root is checked again
before the single control-authenticated apply request. The server independently
checks revision and hashes; superseded, expired or consumed approvals cannot
apply. Noncommitted native outcomes produce a failing client exit status.

`deny` consumes the exact current preview with `approve: false`, without file
effects. `status` shows the API lease and cancellation availability. `cancel`
requests cancellation; an acknowledgement alone is not proof of cleanup or of
no effects. The existing API cancellation and transaction semantics apply.

## Bounds and recovery

The client uses the existing protected token loader. Source/proposal/status operations send
only the client token; settings inspection for disclosure, selection, revocation
and explicit apply/deny send the control token. Tokens never enter arguments, output, prompts or model requests.
Mapping also uses the control token. Loopback HTTP does not authenticate the
process holding the API port: a different local account taking a stale or
mistyped port can receive that bearer, including full control authority. Check
the address printed by the running API. Server authentication or an owner-only
local socket with peer checks remains required work before the install kit;
the protected token directory alone does not protect the listening port.
Each request body is capped at 256 KiB. Responses are capped at 4 MiB and 8,192
streaming events, with an 8 KiB header cap. Connection and write timeouts are two
seconds each; response reads have a 180-second total deadline. The client
refuses redirects, compressed/chunked replies, duplicate headers, mismatched
lengths and unfinished NDJSON. No request that generates, prepares or applies
anything is automatically retried. Read-only status polling handles the API's
known terminal-event/lease-release race before preparation.

For HTTP 400 through 599, the client prints the bounded, escaped JSON refusal
on stderr and exits unsuccessfully. Error replies require unambiguous JSON
framing and at most 4 KiB; malformed, streaming or oversized error bodies are
refused without displaying them. This lets `code.write.stale` and a dispatched
`effect: unknown` remain distinguishable. A missing or invalid reply still
requires manual inspection; it never proves absence of file effects. No error
response triggers a retry or a success result.

Terminal output uses JSON escaping and ASCII Unicode escapes, including for
bidi characters. No repository or model bytes are interpreted as terminal
control sequences. The existing preview contains complete escaped whole-file
before/after strings; a line-oriented diff remains backlog. The API's full
before/after binding and independent kernel checks are unchanged.

After a lost apply reply, inspect `review` records and the actual files before
making another proposal. There is no automatic effect retry or rollback. Records
and previews remain process-only, tests and isolation remain unverified, and a
crash can leave writes without a receipt. Follow the
[manual recovery guidance](coding-writes.md#cancellation-records-and-recovery-limits).
No model is downloaded, no GPU reservation is acquired by this client and no
release or installed-product qualification follows from these commands.

## Approved test envelopes

Use `validation-prepare '<definition-json>'`, `validation`,
`validation-run <absolute-worktree>` and `validation-deny` for the separate
[test-envelope API](coding-validation.md). Execution requires the exact
`test <preview-sha256>` challenge on the controlling terminal. The full escaped
preview includes the command, parser, minimum count, attempt and prior receipt.
The client checks the root before and after consent and sends exactly once.
It exits unsuccessfully when command exit is successful but the normalized
validation does not pass. Its output includes both states. Definition confirmation
alone does not launch a command. Neither normal-command consent nor a client token
can approve validation. These commands do not connect automatic model repair.

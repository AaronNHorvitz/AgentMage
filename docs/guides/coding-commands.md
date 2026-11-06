# Approved confined commands

The [worktree owner](coding-worktrees.md) can execute an exact command after a
separate control approval. This is a development API and terminal primitive;
T-303 and T-306 remain open. Native worktree creation and command execution are
not demonstrated in this lane: the existing system-executable trust checks
refuse admission. Synthetic component outcomes are not native test results.

## Scope and preparation

Start `serve-coding` and create a task worktree through its exact approval path.
Only that service instance's most recently observed successful creation supplies
command ownership. A caller cannot supply a path or receipt to adopt an existing
directory. Both the worktree and management directory remain held and are
revalidated before and after execution. Restart loses this process-only
ownership; canonical receipts alone do not restore command authority.

Commands use the existing kernel `CommandSpec`, registry, preparation, exact
grant, durable authority transaction and `CommandEffectDriver`. The Linux owner
always constructs `LinuxBoundedCommandExecutor`; there is no alternate runner or
production injection hook. Embedding programs can construct a person-selected
specification with the kernel's `CommandSpec::seal` and serialize it as JSON.
The CLI accepts that complete sealed JSON, including the executable digest,
literal arguments, environment and bounds. It does not translate shell text,
discover test commands, compute an approval from model output or silently change
a template. This remains a low-level integration surface.

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> command-prepare '<sealed-command-spec-json>'
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> command
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> command-run <absolute-management-directory>/worktree
```

Preparation checks the existing trusted launcher manifest and executable bytes
without launching a command. It preserves any older preview on failure.
The preview displays the complete command, root identity, creation receipt,
request digest, expiry and separate outer approval digest. Review them all.
`command-run` verifies the displayed root, requires exact `run <preview-sha256>`
consent on the controlling terminal, checks the root again and sends once.
`command-deny` denies the current preview. Non-terminal input cannot approve.

Only the control token can use these routes; a source selection or model reply
confers no command authority:

| Route | Request |
| --- | --- |
| `GET /v1/code/repository/command` | No body; process-only ownership identity and pending preview |
| `POST /v1/code/repository/command/prepare` | Object containing `command`: the complete sealed kernel command specification |
| `POST /v1/code/repository/command/decide` | `preview_sha256` and Boolean `approve` |

Unknown, duplicate, positional and incorrectly typed fields refuse. A foreign
digest preserves the preview. Denial, expiry and an attempted dispatch consume
it; a retry requires another preparation and fresh approval. Each execution
uses a unique attempt and one exact single-use grant. The existing canonical
authority store checkpoints consumption before native launch. Policy denies
network, credential and publication scopes.

## Sandbox and results

The existing systemd and namespace runner supplies an offline, read-only
worktree at `/work`, read-only system runtime files and temporary writable
scratch. It inherits no ambient environment or credentials. The kernel refuses
a fixed list of shell and interpreter names and specific inline-evaluation flag
forms. Other interpreters, including versioned names and attached short options,
can pass that filter and remain subject to the same sandbox and exact approval.
Commands that need generated files must direct them into temporary scratch;
this surface does not provide a writable project checkout or dependency downloads.

The local coding entry point further limits each command to 120 seconds,
4 GiB memory, 64 tasks, four CPU equivalents and 64 KiB retained output per
stream. Existing contract limits, seccomp, cgroup supervision, artifact identity
checks and verified descendant cleanup remain in force. No native trust,
resource or network boundary is weakened for this integration.

Responses include the canonical authority receipt, verified command receipt,
exit classification and bounded stdout/stderr as byte arrays. Treat output as
untrusted data; never execute it or display raw terminal controls. The terminal
prints escaped JSON. Exit zero is command success only:
`test_suite_verified: false` stays explicit. The separate
[test-envelope surface](coding-validation.md) adds per-invocation parsed counts
and failure names with bounded, separately approved reruns. Test-suite coverage,
repository snapshot binding, failure-to-model feedback and budgeted automatic
repair remain unimplemented in this composition.

Command requests serialize with other coding operations. API cancellation is
truthfully unavailable after admission (`cancellable: false`); the runner's
fixed timeout and cleanup still apply. This surface has no live command-output
stream or connected cancellation token. Starting the service does not approve
any command.

Previews and ownership are process-only; command output is response-only.
Canonical content-free authority receipts survive restart. There is no durable
coding-run resume/replay or automatic retry. After interruption or a lost reply,
inspect receipts and disk before any new action. A decision refusal conservatively
reports `effect: unknown`. `no-command-dispatch` on preparation does not mean
no state or artifact-inspection activity. No model, commit, push or deletion
is authorized by these commands.

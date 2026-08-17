# Local Command-Line Interface

## Current Status

The `agent` binary is a pre-alpha command and protocol scaffold. It provides
deterministic help, version, shell completion, strict argument parsing, stable
exit codes, and human or JSON event rendering. The source tree also contains a
verified interactive runtime driver that prepares and starts the same
host-framed `RuntimeRunRequest` used by native Chat, independently checks the
event chain, artifacts, approval or cancellation boundary, and terminal
outcome, and releases the terminal run. It owns no execution authority.

The binary does not yet have an authenticated product transport, production
runtime factory, or stdin/history composition. Operational commands therefore
still fail closed with `client.transport.failed` until those dependencies are
composed and accepted.

The planned public interactive coding entry point is `agentmage code` or a
subsequently approved equivalent. Its source-level driver is a thin client of
the same reusable runtime coordinator port as native Chat; this is not an
installed-product availability claim. The current `agent` name is a
source-level detail and is not a public command compatibility promise.

## Build And Inspect

Build the current source candidate:

```bash
cargo build -p agentmage-host --bin agent --locked
```

Inspect its command reference and version:

```bash
target/debug/agent --help
target/debug/agent --version
```

Generate deterministic completion source without running a shell subprocess:

```bash
target/debug/agent completion bash
target/debug/agent completion zsh
target/debug/agent completion fish
```

## Command Families

```text
agent chat MESSAGE
agent conversations list [--from YYYY-MM-DD] [--to YYYY-MM-DD]
agent conversations search QUERY
agent conversations show ID
agent conversations open ID
agent conversations resume ID
agent resume ID --turn TURN_ID
agent vault search QUERY
agent vault note show ID
agent vault links ID
agent vault backlinks ID
agent vault tasks
agent checkpoint
agent handoff
agent audit
agent memory inspect [ID]
agent memory correct ID REPLACEMENT
agent export PROFILE
agent import MANIFEST_SHA256
agent doctor
agent diagnostics
```

Arguments are shell tokenized before AgentMage sees them. The parser rejects
unknown commands, duplicate global switches, NUL bytes, reversed date ranges,
invalid identifiers, invalid digests, missing values, excessive arguments, and
excessive aggregate input.

## Human And Machine Output

The default `interactive-cli` surface renders bounded human-readable events.
`json`, `sdk`, and `acp` surfaces require `--json` and produce one closed JSON
object or event per line:

```bash
target/debug/agent --json --surface json vault tasks
target/debug/agent --json --surface sdk diagnostics
target/debug/agent --json --surface acp audit
```

At the current pre-alpha boundary, each operational example returns a redacted
error equivalent to:

```json
{"code":"client.transport.failed","exit_code":5,"kind":"error","schema_version":1}
```

There is no hidden prompt, browser launch, cloud login, fallback transport, or
application launch.

## Interactive Coding Session Boundary

The first useful coding-harness milestone composes existing AgentMage contracts
in one local terminal session. The source-level driver proves the shared
request, ordered stream, exact decision, cancellation, artifact, outcome, and
release boundary; the remaining product composition is:

1. Authenticate the client, select one admitted local model, and bind one
   approved repository plus an AgentMage-owned worktree.
2. Explore, read, and search through native registered tools and bounded model
   context.
3. Produce an evidence-backed plan and exact-preimage patch or controlled file
   creation.
4. Render `ALLOW`, `ASK`, or `DENY` from kernel policy. An `ASK` pauses for an
   exact protected approval; the prompt itself carries no authority.
5. Execute only separately granted writes, commands, and targeted tests through
   their existing sandboxed workers.
6. Inspect Git status, diff, log, and show through native read tools and render
   bounded output with explicit truncation; use verified runtime artifact
   references once that lifecycle is enabled.
7. Finish with receipts, checks run and not run, changed files, residual risks,
   and one evidence-backed terminal outcome.

Native tools use the common `ToolRegistry` and `ToolDispatcher`; MCP is not a
prerequisite or wrapper for built-in filesystem, Git, patch, command, or test
operations. The runtime journal, optional transcript, and optional local
content-free diagnostics remain separate. Progress and token streaming do not
perform one synchronous durable write per token.

Persistent session resume, the complete durable-journal and
content-addressed-artifact lifecycle, remote Git, commit, push, advanced deep
indexing, measured routing, MCP, the complete conversation library, desktop UI,
workflow design, and multiple agents remain outside this first milestone and
retain their later gates.

## Exit Codes

| Code | Meaning |
|---:|---|
| `0` | Verified success |
| `2` | Invalid input or malformed protocol |
| `3` | Missing, stale, replayed, or mismatched authority |
| `4` | Exact policy denial |
| `5` | Authenticated service or dependency unavailable |
| `6` | Verified cancellation |
| `7` | Declared resource bound exceeded |
| `8` | Insufficient evidence for success or failure |
| `9` | Incompatible protocol or policy version |

Successful operational execution will require an authenticated local host and
the same runtime request, kernel policy decision, grant, event journal, artifact
references, receipt, evidence, and final state used by native Chat.
Noninteractive callers must present an exact bounded, expiring, single-use grant
before dispatch; they cannot obtain authority through an interactive prompt.

## Protocol References

- [Thin client boundary](../architecture/thin-client-boundary.md)
- [Request schema](../../schemas/runtime/thin-client-request.schema.json)
- [Event schema](../../schemas/runtime/thin-client-event.schema.json)
- [Request fixture](../../schemas/runtime/examples/thin-client-request.valid.json)
- [Event fixture](../../schemas/runtime/examples/thin-client-event.valid.json)
- [Adversarial corpus](../verification/sprint-48-headless-corpus.json)

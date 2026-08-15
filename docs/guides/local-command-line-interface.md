# Local Command-Line Interface

## Current Status

The `agent` binary is a pre-alpha command and protocol scaffold. It provides
deterministic help, version, shell completion, strict argument parsing, stable
exit codes, and human or JSON event rendering. It does not yet have an
authenticated product transport. Operational commands fail closed with
`client.transport.failed` until that transport is composed.

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
the same kernel request, policy decision, grant, receipt, evidence, and final
state used by native Chat. Noninteractive callers must present an exact bounded,
expiring, single-use grant before dispatch; they cannot obtain authority through
an interactive prompt.

## Protocol References

- [Thin client boundary](../architecture/thin-client-boundary.md)
- [Request schema](../../schemas/runtime/thin-client-request.schema.json)
- [Event schema](../../schemas/runtime/thin-client-event.schema.json)
- [Request fixture](../../schemas/runtime/examples/thin-client-request.valid.json)
- [Event fixture](../../schemas/runtime/examples/thin-client-event.valid.json)
- [Adversarial corpus](../verification/sprint-48-headless-corpus.json)

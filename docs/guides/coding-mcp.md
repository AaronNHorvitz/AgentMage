# Coding tools through MCP

`agentmage-mcp` connects an editor or another local program to the existing
versioned local HTTP API. Start the API, then use its trusted controls to select
the workspace and exact files. Launching this bridge shares that selection and
its returned text with the calling program. Choose a trusted program and its
model destination accordingly; the bridge cannot enforce what that program
does with text after receiving it.

Build the existing engine binaries with the pinned toolchain:

```sh
cargo build --locked --offline -p agentmage-local-api --bins
```

Use `agentmage-code <loopback-ip:port> <auth-directory> workspace` to inspect the
selection and obtain `workspace_revision` and `workspace_selection_id`. Configure the calling program to
launch this subprocess, substituting the exact nonzero revision and selection identity:

```sh
agentmage-mcp <loopback-ip:port> <client-auth-directory> <workspace-revision> <workspace-selection-id> --share-selected-workspace
```

The directory must be owner-only (`0700`). It can contain only a copy of the
ordinary `client.token`, which must be owner-only (`0600`), regular, have one
hard link and not be a symlink. Keep `control.token` with the person-facing controls. The
bridge opens only `client.token`, including when both files exist. It accepts
only a numeric loopback address and fixed HTTP routes; it never follows a
redirect or accepts a tool-selected endpoint or bearer credential.

The bridge speaks MCP protocol version `2025-06-18`. An ordinary MCP client
performs initialization and sends `notifications/initialized` before calling
tools. It must support that negotiated version. Standard output contains only
newline-delimited JSON-RPC messages. Diagnostics go to standard error.

| Tool | Result and authority |
| --- | --- |
| `workspace` | The exact selected scope and revision, without file contents |
| `workspace_read` | Complete text of requested already selected files |
| `workspace_search` | Literal matches in requested already selected files |
| `workspace_propose` | One bounded strict edit proposal from the configured model endpoint; the person must already have approved that source disclosure |
| `workspace_stage` | A complete pending strict diff for separate review; replaces the previous workspace preview without writing source |
| `workspace_diff` | Only this revision's pending diff; excludes historical write records |

Tools are listed with closed input schemas. File tools require explicit paths,
at most sixteen distinct paths within the person's selection. Proposals accept
`model`, `task`, `max_message_bytes`, `max_tokens` and optional `include_usage`;
the existing API validates encoding, disclosure and budgets. Staging accepts
`task`, `max_message_bytes`, `answer` (the existing strict JSON edit text) and
`expected_sources` (exact path and SHA-256 pairs). These are the same API inputs,
with the launch revision and selection identity injected by the bridge. Returned API events are JSON
inside an MCP text content item and carry no execution authority.

After staging, inspect the exact diff with the existing terminal client's
`review` command. Its `apply <absolute-root>` command requires a fresh typed
confirmation at the controlling terminal and uses the separate control
credential. Denial, drift and stale previews retain their existing behavior.
There is no MCP apply, approve, command, commit, push or settings tool. Workspace
staging does not record transferable owned-worktree lineage; use the existing
repository workflow when that provenance is required.

Changing or revoking the selected scope invalidates the pin. Each selection has
a fresh random identity as well as a numeric revision. The API checks an
optional `workspace_selection_id` on read, search, proposal and staging requests
under its source owner; the bridge always supplies it. Old API clients keep
their existing revision checks. A service restart cannot reuse this identity. Relaunch
with a newly reviewed selection; the bridge never silently follows it. Context
and model text are untrusted data. This adapter does not add a coding agent loop,
run project tests or qualify a local model. It cannot access private evaluation
tests or supply independent review. Documents and research tools remain open.

Frames are limited to 256 KiB including the terminating newline. Replies are
limited to 8 MiB, in addition to the HTTP client's existing 4 MiB reply, framing
and 180-second deadline. Oversized or unterminated frames end the session;
duplicate object members, batches and malformed JSON refuse. Requests need
distinct integer or bounded string IDs; a session permits at most 4,096 request
IDs. Notifications cannot dispatch tools. Calls run sequentially and are never
retried. There is no acknowledged MCP cancellation during an HTTP call. Closing
the subprocess closes its connection; inspect API state after any uncertain
staging reply. A client should enforce its own process deadline and cleanup.

Wire-format references are the standard's
[stdio transport](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports),
[lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle)
and [tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
specifications. Implementation and tests are original AgentMage work; no SDK,
third-party source or new dependency is included.
The specification project's [licensing notice](https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/LICENSE),
checked on 2026-11-07, records a transition from MIT to Apache-2.0, with remaining
MIT contributions and CC-BY-4.0 for general documentation. This change uses
protocol facts and includes no copied prose, examples or implementation.

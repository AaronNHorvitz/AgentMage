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

## Observe a retained endpoint run

A trusted host can explicitly enable observation delegation when starting the
retained endpoint service. Use
`serve_coding_endpoint_with_observation_access` with
`CodingEndpointObservationAccess::DelegateObservations`. The original
`serve_coding_endpoint` entry point keeps every route control-only. This policy is
also an explicit private development launch choice, as described below; it is
never an HTTP setting. See [Decision 0162](../decisions/0162-endpoint-mcp-observation-delegation.md).

For the existing `serve-endpoint-read-development`,
`serve-endpoint-edit-development`, `serve-endpoint-coding-development`,
`serve-endpoint-edit-review-development` and
`serve-endpoint-coding-review-development` commands, add this field to the
complete private launch JSON before starting:

```json
{"share_run_observations": true}
```

This fragment is not a complete launch document. Use the existing
[endpoint startup](coding-runtime-api.md) configuration, private file permissions
and explicit startup approval flag. Omitting the field or setting it to false
keeps observations control-only. Null, strings, numbers and duplicate fields
refuse startup. The launcher reads the private document once and retains that
policy for the service lifetime; changing the file does not reconfigure it.
No model or HTTP request can enable sharing. See
[Decision 0199](../decisions/0199-explicit-endpoint-launch-observation-sharing.md).

The launcher prints a sharing notice before native admission. All holders of
`client.token` can then retrieve the complete task, tool catalog and canonical
events; these may contain source or test data. Choose trusted recipients and
their destinations. Sharing does not approve a model call or tool effect.
Native admission still must succeed. In the builder lane it currently refuses;
synthetic service/MCP tests do not establish a successful native launcher or a
local-model coding run.

Inspect the run through the trusted terminal controls:

```sh
agentmage-code <loopback-ip:port> <control-auth-directory> endpoint
```

Review the exact request, including its task, selected model and visible tools.
Choose a trusted calling program and its destination for received text. Supply
the displayed `run_id` and `request_sha256` to a separate MCP launch:

```sh
agentmage-mcp <loopback-ip:port> <client-auth-directory> <run-id> <request-digest> --share-selected-endpoint
```

This selection advertises only `endpoint_request` (no arguments) and
`endpoint_events` (one required integer `after`, between zero and 4,294,967,295).
The first returns the original sealed request with a service-observation label.
The second returns one original canonical event page, with individual event
hashes, run/session/task/policy identities, sequence and page bounds checked.
`after` is the event index; start at zero, explicitly request the returned
`next` index, and stop when `next` equals `total`. It is not a resume cursor.
A prefix observed during an active run may grow. These pages do not prove a
complete stream or independently verified task completion.

Every call rechecks the launch pin. A replaced service, changed request or
foreign run refuses rather than sharing a new scope. Released event records
are unavailable; export needed pages before release. The bridge uses only
`client.token` and has no model-disclosure, advance, tool approval, cancellation
or release operation. Use the existing trusted terminal commands for those
controls. Enabling HTTP observation delegation alone does not consent to send
text to any model. The explicit MCP launch flag shares the pinned task and
tool observations with the calling program, whose subsequent handling of them
is outside the bridge's control. The existing workspace launch and tools keep
their separate selection and permissions.

## Follow a run in a second terminal

The terminal client can follow the same shared observations without the control
credential or an MCP caller. With observation sharing already enabled in the
service, use its exact displayed pin:

```sh
agentmage-code <loopback-ip:port> <client-auth-directory> endpoint-watch 300 <run-id> <request-digest> --share-selected-endpoint
```

Choose a time budget from one to 3,600 seconds. This command reads only
`client.token`; it works with piped stdin and supplies no approval. Its explicit
sharing flag authorizes displaying the pinned run's untrusted observations.
Choose where stdout goes: events can contain task, source and test data.

The watcher starts at index zero and flushes each event as a separate escaped
JSON object, with `verification: self-consistent-service-observation` and
`index_is_resume_cursor: false`. It checks the complete observed prefix with the
kernel event state machine, including its request binding, hashes, causation and
legal transitions. It refuses shrinking totals and inconsistent pages. No
service observation proves independent task acceptance. A terminal event ends
the watcher with exit zero, including a failed or cancelled run; inspect the
recorded state. The output is a sequence of pretty-printed JSON objects, not the
single replayable record produced by `endpoint-record`.

An empty active prefix or a fully framed `api.busy` HTTP 409 waits 100
milliseconds before polling the same index. Other HTTP errors, transport errors
and malformed data stop with a nonzero exit; no effect is retried. The watcher
also stops on its time limit, SIGINT, SIGTERM or SIGHUP. None of these stops
cancels, advances or releases the run. Original controls remain available in the
other terminal. Events are observable between coordinator advances; the service
may report busy during a model or tool call.

The additional ceilings are 4,096 events, 40,096 page requests and four MiB of
actual escaped output, also respecting the run's event limit. The original
monotonic deadline covers HTTP polling. Blocking stdout writes and backpressure
are not forcibly preempted; provide a draining output destination. The watcher
writes no record file. Save a complete terminal record through the existing
explicit retention command before releasing a run if offline replay is needed.

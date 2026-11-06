# Version 1 local chat API

T-302.3 adds the `local_chat_api` library and `agentmage-local-api` executable
to the Linux inference crate. This is a local API other programs can use. It
composes the [approved transport](local-chat-transport.md) without activating
the older qualification-bound runtime. It serves text conversations, model
listing and effect-free [snapshot coding proposals](coding-proposals.md);
tools, durable runs, MCP and a task CLI remain later mission work.
There is no desktop UI here. Synthetic socket tests do not qualify a model.

## Start and authentication

Build offline through the shared reservation:

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=2 cargo build --locked --offline -p agentmage-platform-linux-inference --bin agentmage-local-api
umask 077
mkdir -p "$HOME/.local/share/agentmage"
target/debug/agentmage-local-api init-auth "$HOME/.local/share/agentmage/api-auth"
target/debug/agentmage-local-api serve 127.0.0.1:43127 "$HOME/.local/share/agentmage/api-auth"
```

Use the configured Cargo target directory if it differs from `target`.
`init-auth` creates a **new** directory with mode 0700 and separate random
256-bit lowercase hexadecimal tokens in `client.token` and `control.token`,
each mode 0600. It refuses an existing directory and never overwrites tokens.
A failed setup can leave a partial directory; inspect it before choosing a new
directory. `serve` loads only owner-owned, single-link regular 64-byte files
through the held private directory. Symlink files/directories and permissive
modes fail. Tokens are not printed, put in a URL or accepted as command arguments.
The owned token allocations and raw request buffers are erased on normal drop.

Every request needs `Authorization: Bearer <token>`, an exact numeric
`Host: 127.0.0.1:43127` (or the bound IPv6 address and port), HTTP/1.1 and an
explicit `Content-Length`, including zero for empty requests. The service accepts
only numeric loopback bind addresses; port zero is supported and prints the
chosen listener address. It never serves a public/wildcard listener.

The control token belongs to trusted, person-facing settings controls only.
Give ordinary conversation clients the client token. Never put either token or
the model endpoint key into model context, documents, tools, logs or command
history. Possession of the control token is authority to choose an endpoint;
clients must obtain the person's explicit choice before using it. The engine
cannot prove a person clicked a control. These credentials do not isolate
mutually hostile processes running under the same operating-system account.

Browser requests with any Origin, Cookie or fetch-metadata header are refused,
even for the same origin. Proxy forwarding headers are refused too. No CORS or
cookie authentication exists. A separate local shell must use a trusted native
HTTP client. Host checking also refuses a rebound domain name. All responses
carry `Cache-Control: no-store`, `X-Content-Type-Options: nosniff` and
`Connection: close`. Responses contain no upstream error body or credential.

## Routes and explicit choice

JSON bodies use `Content-Type: application/json`; extra or duplicate JSON fields
are refused. Empty GET/DELETE requests must have zero length. There is exactly
one request per connection, with no input chunking, pipelining, compression,
upgrade, redirects or retries.

| Method and path | Credential | Request and result |
| --- | --- | --- |
| GET `/v1/status` | Client or control | Version, configured/busy booleans and settings revision; no connection to a model |
| GET `/v1/activity` | Client or control | Latest 128 ordered metadata events, last sequence and `retention: process-only` |
| GET `/v1/settings` | Control | Chosen endpoint and revision, never its key |
| PUT `/v1/settings` | Control | Explicit endpoint choice below; installs in-memory configuration without opening a model socket |
| DELETE `/v1/settings` | Control | Clears configuration and erases its owned key; fails busy during an operation |
| POST `/v1/probe` | Control | `{"endpoints":["http://127.0.0.1:8080/v1"]}`; at most eight explicit candidates, unauthenticated model listings only |
| GET `/v1/models` | Client or control | Tests the chosen connection and returns bounded model IDs |
| POST `/v1/chat` | Client or control | Streams the explicit text conversation below |
| POST `/v1/cancel` | Client or control | Empty body; requests cancellation of the single active upstream operation |

After the person chooses the endpoint for conversations, trusted settings send:

```json
{
  "endpoint": "http://127.0.0.1:8080/v1",
  "key": null,
  "approve_conversations": true
}
```

The optional key is a bearer string or null; never obtain it from discovery or
model output. The exact address and API base path are bound to the approval.
Only this authenticated explicit settings choice mints conversation approval.
False/missing approval, the client token, a remote destination or unsupported
scheme cannot configure an endpoint. A successful probe never chooses anything:
chat and `/v1/models` still return 428 until explicit configuration. Probe requests
never carry the configured key and cannot receive a key field. There is no
ambient port scan, DNS, proxy, provider selection or environment credential.

Settings and the model key are **not persisted**. Restart requires a fresh
authenticated endpoint choice. API tokens persist in their private directory.
Plain loopback HTTP authenticates the API caller by token, not the process
holding the configured model port: if another local account takes that port,
it can receive the model key. Use an appropriately protected local model service;
the install kit must revisit this residual port trust.

## Streaming and cancellation

Send an explicit model, messages and output-token ceiling:

```json
{
  "model": "chosen-model-id",
  "messages": [{"role": "user", "content": "Explain this function."}],
  "max_tokens": 256
}
```

Roles are `system`, `user` and `assistant`; the last message must be `user`.
The model and messages are validated before upstream I/O. No hidden prompt,
tools, endpoint override or provider fields are added. A successful HTTP status
starts `application/x-ndjson` with one JSON object per line:

```json
{"type":"text","text":"An incremental fragment"}
{"type":"complete","finish":"stop"}
```

`finish: length` means the output token ceiling was reached. A `complete` event
is emitted only after upstream finish, completion marker, valid framing and
physical EOF. HTTP 200, a text fragment or a completion marker in model text
does not establish success. On failure the last event is `type: error` with
`code`, fixed plain-language `message` and `partial: true`. Every earlier
fragment is then partial. If the client loses its socket before the final event,
it must also treat all text as partial. The API never retries automatically.

All fragments and model IDs are untrusted data. JSON encoding protects event
framing; it does not make text safe to execute or display as markup. Clients
must render plain text, neutralize terminal controls and expose bidirectional
overrides and invisible formatting characters in both model IDs and answers.
Never interpret model text as settings, permission, a tool call or a completion
event. The server terminal prints only startup status and fixed startup errors.

Only one upstream operation runs at once. Other upstream calls and settings
changes return 409 while it is active. Cancellation returns
`cancellation_requested`, not a completion acknowledgement; wait for a terminal
event or `busy: false`, then revoke/change settings. This prevents an old
configuration from continuing after an acknowledged replacement. A socket write
failure cancels the upstream call at the next transport check; disconnect while
the model is silent may be noticed only when it next writes or hits its deadline.

## Bounds, activity and verification

| Resource | Limit |
| --- | --- |
| Connection workers | Four fixed workers, no thread per request |
| Request status/headers | 8 KiB |
| Request body | 256 KiB before allocation |
| Entire incoming request | Two seconds, including progress |
| One response write/event | 250 ms absolute budget, 50 ms I/O ticks |
| Model operation | 120 s total, 30 s idle; transport byte/event/text caps apply |
| Discovery | Five seconds total, one second idle per candidate |
| Activity retention | Latest 128 metadata events in this process |

Slow requests release their worker on timeout. Saturation can delay admission;
there is no availability guarantee against a hostile local process. Incoming
reads are interrupted by service shutdown. Orderly library shutdown cancels the
active operation and joins the workers. The standalone process can be stopped
by the operating system and starts no child process or detached worker.

Activity records endpoint choices/revocations, upstream operation start/result
and cancellation requests with an increasing sequence and settings revision.
They retain the exact configured endpoint, never keys, tokens, prompts or answer
text. Probe candidates are returned only to their control caller. `finished`
records the upstream result; it does not prove the client received the terminal
event. Use the first retained and last sequence to detect retention gaps. These
records disappear on restart and are **not** durable resumable/replayable runs;
T-303 must integrate the canonical run store. Discovery retains the transport's
all-or-nothing timeout when its shared budget expires between candidates.

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 cargo test --locked --offline -p agentmage-platform-linux-inference --test local_chat_api
```

The tests use actual API/upstream sockets with synthetic responses and include
the real executable. They cover separate authority, probe isolation, credential
routing, bounded HTTP, first-fragment streaming, partial/error outcomes,
cancellation, settings races, shutdown, activity retention and private token
files. A real local-model conversation through the GPU slot is T-302.4.

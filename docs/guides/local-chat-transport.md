# Approved local chat transport

T-302.2 provides `local_chat_transport` in the Linux inference crate. It calls
real loopback HTTP sockets; its contract tests use synthetic servers, not a model.
It is a library surface. The versioned local HTTP service and its clients remain
T-302.3; a real local-model conversation remains T-302.4. It does not activate the
older qualification-bound gateway, start a server, run a tool or qualify a model.

## Configuration and authority

The application asks for the person's endpoint and optional bearer key. Parse the
endpoint with `LocalChatEndpoint::parse`; this performs no I/O. Accepted endpoints
remain HTTP loopback IP literals, or `localhost` canonicalized without DNS, with
an explicit port and optional API base path. Remote addresses and TLS are not yet
supported. Configuration, credentials and approval must come from trusted
application controls, never model text, documents, tool arguments or discovery.

Construct `BearerKey` by moving an owned string, then `LocalChatTransport` with the
endpoint, key and `TransportLimits`. Neither constructor opens a socket. The key
has no Debug, Clone, serialization or read accessor. Its owned allocation and the
request header allocation are zeroized on drop, including error paths. Reserve
header space before copying a key so later appends cannot reallocate that secret.
The caller remains responsible for earlier input copies. No credential is loaded
from the environment, URL, file, browser, cookie store or an external process.
There is no persistence here: a future settings owner must implement protected
storage separately. The transport neither prints nor logs content.

After the person explicitly chooses the endpoint for conversation data, the
trusted application mints `LocalChatApproval::for_endpoint`. Pass it to `models`
for the first-run connection test and listing, then to `chat` for streaming. Every
operation compares both the numeric address and API base path before I/O. The
caller supplies explicit messages and a token ceiling; the transport adds no
hidden prompt or tools. Only that transport instance owns its key. Changing the
endpoint requires a new instance and approval. These are in-process authority
contracts, not client authentication: do not expose approval constructors to an
untrusted local HTTP client. T-302.3 must enforce that boundary itself.

For optional first-run detection, obtain separate probe-only permission using
`LocalProbeApproval::for_endpoints` and call `probe_endpoints`. It tests at most
eight explicitly permitted candidates using only unauthenticated model listings.
No default port scan, ambient destination lookup, key, conversation or automatic
selection occurs. Results retain each endpoint and either its model IDs or a
content-free error. They contain no conversation approval. If a candidate needs a
key, report the authentication error and ask for explicit endpoint configuration.
An empty model list means the connection worked but no model can be selected.

## Requests and response bounds

Each call opens one TCP socket to the numeric approved address. No DNS, HTTP proxy,
redirect, cookie, automatic retry or ambient credential mechanism is involved.
Requests use HTTP/1.1, an exact Host including port, an explicit Content-Length,
and `Connection: close`. Only GET on the configured models path and POST on the
configured chat path are available. A 401/403 reports authentication failure;
redirects are refused without following Location; other non-200 statuses fail.
Errors provide fixed codes and plain-language guidance, never upstream reason
text, response bodies, keys, prompts or destination strings. Error bodies are
not parsed or displayed; an initial socket read may buffer some bytes before
status checking, discarded when the socket buffer is zeroized on drop.

Successful responses require the appropriate Content-Type: `application/json` or
`text/event-stream`, optionally with a UTF-8 charset. Compression, folded headers,
conflicting or duplicate framing/media headers, interim responses and ambiguous
framing are refused. Content-Length, chunked and close-delimited bodies work.
Chunk extensions and nonempty trailers are deliberately unsupported and fail.
The server must honor the close request: even a framed body is complete only on
actual socket EOF. Extra bytes, a truncated frame or an open stalled connection
fail rather than promote a provisional codec marker to success.

| Bound | Ceiling |
| --- | --- |
| Total operation wall time | Configurable, default 120 s, maximum 300 s |
| Inactivity while sending/receiving | Configurable, default 30 s, at most total |
| Connection attempt and I/O cancellation tick | At most 50 ms, plus scheduler delay |
| Received wire bytes, including headers/chunk framing | 5 MiB |
| Response status and headers together | 16 KiB |
| Model-list body | 256 KiB |
| Model entries / bytes per ID | 256 / 256 |
| Chat body / event / decoded text | 4 MiB / 64 KiB / 1 MiB |
| Encoded chat request | 256 KiB |

Discovery shares one total wall-time budget across its candidates. Socket reads
and writes check cancellation and both deadlines between bounded operations;
progress refreshes inactivity but never extends total time. Each call closes its
own socket on all return paths; it starts no worker thread or child process.
Cancellation is one-way and can be requested from another thread or the text
callback. Callbacks must return promptly; arbitrary blocking caller code cannot
be interrupted by this synchronous library. No retry follows cancellation or a
partial response. Response framing is streamed through a fixed 4 KiB buffer.

## Streaming consumers

The text callback runs before the response completes. All callback text is
untrusted plain text. Terminal consumers must neutralize controls and expose
bidirectional overrides as required by the [codec guide](local-chat-protocol.md).
Do not feed text into tools, settings, permissions or markup interpreters.

The callback receives no completion event. A successful return of `ChatFinish`
requires the codec's finish reason, completion marker, valid HTTP framing and
actual EOF. A length finish remains distinct from normal stop. On **any** error,
every previously displayed text fragment is partial. Do not report success from
seeing a completion marker in content or merely from receiving some text.

## Verification

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 cargo test --locked --offline -p agentmage-platform-linux-inference --test local_chat_transport
```

Tests open ephemeral loopback listeners and make actual model-list/chat requests.
They exercise approval refusal before I/O, exact credential routing, incremental
UTF-8/chunked streaming, statuses and redirects, malformed headers/framing,
truncation and late bytes, bounded model lists/bodies/headers, probe discovery,
cancellation and total/inactivity deadlines. They are synthetic contract tests,
not evidence of inference, real model compatibility or agent workflow execution.

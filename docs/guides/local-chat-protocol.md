# Local chat protocol component

T-302.1 supplies an effect-free codec in
`platforms/linux-inference/src/local_chat.rs`. It is a building block, not a
running chat application. T-302.2 supplies authorized transport; T-302.3 supplies
the versioned local API and UI; T-302.4 records a real model conversation.

## Configuration and requests

`LocalChatEndpoint::parse` accepts an HTTP loopback IP address or `localhost`,
an explicit nonzero port and an optional API base path (default `/v1`).
For example, `http://localhost:8080/api/v1/` becomes
`http://127.0.0.1:8080/api/v1`. IPv6 loopback uses brackets. The parser performs
no DNS lookup or detection and refuses remote hosts, URL credentials, query
parameters, fragments, escaped path bytes and traversal. It exposes a numeric
socket address, model-listing path and chat-completion path. Parsing is not a
permission grant. Other deployment classes remain unavailable on this path.

`encode_chat` preserves explicit system, user and assistant messages. The last
message must be from the user. It encodes one streaming choice without tools,
fallback or an implicit prompt. The model ID is bounded to 256 bytes without
control characters; at most 256 nonempty messages and 32,768 requested output
tokens are allowed. The encoded request must fit 256 KiB, including JSON escaping.
Returned bytes contain private conversation data and must not be logged.

## Streaming contract

`ChatStream` consumes HTTP-decoded event-stream body fragments. LF and CRLF,
comments, inert event metadata, multiline data and arbitrarily split UTF-8 are
supported. Exactly one UTF-8 byte-order mark at body offset zero is ignored,
including when split across fragments. It counts toward the event and body
budgets. Non-ASCII field names are refused, so a second or misplaced mark cannot
silently hide a data field. Marks inside JSON text remain content.
Each JSON event must name the exact requested model. The codec admits
one choice at index zero, assistant-role text deltas and the `stop` or `length`
finish reason. Usage-only events are permitted after the finish reason. Tool or
function proposals, mismatched models and other finish reasons are refused.
It does not qualify a model or verify the truth of its answer.

The limits are 64 KiB per event (including comments and metadata), 4 MiB total
body and 1 MiB decoded output text. These are byte limits, not token estimates.
Text events become available before completion. A `Finished` event requires both
a finish reason and `[DONE]`, after some text; `length` remains visibly distinct
from a normal stop. `Finished` is provisional until `end() == Ok(())` at transport
EOF. Every later byte, including a blank line or comment, is refused; consumers
must revoke provisional completion on a later error. Call `end` at transport
EOF; a missing marker, incomplete line or unfinished event fails. No closed
connection or partial output can imply success. Any failure poisons the decoder.

Consumers must treat every text event as untrusted plain text, render it without
HTML interpretation and mark earlier output as partial if decoding or transport
fails. Neutralize terminal control sequences and make bidirectional overrides
visible in terminal, log and UI views. Do not log model text through debug
formatting: output is private conversation data, just like input.
The codec never executes a proposal. Its errors are fixed content-free
codes; upstream details, prompts, credentials and destination strings are absent.

## Transport and application obligations

Before this component is used by the product, the transport must enforce exact
destination authorization, credential ownership, status and media-type checks,
HTTP framing, wire/header/time budgets, cancellation and no redirects or ambient
proxies. Decoded body limits do not bound HTTP framing overhead or elapsed time.
No automatic retry may duplicate a request without the run's policy.

Before exposing the local HTTP API, require loopback binding, authenticated
clients, strict browser origins and request limits. Configuration comes from the
person, never model output. Model output cannot update settings or permissions.
Do not add this codec to the existing qualification-bound gateway as a shortcut
around its identity and routing checks. Image, speech, tools and remote routes
are separate contracts. No keys, models or server processes are bundled here.

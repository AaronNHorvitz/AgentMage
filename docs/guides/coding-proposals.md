# Snapshot coding proposals

`POST /v1/code/proposals` connects bounded repository context and exact edits to
the [authenticated local API](local-chat-api.md). A caller supplies source
snapshots it is allowed to disclose to the configured local endpoint. The API
selects excerpts, streams an untrusted model answer, and returns a checked
candidate. It opens no source paths, writes no workspace files, runs no commands
and grants no permission to apply, commit or push. Those operations remain
T-303/T-306 work. The [workspace source route](coding-workspace.md) can now
collect explicitly approved files before calling the same proposal machinery.
This is a local API other programs can use.

## Request

Use the existing client bearer token and explicit control-token endpoint choice.
Origins, cookies, nonnumeric Host headers, redirects and proxies retain the chat
API's refusal rules. A client cannot select an upstream address in this request.

```json
{
  "model": "configured-model",
  "task": "Fix add so it adds the numbers.",
  "files": [
    {
      "path": "calc.py",
      "content": "def add(a, b):\n    return a - b\n"
    }
  ],
  "max_message_bytes": 6000,
  "max_tokens": 2048
}
```

All fields are required; duplicate and unknown fields are refused. Task text
is nonempty and at most 4,096 UTF-8 bytes. Supply one to sixteen nonempty UTF-8
files, at most 128 KiB each and in total. NUL bytes are refused. Paths are unique,
at most 256 ASCII bytes, with letters, digits, `.`, `_`, `-` and `/`; empty,
absolute, traversal and `.git` components are refused. Paths are display data,
never resolved on the server. The existing 256 KiB HTTP body limit also applies,
including JSON escaping. No hidden tests, references or Git history are loaded.

`max_message_bytes` is 2,048–32,768 and includes the exact serialized system and
user message content, including JSON escaping. `max_tokens` is 1–4,096. **Bytes
are not rendered model tokens.** This endpoint does not discover or qualify the
served context size or codec. Backend context failures remain failures; a byte
ceiling is not exact token admission. Changing either budget is a separate
measurement variable.

## Context and edits

Files are sorted by path. Existing pinned language parsers supply definitions;
task-word matches rank definition names, source and paths. Unsupported files use
lexical windows of up to five complete lines. Small complete files are also
candidates. Selection considers at most 2,048 ranked candidates and retains at
most 64 non-overlapping excerpts within the serialized message ceiling. A long
line or definition that does not fit is omitted, never cut mid-character. If no
excerpt fits, the request is refused before inference.

The outline lists every supplied path, its complete digest and byte count, and
up to eight parser-backed names, each capped at 64 characters. An empty name
list makes no structural claim. Outlines are deliberately incomplete and do not
establish complete parsing or repository understanding. Excerpts contain exact
original start/end byte offsets and text. Compare their union with each file's
byte count to identify omitted source. All task, outline and source text stays
untrusted data. It cannot change the endpoint, tools, budgets or authority.

The model must return only this JSON shape:

```json
{"edits":[{"path":"calc.py","old":"return a - b","new":"return a + b"}]}
```

The answer is capped at 64 KiB and one to 32 edits. Typed parsing rejects
duplicate/unknown fields, nested values, trailing data, fences and malformed JSON.
Each nonempty `old` must occur exactly once in the **complete original** file,
including overlapping occurrences, and lie entirely within one delivered
excerpt. Edits bind to original bytes, rather than sequentially rematching
previous replacements. Overlapping edits, unchanged replacements, NUL in new
text, net unchanged files and growth beyond the snapshot limits reject the whole
proposal. A changed complete source digest rejects a stale context. These checks
validate an edit proposal, not its correctness or safety to execute.

## Events and cancellation

A successful request returns `application/x-ndjson`:

1. `context`: version, exact message bytes, file outlines and delivered excerpts,
   with `authority: "proposal-only"` and a null `workspace_revision`. The workspace
   route includes the selected revision in context/proposal events.
2. Zero or more `text` events containing untrusted model text.
3. Exactly one terminal `proposal` or `error`. A proposal carries complete changed
   file text and before/after SHA-256s, with `applied: false`, `tested: false` and
   `authority: "proposal-only"`.

Treat text as provisional until a terminal proposal. A length finish, missing
completion marker, malformed or late stream bytes, oversized answer, invalid
edit or cancellation produces an error and no candidate. The operation shares
the existing single-operation budget and `/v1/cancel`; settings changes and a
second upstream operation are refused while it is active. Status and cancellation
remain available. Disconnect detection and the existing transport time/byte
limits remain in force. Events have process-only retention; this endpoint adds
no durable run, interruption recovery or replay claim.

## Endpoint request binding

The context event and terminal proposal/error now include the same
`model_binding`. Its version 1 identity kind is
`selected_endpoint_unverified_model`: the canonical local endpoint and requested
model identify the person's selection, not the server process, weights,
tokenizer or hardware. `request_body_sha256` and `request_body_bytes` describe
the frozen JSON prepared for transmission, including all messages and the
optional usage request. Successful execution sends those exact bytes. On
failure, none or only part may have been sent; the binding alone does not prove
server receipt. HTTP headers and credentials are excluded. The digest is neither an
approval nor a replay token; identical bodies sent to different endpoints have
the same body digest. Compare the complete binding when inspecting an attempt.

The binding records `requested_max_output_tokens`, `provider_usage_requested`,
the exact `total_timeout` and `idle_timeout` as `{ "secs": ..., "nanos": ... }`,
and the transport's response wire, body and text byte ceilings. The coding
proposal consumer additionally caps answer text at 64 KiB. The token ceiling
is a request to the server, not an independently measured token budget.
`verified_input_tokens` remains null; provider counts never fill it.

Terminal `response_complete` confirms only valid HTTP and stream-codec EOF.
It can be true on an invalid strict edit or a length finish, both of which still
refuse a candidate. Transport, malformed-stream and cancellation errors set it
false. Existing optional `usage` is emitted only for completed responses when
requested, labelled `provider-reported`; missing usage stays null. No completion
or counts are fabricated after disconnect.

Context and terminal events also expose `runtime_model_binding`, using the
shared runtime vocabulary: `kind: "selected_endpoint"` and an `identity` object
with the canonical `endpoint` and requested `model`. It describes selection;
the complete `model_binding` above still identifies the prepared body and limits.
Neither value contains credentials or grants inference or tool authority.

Completed responses additionally carry `token_accounting`, with
`kind: "selected_endpoint"`. Its `observation.provider_reported` is the optional
structurally checked usage report. `verified_input_tokens` and
`verified_output_tokens` always remain null. Missing reports remain null;
reported zero counts remain explicit zero counts. No exact prompt count,
remaining token capacity or hardware observation is inferred. `response_complete`
controls whether `token_accounting` is present. Incomplete transport responses,
cancellation and malformed streams carry none. The legacy error flag `partial`
describes provisional coding output; it can be true after completed inference.
A completed length finish or invalid edit therefore retains inference accounting
while still refusing a candidate, even when its error has `partial: true`.

The request binding now uses the shared `EndpointModelRequest` contract. Its
additive `context` describes the exact ordered JSON message array: message
count, serialized byte length and SHA-256. This includes roles and JSON escaping;
it is distinct from the existing `message_bytes` content budget and from the
server's unknown rendered prompt. Repair arrays include the feedback message.
The historical transport name `EndpointChatBinding` remains an alias.

A completed terminal event additionally carries `runtime_model_result`, using
the shared `EndpointModelResult` contract. Its `request` equals the context and
terminal `model_binding`, including selection and all bounds. It records the
SHA-256 and UTF-8 byte length of concatenated decoded response text, excluding
HTTP and stream framing. It retains completed inference on strict-edit refusal
and `finish: "output_token_limit"`, while neither produces an accepted proposal.
An ordinary provider stop is `finish: "stop"`; this says nothing about task
correctness. No result is emitted for malformed, incomplete or cancelled streams.

`resources.dispatch_elapsed` is a locally measured monotonic duration, represented
as `{ "secs": ..., "nanos": ... }`. It starts at dispatch entry and ends after
complete HTTP and codec EOF. It includes synchronous text callbacks and socket
waiting, excludes preparation and approval, and is neither GPU time nor server
inference time. `resident_memory_bytes` and `accelerator_memory_bytes` remain
null. Result `token_accounting` contains the same optional provider report and
null verified counts as the terminal observation. Unknown resources never become
zero or a native resource report. These records are inspectable descriptions,
without deserialization, credentials, approvals or native execution authority.

Native runtime ports use the distinct `exact_profile` binding variant. Their
complete native profile comparison and exact token/resource checks remain
mandatory. A selected endpoint cannot satisfy native admission even if its
model name equals the profile identifier. Shared endpoint exchange records do
not yet admit endpoint execution to that coordinator or its resource ledger.

The credential-owning transport prepares an immutable request without connecting.
Its single-use execution sends those same body bytes through that original
transport, with cancellation rechecked before connecting. Source/model disclosure
consent and every tool approval remain separate. Snapshot, selected-workspace
and failed-test repair proposals share this path. The terminal and MCP clients
can display these additive event fields. This does not connect arbitrary
endpoints to the native coordinator's exact-profile contract; that work remains
open alongside native edit/test/retry execution and its measurements.

## Development measurement

The new collector is separate from the unchanged, frozen chat-only baseline:

```sh
python3 -m scripts.coding_proposal_measurement --api-address 127.0.0.1:43127 \
  --auth-directory <private-auth> --model <configured-model> \
  --output <new-private-output> --candidates <private-candidates> \
  --first 0 --count 5 --approve-task-context
```

It has a fixed policy: 6,000 message bytes, 2,048 output tokens, one request per
development task, no retries, test feedback or filesystem effects. It has no
held-out collection surface. Batches contain at most five tasks; launch local
models only through both resource slots and release reservations between batches.
No source changes or prompt tuning belong inside a measured campaign.

The collector sends only the frozen statement, source path and editable base
source to this API. It retains private requests/events, timing and digest-bound
records and snapshots. Any refusal remains an explicitly labelled base fallback
in the denominator. It does not run the proposed source. Score snapshots with
the unchanged independent admission command and accepted complete control receipt;
collection success is not a test pass. API tokens are unavailable; retained
backend counters, where observed, must be labelled separately. This remains a
single-turn snapshot-proposal measurement, not execution of the native agent loop.

The frozen baseline parser is not reused by this product path. The new native
typed parser and measurement event parser have explicit deep-nesting refusal
tests, addressing the baseline review's L1 prerequisite for future reuse.

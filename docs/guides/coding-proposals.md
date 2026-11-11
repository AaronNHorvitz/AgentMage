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
most 64 non-overlapping excerpts within the serialized message ceiling. Parser
ranges include complete original lines, preserving indentation and trailing
newlines. Overlapping or touching candidates merge when the resulting message
fits; a complete-file candidate can replace smaller selected ranges. Retained
ranges are ordered by path and byte offset. Gaps remain omitted unless a larger
candidate containing them fits. A long line or definition that does not fit is
omitted, never cut mid-character. If no excerpt fits, the request is refused
before inference.

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

The model instruction explicitly requires copying decoded excerpt text with its
original whitespace and enough surrounding source to distinguish repeated
fragments. The checker still refuses ambiguous or altered preimages; it does
not trim, normalize or repair a model's edit.

The 128 KiB candidate limit covers the final combined snapshot, including
unchanged files. Growth in one file can be balanced by shrinkage in another
within the same proposal. File names, source order and edit order do not change
that accounting. All final file sizes and the aggregate are checked before
candidate content is allocated; an overflow rejects the entire proposal.

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
not yet admit endpoint execution to that coordinator. The single model phase
uses the shared resource ledger as described below.

The credential-owning transport prepares an immutable request without connecting.
Its single-use execution sends those same body bytes through that original
transport, with cancellation rechecked before connecting. Source/model disclosure
consent and every tool approval remain separate. Snapshot, selected-workspace
and failed-test repair proposals share this path. The terminal and MCP clients
can display these additive event fields. This does not connect arbitrary
endpoints to the native coordinator's exact-profile contract; that work remains
open alongside native edit/test/retry execution and its measurements.

## Cooperative dispatch deadlines

Snapshot, workspace and repair proposals use the transport's controlled dispatch
entry point. A local monotonic deadline starts immediately before context-event
publication; model dispatch does not restart that allowance. The allowance is
the prepared request's original `total_timeout`. Preparation and approval are
outside it. The original socket total and idle deadlines still apply.

`PreparedEndpointChat::execute_recorded_controlled` also accepts the existing
`ModelOperationControl` contract for a future coordinator composition. It polls
before connection, between socket waits and decoded fragments, after callbacks
and after complete HTTP/codec EOF. Every socket timeout is bounded by the run's
remaining time, the original transport deadlines and the existing I/O tick.
Each positive control observation can narrow the locally retained deadline;
subsequent observations cannot renew it. The immutable body and its binding
remain unchanged. Millisecond run observations round up a positive fractional
millisecond; exact transport durations remain independently enforced.

Owner stops return their existing `model.operation.*` codes and no completed
result, usage or proposal. They neither set the separate chat cancellation flag
nor acknowledge a runtime cancellation. Ordinary chat cancellation retains its
existing transport code. A synchronous callback must return promptly: control
cannot preempt caller code, and its first observation after return refuses late
completion. Operating-system scheduling can also delay polling. The transport
starts no worker, child process or retry.

This connects cooperative control to actual proposal dispatch, but does not admit
configured endpoints to the native coordinator's exact-profile run contract or
resource ledger. No native edit/test/retry or durable-run acceptance follows.

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

## Shared endpoint model phase

The snapshot, workspace and repair proposal routes frame their frozen request
with `runtime_loop::endpoint::RuntimeEndpointModelPhase` before dispatch. Its
constructor also requires `RuntimeEndpointContextPacket`, derived independently
by the kernel from the original borrowed ordered messages before preparing the
transport. Both context admission and phase framing precede consumption of a
repair disclosure receipt. The phase
uses the existing runtime resource ledger with one model-call allowance, the
prepared body length, the transport's decoded-text ceiling and total deadline.
It borrows the same operation control before and after the credential-owning
transport. The `RuntimeEndpointModelPort` moves the original prepared transport
and cancellation reference into this dispatch. The kernel first compares the
port's prepared request against its frozen request, before spending dispatch
accounting or invoking the port. A mismatch consumes the phase but records no
admitted dispatch. It accepts only a result naming the complete original selection,
context, body and limits, within those byte/time bounds. A failed or stopped
phase cannot dispatch again. The phase owns no credential or effect boundary.

The kernel also hashes and counts the actual UTF-8 text at the synchronous
consumer boundary, independently of the transport's response descriptor. The
completed descriptor must match those bytes exactly. Fragment boundaries and
empty fragments do not change that comparison; reordering, omitted text, a
different digest or a different byte count refuse completion. The kernel retains
only the streaming hash and byte count, without another plaintext buffer.

Each fragment is checked against the decoded-text ceiling before delivery, with
control polled before and after the consumer callback. A limit or control stop
latches for the rest of the phase: later callbacks are suppressed even if a
broken port continues or the control subsequently reports more time. The
transport still polls during socket waits and retains its original limits and
cancellation. A synchronous consumer cannot be preempted. Text already delivered
remains provisional; a later refusal produces no completed result, proposal or
completed resource accounting. A transport error keeps its original dependency
code even if a stream refusal was also observed.

Terminal events include `endpoint_phase_usage`: `dispatch_attempts` counts
admitted calls to the transport, including failed attempts, and
`prepared_body_bytes` counts their frozen request bytes. These are neither
socket-byte measurements nor model-token estimates. A refusal before dispatch
has zero attempts and bytes. `completed_response_text_bytes` and
`completed_dispatch_elapsed_ms` are null without an accepted complete response;
a complete zero-length response has zero bytes. Time is the transport's local
observation through HTTP/codec EOF, rounded up to milliseconds. Unknown memory
and tokens are not projected from empty native counters. Completed length
refusals and invalid strict edits retain their completed model observations;
they still do not establish a successful coding task.

This phase is an effect-free execution component used by the live proposal API.
It adds no native profile, second agent loop, tool admission, persistence or
retry. The full coordinator still requires its exact native run/context/result
contracts. Configured-endpoint edit/test/retry composition and parent scores
remain open.

## Endpoint context admission

The kernel accepts the closed system, user and assistant text roles, with
1–256 nonempty messages ending in a user message. Each role remains descriptive;
source excerpts and previous responses are untrusted. Both total UTF-8 content
and the serialized JSON array must fit 256 KiB. JSON escaping, punctuation and
roles count toward the array limit. This does not enlarge the transport's
existing complete request-body limit.

Admission streams the exact ordered array through a bounded hash writer. It
retains a digest and byte counts, without another plaintext buffer. The phase
constructor requires the transport descriptor's message count, serialized size
and digest to match that admitted packet. There is no descriptor-only phase
constructor. Role, order, content or count drift refuses before dispatch and
before repair consent is consumed. The transport still owns and attests to the
encoded body; this comparison does not independently parse that private body.

The initial context event includes `runtime_endpoint_context`: version 1,
`messages` (the exact array observations) and `content_bytes` (original content
only). JSON bytes are not rendered prompt tokens, and this record grants no
disclosure or effect authority. It is process-local metadata, not a durable run
or native context. The full coordinator's native contracts remain unchanged;
endpoint run/turn composition and measured coding workflows remain open.

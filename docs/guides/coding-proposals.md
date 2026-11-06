# Snapshot coding proposals

`POST /v1/code/proposals` connects bounded repository context and exact edits to
the [authenticated local API](local-chat-api.md). A caller supplies source
snapshots it is allowed to disclose to the configured local endpoint. The API
selects excerpts, streams an untrusted model answer, and returns a checked
candidate. It opens no source paths, writes no workspace files, runs no commands
and grants no permission to apply, commit or push. Those operations remain
T-303/T-306 work. This is a local API other programs can use.

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
   with `authority: "proposal-only"`.
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

# Endpoint run framing

The engine can statically seal and verify a `RuntimeEndpointRunRequest` without
constructing an `ExactModelProfile`. This is a crate API prerequisite for
configured-endpoint coordinator composition under T-303/T-306. It is not yet
accepted by the local HTTP coding runtime, CLI or MCP runtime interface, and
does not enable an endpoint edit/test/retry workflow.

`runtime_coordinator::seal_runtime_endpoint_run_request` sets the contract
version and computes the canonical request digest. Its corresponding
`verify_runtime_endpoint_run_request` checks that digest and the complete shape.
The contract JSON reader rejects unknown, duplicate and missing fields, including
an omitted `event_cursor`; explicitly use null when there is no cursor.
Endpoint and native run requests cannot be decoded as each other.

Both forms use the same task/session/work-packet, workspace and repository
identity, policy, visible-tool catalog, cursor and run-limit checks. The existing
native form still requires its complete exact model admission and matching
tokenizer context budget. The endpoint form instead carries `model_selection`
(the address and requested model) and byte ceilings in `context_budget`:

| Field | Meaning |
| --- | --- |
| `max_messages` | Original ordered messages, from 1 through 256 |
| `max_content_bytes` | Aggregate original UTF-8 content bytes |
| `max_serialized_messages_bytes` | JSON message-array bytes including roles and escaping |
| `max_request_body_bytes` | Complete prepared JSON body, excluding headers and credentials |

All byte ceilings are positive and ordered from content through body; the body
ceiling is at most 256 KiB. `max_output_tokens` is a requested server ceiling
from 1 through 32,768. It is not a locally verified output count. Endpoint runs
carry no native tokenizer, weight, hardware or context-capacity assertion.

Selection checks reject control characters, whitespace around the model name,
non-HTTP address prefixes and literal user information, query, fragment or
backslash delimiters in addresses. These are lexical guards for the selection
record. The credential-owning transport still owns full destination parsing,
address restrictions, credentials and disclosure consent; sealing a run permits
no network operation.

Controlled-write framing uses the unchanged required resource set and budget
coverage rules. Its cumulative input allowance must cover
`max_request_body_bytes * max_context_refreshes`, including encoding overhead.
Missing or narrower budgets refuse framing. This grants no write or command:
the tool owner still needs its existing exact approval and permission checks.

`runtime_loop::endpoint::RuntimeEndpointModelPhase::for_run` checks a sealed run,
matches the complete selected address/model to the frozen transport, and checks
original message count/content/JSON bytes, full body bytes, requested output
tokens, response-text ceiling and transport timeout against that run. It then
uses the existing original-context and single-dispatch phase checks. Exact
ceilings fit; a narrower ceiling refuses before dispatch. The phase retains
only the immutable run/session/task identifiers and full request digest as
`run_binding`; the ordinary proposal-only constructor leaves that value absent.

This is one phase bound to a statically framed run. The eventual coordinator
must own turn identities, cumulative resource spending, the overall clock,
recovery and effect authority. Creating a second phase does not establish a
fresh run budget, and a cursor alone does not establish resumability. No new
agent loop, model prompt, live API route, tool or consent mechanism is added.

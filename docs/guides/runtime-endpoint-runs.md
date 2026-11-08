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

`runtime_loop::endpoint::RuntimeEndpointTurns` now supplies explicit model-turn
admission and event recording for eventual full coordinator composition. It accepts only a sealed
ephemeral read-only request without a resume cursor. Its constructor starts one
run clock. Constructing another instance for the same sealed run is not prevented
here; its fresh local ledger/clock gives no new run budget, and the eventual
coordinator must prevent it through exclusive run ownership. `remaining_ms`
and every dispatch borrow the existing coordinator's
clock/cancellation implementation: time cannot restart between turns, regress,
or continue past the deadline. Cancellation must match the original task and
derived correlation identity. The exact observed signal remains inspectable;
it is never a process-exit or cancellation-acknowledgement receipt.

Each caller-supplied turn uses the original-context and streaming phase above.
The turn ID follows the coordinator's run/ordinal derivation. Accepted results
retain that ID, sealed run identity and complete request/response descriptor.
They are model observations, not verified answers, approved tools or task success.

One cumulative shared resource ledger bounds turns, context refreshes, model
calls, full prepared input bytes, accepted output bytes and accepted rounded-up
dispatch durations. Read-only packets may omit budgets; finite defaults still
come from the run, and any declared narrower budget wins. No native token or
server-memory observation is synthesized. Before dispatch, the entire configured
response-text and transport-time envelopes must fit remaining cumulative
capacity. Prepare a smaller envelope for a later turn when needed. Only actual
accepted output/time is spent, rather than the reserved maxima. Wall-clock time
also bounds preparation, waiting between turns and synchronous callbacks.

Failed dispatches retain their admitted call/input usage. A control stop before
the phase admits an attempt counts no model call or input bytes. Any refusal or
failed turn closes this component; it cannot retry or renew its ledger. Streamed
text remains provisional until completion is accepted. The containing
coordinator must decide what to do with accepted results.
If a dependency unwinds and its caller catches the panic, the component stays
closed with its reservation spent; that reservation proves no remote execution
or process exit.

The component now records its model observations with the same event framing,
hash chaining, bounded publisher, subscriptions and replay implementation used
by the native coordinator. Construction records `RunStarted`; each admitted
explicit turn records `TurnStarted`, `ModelRequested`, either `ModelCompleted`
or `ModelFailed`, and `TurnCompleted`. Request and result digests bind complete
endpoint descriptors; the completed-turn digest additionally binds run and turn.
No source, model text, address, requested model name or credential is in these
event envelopes. Sensitivity, policy, run/session/task and correlation identities
come from the sealed run. Retention is explicitly ephemeral.

Before publishing turn intent or entering the model port, the entire four-event
completion envelope must fit the shared run ledger. Preflight uses the largest
legal timestamp encoding and both accepted/failed model-event shapes. Only
actual canonical count and bytes are committed. `recorded_events` and
`canonical_event_bytes` expose those known cumulative observations. A run with
space for its initial event but no complete turn refuses dispatch. Capacity is
never silently enlarged.

`events`, `subscribe_events` and `replay_events` expose the existing event
interfaces. Subscriptions deliver future events; replay supplies the prefix
already recorded. A full queue disconnects that subscriber without blocking
inference; reconnect by the exact retained cursor. Replay invokes no model and
consumes no tool authority. Wrong-run or altered cursors and invalid batch bounds
are refused by the existing implementation.

`ModelRequested` records admitted intent, not proof of socket entry. A normal
phase stop before the attempt can therefore have a failed intent record with
zero dispatch usage. A failed model turn records only a stable diagnostic code,
never dependency text. If a dependency panics, the retained prefix stays open
at its model intent and reservations stay spent. Recording uses the last trusted
clock observation; it does not invent a newer timestamp after clock failure.
Neither a closed turn nor accepted model output asserts task success. This
component emits no `RunTerminal`, cancellation acknowledgement or exit receipt.

It interprets no proposals and executes no orchestration loop, tool, grant,
edit, command or retry. The shared event framing is integrated with the native
coordinator; endpoint turns are not yet composed into its orchestration loop
or any live runtime API route. The scope interfaces below remove the native-only request
requirement at explicitly supporting boundaries. Full coordinator composition,
durable ownership, recovery, effect authority and task verification remain open;
a cursor alone establishes no resumability. No new model prompt or consent
mechanism is added.

## Coordinator tool and verifier scope

The existing coordinator now calls `RuntimeToolBoundary::evaluate_scoped`,
`resolve_scoped` and `execute_scoped` on its non-transactional tool path, and
`RuntimeVerifierPort::verify_scoped` for deterministic completion. Its native
model admission and orchestration loop are unchanged. Transactional effects,
journals, checkpoints, recovery, artifact and research hooks still require their
original native request; no endpoint execution is enabled through those hooks.

`RuntimeRunScope::native` and `RuntimeRunScope::endpoint` verify the original
sealed request's complete shape and digest, then retain an immutable borrow.
The scope exposes common task, work-packet, workspace, repository, tool-catalog,
policy and limit observations, plus the complete request digest and its distinct
model binding. It has no deserializer or mutable field access. It does not
convert an endpoint selection into a native profile, token count or permission.
`native_request` refuses an endpoint scope. `endpoint_request` returns the
original endpoint request only for that family.

Existing tool and verifier implementations inherit adapters that forward the
original native request and all arguments unchanged. An endpoint scope returns
`RuntimePortFailure::Invalid` before invoking a native-only implementation.
An explicitly supporting implementation must override the scoped entry point;
there is no fallback profile, retry or broader grant. These methods do not
replace registry validation, exact approvals, effect authority or the verifier
registry's proof checks.

The existing coding completion verifier explicitly supports both request
families through the same deterministic checks: objective, workspace/repository
snapshots, ordered writes and later validations, clean repository inspection,
current evidence and complete distinct receipts. Native input is reframed with
the same checked scope. A successful standalone verifier candidate is inert;
the containing coordinator still owns proposal admission and proof validation.
Synthetic endpoint verifier fixtures establish no actual endpoint tool effects
or measured coding result.

This integrates request scope into the existing coordinator and coding verifier.
The coordinator still admits native runs only. Connecting endpoint turns to its
model/context/proposal loop and the live API remains required before any
configured-endpoint edit/test/retry workflow is claimed.

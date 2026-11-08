# Endpoint run framing

`coding_endpoint_owner::CodingEndpointRuns` retains up to four complete configured
endpoint coordinators under one non-cloneable in-process owner. Keep that same
owner in the hosting service. Admission takes trusted original configuration,
source snapshots, catalog and permission/verifier/clock ports; it executes no
model or tool. Later calls identify the exact sealed request and advance only
the retained instance. The original coordinator validates approvals, keeps
cumulative resources and publishes the distinct endpoint outcome.

Release refuses a running or approval-waiting run. Completed and faulted entries
keep their slots until release. Release drops private context and ports, keeping
only the run identity and request digest so it cannot reset that run's budgets.
At most 256 identities are admitted over the owner's lifetime; no tombstone is
evicted. Construction failures before admission have no effects and may be
corrected. The kernel's original reconciliation flag distinguishes rejected
approval input from errors after stateful advancement. A rejected approval
preserves the challenge. A stateful error latches the entry closed; its event
prefix remains readable and no further dispatch is allowed. Such a prefix is
not a terminal or a recovery declaration.
Export required events before release. Ordinary context buffers make no
secure-erasure claim.

This is synchronous library scheduling and exclusive ownership within one
retained owner. Authentication, host-wide identity allocation, worker scheduling,
native filesystem/grant composition and HTTP/CLI/MCP endpoint projections still
need service integration. A fresh owner cannot resume an old run; it has no
old clock, budgets or authority. It must use fresh service-issued identities.
The socket tests use synthetic tools, permissions and verifiers and establish
no real-model coding result. T-303/T-306 remain open.

`ConfiguredCodingEndpoint::for_coding_run` now supplies a concrete
`CodingEndpointContext`. The trusted caller first obtains source-read and
endpoint-disclosure consent, then supplies already authorized snapshots. The
constructor checks the sealed read-only run, actual transport selection and
complete registered catalog. It uses the existing deterministic source selector
with its unchanged 2–32 KiB allocation. Every supplied file has a whole-content
digest and byte count; only retained exact UTF-8 byte ranges are visible source.
The builder opens no path and refreshes no snapshot itself.

The system message contains the exact original model-visible tool definitions
and complete input schemas, with shared schemas written once, plus the existing
coding completion schema. The endpoint wire instruction asks for a plain JSON
tool request or completion claim. Repository excerpts remain in the user data
message. Each turn includes the original task and objective digest, selected
coverage/ranges, all current coordinator calls/results/evidence and pre-effect
refusals. Stored tool contracts retain their original bytes. For model context,
calls, result outputs and refused calls verify each payload digest and decode
UTF-8, parsing `application/json` as JSON. Each projected payload contains its
original schema, media type and digest plus readable `content` instead of a
`bytes` array; missing outputs stay null. Invalid digests, invalid UTF-8 or
malformed JSON refuse the whole preparation and latch it closed. Projection
streams one observation at a time into the same bounded writer; it never drops
history. Each raw payload must also fit the remaining content capacity before
JSON decoding allocates its temporary value tree. Oversize refuses first. No successful check or grant is inferred from model prose. Shell-only definitions
remain excluded by the original model-visible catalog projection.

The mutable context pins the complete sealed request digest and accepts only
consecutive turns starting at one. A refused preparation latches closed. It
checks the same borrowed control before and after work. A bounded writer refuses
the whole message when current observations exceed the content ceiling; the
independent kernel context admission also checks complete escaped message-array
bytes. It never drops history or truncates a result to make it fit. Prepared
body and cumulative run limits remain enforced by the original adapter and
coordinator. These are byte bounds, not native tokens or proof of server capacity.

Context and excerpts are transient process memory, with no disk transcript here;
no secure-erasure guarantee is made for those ordinary application buffers.
The original transport still erases its own frozen body on drop. Construction
of a new context is not exclusive run admission, recovery or a new budget.
The service owner still needs to prevent duplicate runs and connect its actual
filesystem/permission owner and distinct endpoint API projection. Synthetic
socket tests of context, snapshot reads and a fixture verifier establish no
real-model result or native filesystem authority. T-303/T-306 remain open.

The host's `coding_endpoint::ConfiguredCodingEndpoint` now joins the configured
local chat transport to `ReusableRuntimeCoordinator::new_endpoint`. It borrows
the credential-owning transport and original exact destination approval. Its
selection comes from that actual transport; no caller-supplied address can
replace it. Construction performs no probe or dispatch and admits no native
model profile.

The trusted `CodingEndpointContextPort` receives the original sealed run and
the coordinator's current calls, tool results, evidence and pre-effect refusals.
It returns borrowed original messages once per turn. It owns source selection,
read/disclosure consent, treatment of untrusted text and private-text retention.
The adapter independently admits those same ordered messages, then freezes them
through the original transport. Only that single-use encoded body reaches the
socket; no callback reconstructs context or resolves another credential.
The transport erases its frozen body on drop; the context owner remains
responsible for erasing its own retained text.

The original run control bounds preparation and all transport waits. A stopped
or mismatched selection refuses before entering context. Invalid context or
destination approval refuses before connecting. Failed/incomplete streams,
output-token limits and ungrounded answers cannot establish task completion.
There is no retry in this adapter. Model output still goes through the existing
coordinator's exact tool catalog, separate permission approval and owning
completion verifier. Source text and model prose never grant authority.

The unchanged transport advertises a 1 MiB decoded-text ceiling per dispatch.
The coordinator requires that entire ceiling to fit the remaining cumulative
output budget before each call. Plan that allowance explicitly for multiple
turns; the adapter never raises the sealed run's limits or silently narrows a
descriptor while retaining a larger socket allowance.

Socket tests compose this adapter with a synthetic endpoint, a test-only
permission owner and the native read-only snapshot operation. They are not a
real-model coding result or the live filesystem/grant boundary. The live HTTP
service still needs exclusive endpoint run ownership, a concrete scoped context
and tool composition, and its distinct endpoint result projection. Endpoint
edit/test/retry, durable recovery, approved Git and measurements remain open.

The engine can statically seal and verify a `RuntimeEndpointRunRequest` without
constructing an `ExactModelProfile`. The reusable coordinator also accepts this request for ephemeral read-only
library execution under T-303/T-306. The local HTTP coding runtime, CLI and MCP
runtime interface do not yet expose that composition. An endpoint
edit/test/retry workflow remains open.

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
admission and event recording for independently supplied model turns. It accepts only a sealed
ephemeral read-only request without a resume cursor. Its constructor starts one
run clock. Constructing another instance for the same sealed run is not prevented
here; its fresh local ledger/clock gives no new run budget, and the live
run owner must prevent it through exclusive run ownership. `remaining_ms`
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
edit, command or retry. The shared event framing and underlying model phase
also serve the read-only coordinator described below; that coordinator owns
proposal interpretation, tools and task verification. No live runtime API route
yet exposes endpoint coordination. The scope interfaces below remove the native
request requirement at explicitly supporting boundaries. Controlled writes,
durable ownership, recovery and coding workflow acceptance remain open;
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
The coordinator admits read-only endpoint runs through its explicit endpoint
constructor below. Connecting the live API and native controlled tools remains
required before any configured-endpoint edit/test/retry workflow is claimed.

## Completion verification without a native proposal

The existing coordinator now calls `RuntimeVerifierPort::verify_completion`.
`RuntimeCompletionVerificationInput` retains the original request scope and
all evidence, ordered tool results, receipts, postconditions and state revision.
Its claim carries either the original native proposal or an endpoint completion;
no endpoint profile or codec identity is invented. Native model/proposal
admission, the state controller and verifier registry remain unchanged.

The default adapter requires the original native request and proposal, checks
matching native profile and codec identities, and forwards all observations to
`verify_scoped`. Endpoint claims refuse before either legacy verifier method.
Existing scoped overrides keep receiving their original native arguments.

`RuntimeCompletionClaim::endpoint` requires an accepted `RuntimeEndpointTurnResult`
and the complete original decoded response text. It checks the sealed run,
selection and exact response length and SHA-256 before copying bounded text.
The caller supplies the trusted payload schema; the model cannot choose it.
The resulting payload is JSON text for deterministic parsing by its verifier.
Its claim digest binds the complete accepted turn (including request, response,
run and turn identities), schema and payload. Proposal identity derives from
that digest. No native proposal, profile, codec, token or memory observation is
created. Claims have no deserializer and cannot reconstruct an accepted turn
from wire metadata. Rechecking scope refuses a valid claim moved to another run,
request revision or request family.

`CodingCompletionVerifier` explicitly supports this entry point. It parses the
same completion-candidate schema and applies the same objective, snapshot,
ordered validation, repository inspection, current-evidence and distinct receipt
checks. Its returned candidate is inert until the existing verifier registry
checks the exact proposal identity, state revision and postconditions. Neither
accepted model text nor a standalone candidate proves a coding task succeeded.
The older `verify_scoped` API retains its native-shaped descriptive input for
compatibility; synthetic use of that API with an endpoint request never proves
endpoint model/proposal admission. Endpoint composition must use the new claim
entry point, not construct a native-shaped proposal.

The native coordinator is connected to this completion entry point. Endpoint
turns can now feed the actual coding verifier without fabricated native model
fields. Endpoint tool-proposal admission, the containing coordinator's exclusive
run ownership and model/context orchestration, answer-evidence provenance and
live HTTP/CLI/MCP composition still require implementation. This increment
provides no endpoint coding workflow, durable continuation or new model score.

## Terminal framing and endpoint answer provenance

The coordinator's existing native outcome validator now shares its identity,
terminal-state, counter, evidence, receipt, diagnostic and bounded-output
predicates with endpoint terminal framing. Native sealing, serialization and
answer-evidence validation retain their original behavior and exact model
manifest requirements.

`seal_runtime_endpoint_outcome` and `verify_runtime_endpoint_outcome` use the
distinct `RuntimeEndpointOutcome` envelope. Its `outcome` contains the common
terminal observations; its outer `answer_evidence` binds an endpoint completion
claim and the complete ordered current evidence. The nested native
`outcome.answer_evidence` must be null. An endpoint model observation never
becomes a native manifest, token count or evidence-state assignment.

Successful rendered output requires the owning `RuntimeCompletionClaim` at
both sealing and verification. Its original accepted turn, ordinary model stop,
request/response descriptors, complete sealed run and exact schema-bound bytes
remain bound by the claim digest. The output must equal that claim's complete
inline payload, including its schema, media type and digest. Decoding recorded
metadata cannot reconstruct the accepted claim. The accepted claim's turn must match
the supplied nonzero turn count, with an equal model-call count. This pure sealer
cannot inspect the live ledger or determine whether a later turn exists. Missing or
foreign claims, changed output and native answer evidence refuse. Shape and
size validation precede citation hashing. Failures carry no answer
provenance or supplied completion claim. Shared shape validation still permits
success without rendered output, with evidence, just as native outcomes do;
that grants no task-success authority.

`outcome.outcome_sha256` hashes the **whole endpoint envelope**, with that
nested digest set to zero. The endpoint coordinator terminal event references
this whole-envelope digest. Hashing only the nested outcome is insufficient. The wire
form is closed and versioned through the nested outcome; outer answer evidence
must be explicitly present or null. It is distinct from native outcome JSON.

Only ephemeral read-only endpoint framing without a resume cursor is admitted.
No durable, artifact-backed or controlled-write endpoint mode is enabled here.
These functions frame an already decided result; they never verify a coding
task, inspect a repository, select a proposal, emit a terminal event or run a
loop. The containing coordinator must obtain its own current tool observations,
receipts and state revision and pass its exact verifier registry before deciding
success. Full endpoint coordinator and API wiring, exclusive ownership,
edit/test/retry and measured coding acceptance remain open.

## Read-only coordinator execution

`ReusableRuntimeCoordinator::new_endpoint` composes a `RuntimeEndpointTurnPort`,
the exact sealed endpoint request, its exact read-only tool registry, a scoped
tool boundary, a completion verifier and the original run clock. This uses the
existing coordinator's advancement loop, permission challenges, tool attempts,
observations, resource ledger, event publisher and replay. It creates no second
agent loop or native model profile. The explicit model-turn component described
above remains separately usable and is not nested inside this coordinator.

Each preparation receives the original run, current turn, actual completed calls,
tool results, current evidence and pre-effect rejections. The transport owner
must derive context from these observations and obtain the existing disclosure
approval. It returns the single-use frozen transport, independently admitted
original-message context and a trusted completion schema. Prepared transports
may borrow externally owned configuration; no permanent allocation is required.
The same run control bounds preparation, dispatch and callbacks. The kernel
checks exact selection, request/context agreement, ordinary stream completion,
text bytes and digest before using the response. Count, full prepared input,
output and dispatch-time envelopes must fit the cumulative run budget.

A tool response is a closed JSON object with exactly `tool_call`, whose object
contains `tool_id`, `tool_version` and a plain JSON `arguments` value. It carries
no model-authored schema, digest, call identity, run, grant, approval or receipt.
The coordinator rejects duplicate keys, derives the call identity, encodes the
arguments and builds their schema-bound envelope from the exact registry. It
checks that complete envelope and tool-specific arguments before requesting a
tool. A malformed tool representation or invalid argument value produces a
canonical terminal refusal, with no permission evaluation or execution. Valid
calls invoke the existing scoped permission and tool boundaries. Every effect
still needs the boundary's own valid authority. Endpoint composition admits
only the existing ephemeral read-only catalog; writes, commands, network tools,
durable state and resume cursors remain refused.

Other response text is an inert completion payload under the schema fixed
before dispatch. The owning coordinator constructs the exact verifier registry
from the current task, claim, repository, state revision and ordered acceptance
criteria, and supplies its own tool results, receipts and evidence. Model prose,
foreign candidates and advisory verifiers cannot complete the task. Native-only
tool/verifier ports refuse endpoint scopes before their legacy entry points.

Call `run_endpoint_until_boundary` to obtain a concrete
`RuntimeEndpointCoordinatorStep`: an approval or the complete
`RuntimeEndpointOutcome`. Native advancement methods refuse an endpoint
instance, and the endpoint method refuses a native instance before starting.
Terminal publication binds the whole endpoint envelope digest, including its
separate answer provenance. `endpoint_outcome()` exposes that envelope only after
terminal publication; the native `outcome()` accessor remains absent. The nested native answer evidence remains null.
Repeated terminal reads run no model or tool. Advancement errors latch the
existing reconciliation refusal; rejected approval input leaves its challenge
intact. Event replay replays observations without re-executing operations.

Keep one mutable coordinator as the owning instance for a run. Constructing a
second instance with the same request cannot resume the first or grant a fresh
budget; this library constructor is not a global identity registry. The live
API still needs to enforce exclusive run ownership and connect its real
credential-owning transport and native tool authority. Controlled writes,
native edit/test/retry, durable ownership, approved Git and parent measurements
remain open. Synthetic coordinator tests establish no real-model coding score.

The generic resource snapshot is a ledger diagnostic, including unused counters;
it is not endpoint token or memory telemetry. Endpoint memory and locally
verified input/output token counts remain unknown.

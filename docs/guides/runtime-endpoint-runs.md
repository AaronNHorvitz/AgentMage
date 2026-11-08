# Endpoint run framing

## Terminal access to a retained endpoint service

`agentmage-code <loopback-ip:port> <private-auth-directory>` now supports the
following commands against the dedicated endpoint service described below.
Every endpoint route requires the control credential. This uses the existing
bounded HTTP client; it never follows a model-supplied URL or redirects.

| Command | Action |
| --- | --- |
| `endpoint` | Inspect the original sealed request and fixed configuration |
| `endpoint-step <run-id> <request-digest>` | Confirm model disclosure, then advance the retained coordinator |
| `endpoint-approve <run-id> <request-digest> <challenge-json>` | Review and approve the exact challenge, separately confirm model disclosure, then send that one response |
| `endpoint-deny <run-id> <request-digest> <challenge-json>` | Deny that exact challenge after confirming disclosure needed to continue the run |
| `endpoint-events <run-id> <request-digest> <index>` | Inspect one canonical event page, starting at zero |
| `endpoint-cancel <run-id> <request-digest>` | Latch cancellation without model disclosure |
| `endpoint-release <run-id> <request-digest>` | Separately confirm dropping a terminal or faulted run's process-only record |

Use the run identity and full request digest returned by `endpoint`. Copy the
complete `step.challenge` JSON from an approval-waiting response when approving
or denying it. These arguments identify an existing owner; they never construct
a new run, change settings, select files or reset a budget. JSON arguments are
bounded by the client and the operating system's command-line limit.

Before advancing, the terminal fetches and verifies the service's sealed
request and checks its exact run identity and digest. Its complete request,
endpoint, model and limits appear in the disclosure review. Tool approval also
shows the exact protected presentation and readable JSON arguments; the service
still validates the original pending challenge. Declining either operation or
disclosure consent posts no approval. Each new advance needs new disclosure
consent, including after a tool denial. The same coordinator may make several
model turns within one advance, up to its next approval boundary or terminal
observation and within the original budgets. No automatic tool approval or
session authority is added. Consequential consent and release require a real
controlling terminal; piped input is refused. Terminal text is escaped.

The command prints the original step under `step`, labelled
`verification: service-observation-only`. It checks identity and response shape
but cannot recreate the coordinator's accepted completion claim from wire
metadata. It does not independently certify task success. Event pages check
each event's canonical digest, sequence and run/session/task/policy binding;
the label `individual-event-hashes-only` does not claim whole-stream replay or
completion verification. `next` is an inspection index, never a recovery cursor.
Export pages before release or shutdown. Cancellation acknowledges a request;
it does not declare the model or tools stopped. Release cannot reopen a run.

A refusal, disconnect or timeout is reported without retrying, cancelling or
releasing automatically. Inspect events after an uncertain reply before taking
another action. There is no automatic driver or durable recovery here.

This connects an executable terminal client to the HTTP library entry point.
The service still needs trusted native filesystem/grant composition and an
executable activation. It is not yet an installable configured-endpoint coding
workflow. The native evaluation runtime commands remain separate. Endpoint MCP,
edit/test/retry, approved Git and measured T-303/T-306 acceptance remain open.

## Retained coordinator ownership

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
The owner marks a retained run faulted before entering the coordinator. A normal
return clears that mark only on success or an error that the kernel identifies
as preceding stateful advancement. If a dependency panics and the hosting service
catches the unwind, that run stays faulted: events remain inspectable, release
is allowed, and another advance invokes no dependency. The owner does not catch
panics, manufacture terminal events or declare recovery. Missing or mismatched
endpoint envelopes after terminal publication also set the kernel's original
reconciliation latch. Restoring envelope metadata does not reopen the run.
Export required events before release. Ordinary context buffers make no
secure-erasure claim.

This is synchronous library scheduling and exclusive ownership within one
retained owner. Authentication, host-wide identity allocation, worker scheduling,
native filesystem/grant composition and CLI/MCP projections still need
service integration. The dedicated HTTP library adapter below fixes one run
and one configuration per service; it does not activate those native ports. A fresh owner cannot resume an old run; it has no
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

## Retaining configuration independently of settings

Trusted composition can pass `CodingEndpointConnection::Shared` to
`CodingEndpointAdmission`. It retains the exact original immutable transport
and exact destination approval through `Arc`; the borrowed alternative remains
available. The local API now holds those two objects through `Arc` as well.
No credential or approval is decoded from a run request. Replacing a settings
reference cannot redirect a retained coordinator, replace its key, reconstruct
its context or renew its budget. This ownership primitive is not a settings
revocation policy; the eventual service composition must still enforce its
existing consent, selection and cancellation rules before dispatch.

`LocalChatTransport::prepare_chat_shared` uses the same approval validation,
encoding and binding as ordinary preparation. Its single-use request owns the
original transport and zeroizing encoded body, so it can outlive the caller's
settings and context references. Dispatch uses the existing controlled socket
path. It creates no worker, global allocation, cloned credential or retry.
Dropping or consuming the prepared request releases its transport reference;
releasing a terminal or faulted retained run drops that run's configuration
references. Other legitimate holders can still retain their own references.

This remains in-process configuration ownership. No configured-endpoint HTTP
route, native filesystem/grant composition, persistent recovery, write/test/retry
workflow, model score or parent closure is established by these socket tests.

## Dedicated HTTP ownership

`agentmage_local_api::serve_coding_endpoint` is a trusted-host library entry
point over the same retained owner. It creates fresh random run and session
identities, calls its trusted factory once and rejects any other identities.
The factory returns `CodingEndpointHttpRun` with the sealed request, original
shared configuration, already consented source snapshots and original
permission/tool/verifier/clock ports. HTTP input cannot construct these ports,
select source paths or change configuration. Admission performs no model or
tool dispatch. The caller must obtain source-read consent before capture and
supply actual confined native ports; the adapter grants none of that authority.
The temporary snapshot vector is dropped immediately after context admission.

This service has fixed configuration and one admitted run for its entire
lifetime. It exposes no manual settings, workspace, repository or native-host
routes. To change scope or configuration, cancel the old service and start a
new one with fresh identities and consent. There is no settings replacement
race within it. Each advance requires `approve_model_context: true` using the
control credential. Refusal performs no dispatch and preserves any challenge;
consent can be supplied by the person on a later request. Cancellation uses the original coordinator correlation, even before its first
event, and latches for the rest of the service lifetime and permits cancellation advancement
without disclosure consent. It cannot be cleared by a later request.

All routes require the control credential and ordinary exact numeric Host,
loopback, bounded-body and browser-origin protections. The client credential
cannot read events, approve, advance, release or cancel this run. Apart from
request inspection, bodies bind the exact `run_id` and `request_sha256`.

| Method and path | Input and behavior |
| --- | --- |
| `GET /v1/code/endpoint/request` | Returns the original sealed request and process-only retention. |
| `POST /v1/code/endpoint/advance` | Binding, `approve_model_context`, optional `response`; returns the original approval challenge or complete distinct endpoint terminal envelope. |
| `POST /v1/code/endpoint/events` | Binding and zero-based `after` index; returns at most one original canonical event, `next` index and current `total`. This index is not a recovery cursor. |
| `POST /v1/code/endpoint/cancel` | Binding; latches a cancellation request independently of a busy coordinator. The response acknowledges a request, not termination. |
| `POST /v1/code/endpoint/release` | Binding; releases terminal/faulted private state and retains the spent identity. Running and approval-waiting release refuses. |

Advancement borrows the same complete coordinator. Exact challenge validation,
model/tool budgets, state transitions, completion verification and terminal
idempotence remain in the kernel. Concurrent advancement, event reads and
release return busy while that owner is locked. Request inspection and
cancellation remain available through the four existing bounded HTTP workers.
Shutdown reaches the same cancellation probe and joins every owned worker.
No new agent loop, model process, automatic HTTP retry or detached worker exists.
If a response is lost, inspect events and the original boundary before deciding
whether to advance again. Events must be exported before release or shutdown;
this service promises no durable recovery, artifact download or live event stream.

No executable activates this entry point yet. Its socket tests use synthetic
ports and a verifier that deliberately rejects completion; no software task
success is established. Actual native filesystem/grant composition, CLI/MCP,
edit/test/retry, approved Git and measured coding results remain open. This
adapter does not relax native model-profile admission to provide those ports.

## Endpoint policy composition

`coding_authority::bind_endpoint_coding_policy` accepts trusted startup inputs:
a complete sealed endpoint request, a held workspace authorization, an exact
registry and explicit excluded scopes. It returns the resealed request together
with the original `CodingRuntimePolicy` type. Use the returned request for
subsequent admission and client pinning: the preliminary seal is replaced.
Only the policy identity, policy digest and resulting whole-request seal change.
The original task, endpoint selection, snapshots, catalog and budgets remain.

The function checks the original seal, workspace identity, complete ordered
catalog references and catalog digest. It refuses non-read tools, persistent
or write modes and event cursors. The shared policy constructor retains exact
actor/task identities, bounded coordinator action identities, authorization-bound
workspace scopes and all canonical exclusions. Network, credentials and
publication remain denied. Native coding continues through its original
profile checks and guidance exclusions before using that same constructor.

Binding a policy creates no grant, source-read consent, disclosure approval,
coordinator, file read or effect. A trusted factory must still compose the real
tool boundary with separately rendered approvals, grant issuance/consumption
and native path revalidation. This helper does not activate the native-profile
Linux boundary for an endpoint. Its component tests use actual Linux descriptors
through the existing test-only workspace selector and kernel grants in memory;
they establish neither production platform admission nor a running coding agent.
Executable endpoint activation, native tool/grant composition, edit/test/retry,
durable recovery, approved Git and parent measurements remain open.

## Native file and grant composition

Trusted hosts can compose `coding_endpoint_files::CodingEndpointFiles` as the
retained endpoint coordinator's `RuntimeToolBoundary`. This currently accepts
only the original text file-read contract from
`runtime_tools::workspace_file_read_runtime_registry`. It requires the original
sealed request and matching kernel policy, a continuously held authorized Linux
workspace, privately rooted durable authority and the actual admitted offline
Linux sandbox runner. Constructing it performs no model call, source read or
grant issuance. Native model admission is not synthesized or bypassed.

Select the workspace and obtain permission to inspect its files before composing
this boundary. Source snapshots and endpoint disclosure require their existing
separate consent. Each model tool call still requires a fresh exact protected
decision. Exclusions are checked before path resolution. A path is resolved
under the held root without following symbolic links, with the existing bounded
native preimage hashing; the held regular file and exact preimage enter the
protected preview. Revalidate after approval and before consumption. Concurrent
replacement refuses. A denial cannot become an allow; a used action cannot
receive fresh authority. Only one pending or issued operation is held at a time.

Approved reads derive the original single-use durable kernel grant and pass it
through the original authority transaction and offline native worker. Borrowed
cancellation retains the original task and correlation. An uncertain transaction
or cleanup latches this boundary closed. Output uses the same verified read
result, sensitive-text filtering, byte ceiling and receipt-disposition handling
as the native coordinator. Durable grants and receipts do not establish durable
coordinator recovery. No command, edit, network, credential, commit or push tool
is exposed by this catalog.

This is a library composition increment. The lane's native executable ownership
check remains unavailable; successful native reads and executable service
activation have not been demonstrated. Descriptor/grant component tests use the
ordinary private grant owner and explicitly synthetic effect observations;
they supply no replacement runner and cannot satisfy native admission. Both
T-303/T-306 and their model measurements remain open.

The endpoint policy binder and this file boundary do not discover repository
guidance or narrow its catalog/budgets automatically. Before executable
activation, trusted startup must derive exclusions, disabled tools and budget
caps from the verified repository-guidance policy. Supplying a broader catalog
is not permission to ignore that guidance. The zero-call policy-binding edge
case remains low backlog in the binder; the native file/grant owner refuses it.

## Trusted startup composition

`coding_endpoint_startup::CodingEndpointStartup` connects that narrowing step
to the original retained coordinator owner. Trusted startup supplies the sealed
read-file selection, a continuously held authorized workspace, the actor and
the complete current instruction ledger. Its snapshot must match the request.
The original provenance verifier refuses corrupt, conflicting or stale ledgers;
the original guidance compiler applies explicit trust decisions. Missing
provenance is not interpreted as an empty ledger. The host remains responsible
for actual selection/read consent and fresh capture: hashes cannot supply either.

Budget reductions precede kernel action binding. Disabled tools are removed;
removal of this catalog's only tool or a zero-call ceiling refuses startup.
Stop constraints refuse. Required validations also refuse because no validation
worker is registered by this file-only composition. Confirmation constraints
retain the existing exact approval on every read. No constraint creates grants.
The complete compiled guidance digest enters the final sealed task constraints.
Use the final request for client pinning; this replaces the preliminary seal.

Admission consumes preparation and requires the real admitted offline worker,
privately rooted durable authority, original configured endpoint connection,
trusted identities and clock, and an independent deterministic verifier port.
Supplied source snapshots still require their prior read/disclosure consent.
An excluded or malformed source path refuses the whole admission before context
construction; sources are not silently omitted. The original context validates
the remaining source bounds, catalog and endpoint selection. The same owner
retains the native file boundary, model context and verifier across advances.
Admission executes neither model nor tool and issues no grant.

This factory accepts a verifier from trusted code; it does not qualify arbitrary
checks. The exact file-observation verifier below is available for its supported
criteria. Executable activation and successful native admission remain open.
There is no native-model profile fabrication, worker admission exception,
durable recovery, edit/test/retry or new measured coding result. Tests of
held-root preparation and provenance do not establish successful worker
execution or a model workflow. Both parent rows remain open.

## Exact file-observation acceptance

`coding_endpoint_read_verifier::CodingEndpointReadCheck` represents a complete
UTF-8 file observation: a canonical workspace path, byte length and lowercase
SHA-256 of the full consented snapshot. Its `criterion()` returns the complete
supported acceptance text. Trusted startup puts those exact criteria, in order,
in both the task and work packet **before sealing and preparation**. Do not
replace a person's arbitrary acceptance text with these narrower checks.
Obtaining file identities requires prior workspace-read consent; constructing
checks opens no path and creates no read or disclosure permission.

After preparation, call `prepared.read_verifier(checks)`. This refuses excluded
paths and binds the concrete verifier to the entire final sealed request. Pass
it as the existing `CodingEndpointStartInput.verifier` when consuming
preparation. The supported request is ephemeral read-only, with the exact
single-file text catalog, observation authority, no mutable files or initial
authoritative evidence, and ToolOutput as its sole required evidence kind.
Unknown/free-text criteria, required validations, duplicate file checks and
foreign workspace paths refuse. A different run, policy, snapshot, endpoint or
postcondition cannot reuse the verifier. No model profile is manufactured.

Successful completion requires a valid no-op candidate for the exact objective
and a complete successful read for every selected file. Native output schema,
payload/result digests, complete byte ranges, full content hash, unchanged
state, unique calls/receipts and the exact current native evidence references
are checked. Evidence for each postcondition comes only from that file's
matching results. A good earlier read cannot conceal a later conflicting read.
Partial/truncated results, missing or stale evidence and a model assertion of
changed state or unrun checks cannot establish success. Native-family verifier
entry refuses; the original coding verifier remains unchanged.

The no-op disposition means **the selected file observations completed without
changing state**. It does not verify the meaning of the model's summary, prove
repository correctness, certify any tests or promise that a file stayed unchanged
after the observed read. Broader tasks need their own deterministic acceptance
checks. Constructor inputs are a trusted host responsibility, never model choices.

Component tests use direct synthetic endpoint completions and pure snapshot
reads. The socket test exercises the production retained owner, context,
transport and concrete verifier, with a synthetic endpoint and explicitly
synthetic approval/effect/receipt boundary. It records success for matching
bytes and failure for different bytes through the original canonical terminal
path. This does not demonstrate native admission, a real model or an executable
coding workflow. Activation, edit/test/retry, recovery, approved Git and parent
measurements remain open.

## Explicit native read-only development launch

The local API executable now offers this narrowly supported launch:

```sh
agentmage-local-api serve-endpoint-read-development 127.0.0.1:8787 \
  <private-auth-directory> <absolute-private-launch.json> \
  --approve-workspace-state-and-endpoint
```

Initialize API credentials with the existing `init-auth` command. The launch
document and optional endpoint credential must be absolute paths in canonical,
owner-only directories (mode `0700`), with regular, singly linked, owner-only
files (mode `0600`). Symlinks, device files and oversized input refuse. The
launch document is capped at 1 MiB; a credential is capped at 4096 bytes, with
the original bearer-key syntax and no trailing newline. No endpoint key appears
on the command line or inline in the document.

A trusted caller prepares this closed version-1 JSON object:

| Field | Value |
| --- | --- |
| `version` | Integer `1` |
| `state_root` | Existing private development state directory |
| `disposable_root` | Existing private disposable parent, separate from state |
| `workspace_root` | Its existing private Git worktree with the original explicit development marker |
| `worker` | Absolute native read-only worker path, subject to the selected trust rule below |
| `worker_trust` | Optional `root_owned` (the default) or explicit `sealed_sibling_development` |
| `credential_file` | Optional private endpoint-key path, or `null` |
| `request` | Original sealed `RuntimeEndpointRunRequest`, using the exact original file-read catalog |
| `instructions` | Verified `InstructionEvidenceLedger` for that request's workspace snapshot |
| `checks` | Objects with canonical `path` components, full `byte_len` and lowercase `content_sha256` |

This is a trusted preparation interface, not a command that turns arbitrary
task prose into verified acceptance. Each check's generated criterion must
already appear, in order, in both the task acceptance criteria and work packet.
Any unsuccessful, truncated, multi-item or insufficiently evidenced read in
the run fails acceptance, even if a later read matches every declared check.
The preparer owns the prior consent and accuracy of snapshot/provenance metadata.
Startup does not silently discover instructions or read source files to invent
checks. Required validations or broader semantics refuse. Source excerpts are
empty; later exact reads go through the worker. Template run/session/task/packet
identities are replaced with fresh service identities and the request is sealed
again before original narrowing and policy binding.

The startup flag authorizes configuration/key reads, workspace selection and
private authority state. It does not dispatch the model or grant a tool read.
After successful admission, use the existing `agentmage-code` endpoint commands
against the printed loopback address. Every advance needs fresh model-disclosure
consent; every read requires its exact challenge response. The original endpoint
controls, events, cancellation and release operate on this same retained owner.
Startup diagnostics and the bound address are printed on stderr. A refused
launch emits no service output on stdout. Select `worker_trust` with a JSON
string; object representations refuse.
MCP observation delegation is off for this launch. Retention is process-only;
restarting creates a fresh run and does not restore prior approvals or budgets.

Native admission remains mandatory. The launch verifies the fixed system
executables, the worker and the fixed runtime libraries before opening the
endpoint key or canonical state. A refusal prints the content-free
`code.endpoint.native-admission` diagnostic and the original sandbox code.
The current lane's system executables fail the original ownership checks.
With the default `root_owned` selection, a user-owned worker also refuses.
An explicit `sealed_sibling_development` selection uses the existing development
snapshot mechanism: `worker` must name exactly `agentmage-read-only-worker`
beside the canonical running `agentmage-local-api` executable. Other paths,
aliases, missing files, unsafe ownership/modes and non-executables refuse.
The worker bytes are copied into a sealed descriptor before native launch;
subsequent pathname edits cannot change that snapshot. This does not authenticate
a development build as a production package. Build both binaries together:

```sh
cargo build --locked --offline -p agentmage-local-api \
  -p agentmage-capability-read-only \
  --bin agentmage-local-api --bin agentmage-read-only-worker
```

There is no automatic fallback between selections. Both keep the fixed system
executables, libraries, namespaces, seccomp, limits and per-action approvals.
No root install or policy relaxation is authorized by this guide. Successful
native service/model execution remains unobserved, as do edits, tests, retries
and parent acceptance. See
[Decision 0163](../decisions/0163-native-endpoint-read-development-launch.md).

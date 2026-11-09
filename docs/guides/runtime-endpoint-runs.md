# Endpoint run framing

## Return the original tool owner after a terminal run

Trusted in-process hosts can use
`CodingEndpointRuns::release_with_tool_boundary(&original_request)` after
exporting the run's events. This consumes the original terminal coordinator
and returns its actual tool boundary, leaving the run identity permanently
spent. Running or approval-waiting entries return `StillRunning`; faulted
entries return `Faulted`. Unsafe terminal return, including uncertain effects,
returns `ToolNotRecoverable`. Refusals leave the original entry intact for
inspection. Ordinary `release` still discards terminal or faulted owners.

For `CodingEndpointPreparedBoundary`, call `into_repository()` on the returned
boundary to retain the original live worktree owner. Existing edit lineage and
validation attempts survive. Pending edit/command previews and grant handles
are abandoned through the original native methods; issued grants remain in
durable authority until ordinary expiry. A later test, edit or original-checkout
diff must be freshly prepared and separately approved. No run, old challenge,
model connection, budget ledger or execution cursor is restored.

The kernel's consuming `try_into_endpoint_tool_boundary` refuses missing or
disagreeing terminal envelopes, incomplete terminal publication, pending
approvals, suspension, reconciliation requirements and uncertain state. It
returns the unchanged coordinator on refusal without invoking dependencies.
Successful extraction drops the old context, connection, verifier and clock.
The boundary can retain its own original control and clock until abandonment;
neither is renewed by extraction.

This is process-local library continuation, with no new HTTP, CLI or MCP route.
It does not provide service restart recovery, automatic retries or continued
execution under the old run's budget. Native positive startup and model-driven
workflow measurements remain unobserved. See [Decision 0170](../decisions/0170-terminal-endpoint-tool-owner-return.md).

## Trusted startup for an original prepared effect

`serve_coding_endpoint_prepared` is the typed trusted HTTP entry point for this
composition. Its factory runs once with fresh service run/session identities and
returns `CodingEndpointPreparedHttpRun`: the original bound preparation,
credential-owning connection, continuous control, clock, independent verifier
and previously consented source snapshots. It admits through the original
prepared factory into the same registry served by the authenticated endpoint API.
Excluded or malformed snapshots and foreign identities refuse before serving.
Both prepared native catalogs now project their exact original input schemas
into endpoint context; a substituted schema identity or digest still refuses.

After explicit successful terminal release and joined service shutdown, it
abandons pending previews through the original native boundary and returns
`Option<LinuxCodingRepository>`. Existing edit lineage and validation attempts
stay with that repository. Stopping without release returns `None`; uncertainty
or faults refuse release and preserve inspection. Errors can drop process-held
ownership. Each later effect needs separate preparation, admission and approval.
There is no path adoption, automatic cleanup or renewed grant/budget.

Real HTTP integration tests now exercise successful prepared-edit binding,
admission, exact challenge resolution, native writes and repository return over
two fresh services, with synthetic creation and endpoint responses. They also
exercise excluded snapshots inside admission and a stop without release. The
returned owner prepares an original-checkout diff spanning both edits; it does
not apply that diff. These are component/service observations, with no native
Git creation, real model coding, command-factory admission or task score. The
Linux `test-support` feature supplies the explicitly synthetic creation fixture;
normal builds have no fixture constructor or route that adopts a worktree.

`CodingEndpointPreparedStartup` composes one original prepared edit or command
with the retained configured-endpoint coordinator. The trusted host supplies
current repository instruction provenance, selects the exact catalog and obtains
source-read and model-disclosure consent. `prepare` checks the original seal and
catalog, compiles the existing narrowing rules, reduces limits and reseals the
request with the narrowing identity. It performs no source read or effect.

Edit exclusions remove paths from the explicit mutable set. The original native
constructor then refuses any diff touching a removed or protected path. Source
snapshots under excluded scopes refuse the entire admission. Commands refuse
per-directory read exclusions because the original offline command worker can
read the whole held worktree. Required verification also refuses here: this
factory has no independently bound completion-validation registry. A prepared
test command alone cannot satisfy that requirement.

Use the final `request` to construct the original canonical call, then `bind`
with the live `LinuxCodingRepository` and original coordinator turn/operation
identities. The native adapter checks creation ownership and the full preview,
including command ceilings against the narrowed run. A binding refusal returns
the repository intact. Request policy metadata is preserved; it is not used as
a substitute for the native owner's actual kernel policy or exact grants.

`CodingEndpointPrepared::admit` checks every supplied source path before context
composition and moves the original effect owner into `CodingEndpointRuns`.
The coordinator and native boundary share one epoch-clock owner; neither clones
a clock nor renews a budget. Clock poisoning refuses both consumers. The host
also supplies the original continuous control and independent verifier. Model
advances still need separate disclosure consent, and every effect still needs
its original protected decision. The selected edit or command adapter delegates
all effect stages to the original native owner, without another executor.

Construction and admission perform no model/tool dispatch. Admission failure
drops the process-held owner and does not clean up or adopt the worktree.
Abandoning a bound preparation returns that same owner with the pending effect
dropped. There is no replay or durable recovery. This is a single-prepared-effect
library composition; executable controlled startup, general model-driven
edit/test/retry, recovery, approved Git and coding measurements remain open.
Positive prepared-edit admission is demonstrated only by the synthetic-creation
service fixture above. Native Git creation and prepared-command admission through
this factory remain unobserved in this lane.
T-303/T-306 remain open.

Trusted hosts can call the exported `serve_admitted_coding_endpoint` with a
factory that uses its fresh service-issued run/session identities to prepare,
bind and admit the original native owner. The returned request must identify an
actually held run with those exact identities. The original authenticated HTTP
service then owns the same coordinator; no port is extracted or reconstructed.
All existing routes, startup observation policy, disclosure consent, exact
approvals, cancellation and release checks stay in the same service path. This
is an embedding API, not an executable controlled-write launch command.

The original command attempt owns one cancellation token. Handles obtained
through `command_cancellation` or `validation_cancellation` before handoff share
it with the endpoint supervisor bridge. Repeated lookups and direct decisions
without an explicit handle also observe that same token. The retained run
control and scoped probe still validate their original scopes and signal the
same attempt; the first accepted reason remains authoritative. Cancellation
cannot approve execution or reopen a spent decision. A fresh preparation or
validation retry owns a fresh token; old handles cannot cancel it.

Cancellation observed before endpoint native preflight retires dispatch without
launch or validation-attempt consumption. A grant already issued remains recorded
until its ordinary expiry; cancellation is not revocation. During dispatch, the
original supervisor and authority/command receipts determine the outcome. An
acknowledgement alone does not establish that a child stopped.

Before handoff, command admission reserves
`4 * (stdout_bytes + stderr_bytes) + 16384` bytes within the sealed per-result
output limit. Channels are JSON byte arrays, so the reservation includes decimal
expansion and a fixed 16 KiB metadata allowance. Overflow or an insufficient limit returns the original
repository intact, without a grant or validation attempt. This is a minimum
reservation, not a complete bound on parsed validation metadata or the run's
remaining aggregate output budget. The actual complete projection is still
checked after dispatch; refusal retains the original result and never replays.
CPU and scratch/disk preflight coverage remains backlog. Inspect actual retained
state; refusal does not establish that no process effect occurred.

## Native prepared-edit decision handoff

Trusted Linux hosts can consume a `LinuxCodingRepository` with its original
prepared edit through `into_endpoint_edit`. Use the exact catalog returned by
`linux_coding_endpoint_edit_registry`, a sealed controlled-write endpoint request,
the original coordinator turn/operation identities and a canonical call whose
arguments are `{"preview": <complete-original-native-preview>}`. Source reads and
diff preparation retain their original consent requirements. The caller must
already own run admission, instruction narrowing and source/model disclosure.
The constructor performs no write, model call or grant issuance.

The non-cloneable `LinuxCodingEndpointEdit` retains that owner, request, call and
protected challenge. Every edited path must be explicitly mutable and outside
protected paths; altered catalog, workspace, creation binding, call or native
preview refuses. Its challenge shows the complete native diff and uses the
original writer's eventual approval/grant identities. `decide` accepts only the
exact response to that challenge and the continuously retained original run
control. A matching decision spends the handle before any further dependency;
denial, stop, error or unwind cannot authorize a second attempt. A malformed or
foreign response leaves the original pending decision intact.

For direct `decide`, the original `decide_edit` owns grant issuance, single-use
consumption, atomic write, preimage/link checks and reconciliation. Inspect the original native result
with `inspect`; its actual per-operation receipt chain is returned unchanged.
The bounded transaction is non-interruptible once admitted. A lost result never
means no effect. There is no retry, recovery or secure-erasure claim.

`into_repository` consumes the handle and returns the same live native owner for
separately approved later work. It drops any pending edit while preserving native
results and successful lineage. It cannot restore an old preview or reconstruct
ownership from a path, receipt or restart.

The [scoped coordinator adapter](#original-native-edit-through-the-coordinator)
now connects this same live handle to `RuntimeToolBoundary`. This remains library
composition without service activation. Controlled endpoint startup, general
model-driven edit/test execution, recovery, approved Git and measurements remain
unfinished. Real file-write tests use synthetic creation provenance and
do not demonstrate native Git creation or local-model coding. T-303/T-306 remain
open. See [Decision 0167](../decisions/0167-native-endpoint-edit-decision.md).

## Controlled-write composition for trusted hosts

The shared kernel coordinator and host context accept a sealed
`controlled_write` endpoint request through the existing trusted composition
interfaces. A host must supply the actual scoped permission/tool owner, exact
registered catalog, original connection, continuously retained clock and
independent completion verifier. Request bytes and model output cannot select
or reconstruct these dependencies. The read-only development launch and its
file/grant owner still accept only `ephemeral_read_only`; they cannot be used
as a write or test executor.

Controlled mode admits only workspace reads, workspace writes and bounded
commands under the original catalog rules. Every effect still passes the
original scoped permission boundary; endpoint transport creates no grants.
Full controlled-write resource ceilings must cover the complete prepared body
and all planned turns. Decisions remain bound to the exact pending challenge,
operation and run. A write decision cannot approve a later test command.
The original state-change checks refuse a successful write reporting no
change, and uncertain or failed observations cannot establish completion.
Completion still requires the registered deterministic verifier and an accepted
endpoint claim, with the whole terminal envelope bound to its original request.

This is process-held execution only. Durable read-only mode and event cursors
remain refused by endpoint composition. There is no automatic effect replay,
restored grant or restarted budget. No native model profile is synthesized.
Tests use synthetic effect ports and a loopback model peer; they establish
coordinator and transport composition, not native edits/tests or model quality.
The executable native controlled-write owner and its trusted activation remain
unfinished. T-303/T-306 remain open. See
[Decision 0166](../decisions/0166-endpoint-controlled-write-composition.md).

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
constructor checks the sealed process-held run, actual transport selection and
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
constructing an `ExactModelProfile`. The reusable coordinator accepts this
request for ephemeral read-only or process-held controlled-write library
execution under T-303/T-306. Trusted hosts can expose it through the dedicated
endpoint API. Native endpoint edit/test/retry activation remains open.

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
also serve the shared coordinator described below; that coordinator owns
proposal interpretation, tools and task verification. The scope interfaces
below remove the native request requirement at explicitly supporting boundaries.
Native controlled-write activation, durable ownership, recovery and coding
workflow acceptance remain open;
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
The coordinator admits read-only and controlled-write endpoint runs through its
explicit endpoint constructor below. Connecting native controlled tools and
trusted activation remains required before any native configured-endpoint
edit/test/retry workflow is claimed.

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

Ephemeral read-only and process-held controlled-write endpoint framing without
a resume cursor are admitted. Durable and artifact-backed endpoint modes remain
unsupported. The selected mode does not construct or authorize a tool owner.
These functions frame an already decided result; they never verify a coding
task, inspect a repository, select a proposal, emit a terminal event or run a
loop. The containing coordinator must obtain its own current tool observations,
receipts and state revision and pass its exact verifier registry before deciding
success. Full endpoint coordinator and API wiring, exclusive ownership,
edit/test/retry and measured coding acceptance remain open.

## Shared endpoint coordinator execution

`ReusableRuntimeCoordinator::new_endpoint` composes a `RuntimeEndpointTurnPort`,
the exact sealed endpoint request, its exact mode-appropriate tool registry, a scoped
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
still needs the boundary's own valid authority. Read-only mode admits only its
existing read catalog. Controlled-write mode additionally admits workspace
writes and bounded commands through the supplied scoped owner and full explicit
resource ceilings. Network tools, durable state and resume cursors remain refused.

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
credential-owning transport and native tool authority. Native controlled-write
activation, edit/test/retry, durable ownership, approved Git and parent measurements
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

## Export and replay a terminal event record

Before releasing a terminal run, explicitly export its request and complete
canonical event envelopes through the same authenticated API:

```sh
umask 077
agentmage-code <loopback-ip:port> <private-auth-directory> \
  endpoint-record <run-id> <request-digest> > <private-record.json>
agentmage-code endpoint-replay < <private-record.json>
```

Export prints JSON on stdout; the caller chooses its destination and filesystem
permissions. The command itself writes no file. Treat the record as private:
the task, endpoint address and event metadata can identify sensitive work.
It contains no API token, endpoint key, source-context excerpts or original
tool-result bodies. It retains event payload references without fetching them.
The version-1 record has exactly `version`, `request` and `events` members.

Export uses the control credential and only the existing request/event routes.
It never advances the run, sends context to a model, approves a tool, cancels or
releases anything. It refuses empty, changing, incomplete or invalid event
histories and records exceeding 4096 events or the run's own event ceiling.
Collection shares a 180-second socket deadline. Aggregate serialized content
and the final escaped output are bounded to 4 MiB. Refusal emits no record on
stdout; shell redirection may already have created an empty destination file.
Export before release: the service does not retain events after releasing them.

Offline replay needs no service, address, credentials or model. It reads at most
4 MiB plus one oversize-detection byte from explicitly supplied stdin, rejects
duplicate/unknown members and positional objects, and checks the original
request seal, exact start digest, scope, complete hash chain, causation, legal
transitions and terminal ordering using the existing kernel event verifier.
It prints the original record and deterministic inspection fields. Recorded
text stays escaped for terminal display.

`canonical-event-chain-only` is a consistency result. Digests do not authenticate
the original writer, verify referenced payloads, prove a model's claims or
independently certify the recorded terminal state. Those limits appear in the
replay output. No grant, pending approval, live port, budget, payload or execution
cursor is reconstructed. Execution recovery, full tool-output retention and
durable service storage remain open under T-303/T-306.

## Original native edit through the coordinator

A trusted library caller can consume its `LinuxCodingEndpointEdit` into
`LinuxCodingEndpointEditBoundary` with `into_boundary`, retaining the original
continuous run control and epoch clock. The scoped coordinator port accepts only
that original sealed endpoint request, exact tool definition, canonical call and
protected challenge. Its native-family methods refuse. This one prepared-edit
adapter does not discover sources, generate a replacement diff or activate an
HTTP service.

Evaluation asks for the original challenge. Resolving an exact Allow issues the
original kernel write grant and returns its actual authority binding; no file is
written yet. Execution consumes that grant through the existing native atomic
writer, with original expiry, preimage and link checks. It observes original
control, cancellation and clock before dispatch. Denial, stop or dependency
failure spends the matching stage; invalid substituted inputs stay inert.
A previously spent direct-decision handle cannot become a fresh boundary.

The output retains the original native result and full operation receipt chain.
Its coordinator receipt uses the final chain entry's real digest and a descriptive
`transaction:sequence` identity. It is process-held evidence, without a durable
coordinator transaction. An output-budget refusal after writing reports uncertain
execution; inspect the original native record and disk before further action.
No automatic replay occurs. Returning the same live repository drops any pending
preview or grant while preserving successful lineage and existing results.

The coordinator fixture uses synthetic endpoint and creation provenance with
real native writes. Its deliberate later model refusal yields a non-successful
run, even after a committed edit. It establishes no model-driven coding result,
native Git creation, tests or coding score. Trusted controlled startup, general
edit/test execution, durable recovery, approved Git and parent measurements
remain open under T-303/T-306. See
[Decision 0168](../decisions/0168-native-endpoint-edit-coordinator.md).

## Original command and tests through the coordinator

Trusted library code can move its original `LinuxCodingRepository`, with an
already prepared command or validation, into `LinuxCodingEndpointCommand` using
`into_endpoint_command`. Consume that handle with `into_boundary`, retaining the
original continuous run control and trusted clock. Use the exact single-operation
catalog from `linux_coding_endpoint_command_registry`. The complete original
preview is the only accepted argument. This does not activate an HTTP service,
select tests from prose, prepare a command or permit model disclosure.

Evaluation presents the original protected challenge. An exact Allow verifies
the original native manifest and issues the original durable operation grant,
without launching a process or spending a test attempt. Execution verifies native
admission again and consumes the same grant through the original command driver.
Memory, task-count and timeout ceilings must fit the sealed run. Run cancellation
and deadline stops reach the original supervisor as attempt-scoped signals;
foreign or unavailable observations fail closed. Native execution stays offline
with a read-only worktree and temporary scratch. NotChanged describes workspace
state; process execution still requires its own exact permission.

The output contains the actual authority receipt, original command receipt,
untrusted byte-array output and optional normalized validation. A failed test
can select a fresh separately approved rerun through the same live owner; the
original parser, immutable definition, failure receipt and three-attempt ceiling
remain in force. These results do not independently grade a task or verify the
current repository. Model-generated success text is never a test receipt.

Inspect `last_result` on the boundary after dispatch. Output-budget refusal
returns uncertain even when an original result is retained. Errors after dispatch
also conservatively report uncertain; inspect canonical authority receipts before
another action. Every matching stage is single-use, including after stop, error
or unwind. `into_repository` abandons pending preview/grant handles and preserves
existing validation state. Issued grants remain in durable authority until normal
expiry; abandoning a handle is not revocation. No pending command is restored
from a digest or event record.

The component fixtures use synthetic creation and command results with real
kernel authority transactions and parsing. The actual shared-coordinator test
uses an explicitly synthetic executor and refuses the later model turn, so its
run remains non-successful. Original native admission still refuses in this lane.
Successful native execution, controlled service startup, general model-driven
coding, durable recovery, approved Git and parent measurements remain open.
See [Decision 0169](../decisions/0169-native-endpoint-command-coordinator.md).

### Returning the terminal tool owner from the HTTP service

Trusted hosts may use `serve_admitted_coding_endpoint_returning_tool` with the
same original admission factory and protected HTTP controls. Its existing
`/v1/code/endpoint/release` route returns no tool data over HTTP: it retires the
exact known terminal run and keeps its original tool owner inside the service.
After shutdown joins all HTTP workers, the host receives `Some(original_tool)`.
An observation credential cannot release; a substituted request, live run,
fault or uncertain terminal refuses. Refusal preserves inspection. Successful
release removes the old events, so export them first. Repeated release refuses.

Shutdown without an explicit successful release returns `None`, including after
cancellation. Service errors may drop process-held state. This is ephemeral
continuation; it does not reconstruct or resume a run. Prepared native boundaries
must return their repository through `into_repository`, abandoning old preview
and grant handles, before preparing later work under fresh admission and approval.
Only the original tool boundary returns, including its existing native authority;
no grant is renewed. The old verifier, context, endpoint connection and run
budget are dropped. Ordinary service entry points continue to discard on release. The opt-in function
is a host composition surface; executable controlled startup and a general
model-driven edit/test/retry workflow remain open.

## Strict edits in trusted prepared startup

An edit-kind `CodingEndpointPreparedStartup` can now take a strict model answer
through `prepare_edit_proposal(&mut repository, &context, answer, now_epoch_ms)`.
Supply the original `CodingProposalContext` from the consented model request,
the same live native repository owner and fresh time. The context exposes its
original task through `task()`; it must equal the final sealed task objective.
This is a trusted library call. Obtain consent to re-read every context file
before calling it. It supplies no read, disclosure or write permission itself.

The method checks instruction exclusions before selecting or opening files,
then re-reads every original context file from the owner's held worktree. The
existing strict proposal checker verifies complete snapshot hashes and permits
only unique, visible, nonoverlapping exact edits. A changed file must appear
literally in the narrowed mutable list and stay outside every protected file
or directory. Protected files may provide separately consented context.

Only a fully valid proposal reaches the original native diff preparation.
Refusal preserves the previous preview; success replaces that inert preview.
Preparation does not write source files. Call the existing `bind` with the
complete resulting preview, then admit through `serve_coding_endpoint_prepared`.
Its original exact challenge, protected approval, atomic writer, preimage
checks and terminal owner return remain authoritative. Reads made during
preparation cannot be undone; snapshots are checked at preparation time, with
edited-file preimages checked again by the native writer at dispatch.

This call does not generate a model answer, account for its usage or manage its
budget; those remain with the original caller. It is no new HTTP tool, executor,
command admission exception or general edit/test/retry loop. Integration tests
use synthetic strict answers and endpoint responses with real native writes
through two HTTP services. Native Git creation, positive command-factory
admission, executable controlled startup, model quality, recovery, approved Git
and T-303/T-306 measurements remain open.

## Strict generation before native preview

Trusted library callers can use `prepare_edit_generation(connection, context,
include_usage)` on an edit-kind `CodingEndpointPreparedStartup`. Supply the
original consented `CodingProposalContext`, credential-owning connection and
exact destination approval. It freezes the original strict-edit instruction
and exact context message; it checks the task, exclusions, selected endpoint
and model, and sealed per-phase context, request, output and time ceilings.
Preparation opens no socket and reads no file.

The transport currently fixes its per-response text ceiling at 1 MiB. The
kernel admits that complete envelope before dispatch, so a narrowed run must
have `max_output_bytes` of at least 1 MiB even though generation retains at most
64 KiB. Smaller ceilings refuse with `ResourceExhausted` before a socket opens.
Supporting a smaller prepared transport envelope remains same-row backlog.

Inspect the returned `binding()` and reserve its model call and frozen body
in the existing owner's cumulative budget before calling `execute` with the
original continuous run control. Execution consumes the frozen request once,
through the existing kernel endpoint phase and controlled streaming transport.
It never selects tools, issues authority or automatically retries. Streamed
text is provisional. Answers over 64 KiB cancel the generation; partial or
output-token-limited answers cannot prepare a native edit.

The returned observation retains `result()` and `usage()` even when the model
answer is malformed or later source checks fail. Missing completed counts
remain absent; optional token counts remain provider reports. These are
single-phase observations, without a task grade or independent verification.

After separate consent to re-read every original context file, call
`prepare_preview` with the same sealed startup, original live repository owner,
original run control and fresh time. A different startup seal, failed generation
or stopped control refuses before reads. The original strict parser rechecks
full snapshots, including unedited context files, and mutable/protected paths.
Only a valid answer prepares the complete native diff. Exact approval, original
kernel grants, atomic writes and terminal owner return remain unchanged.

This is a library composition primitive. It does not add an HTTP route or
activate a model during ordinary service admission. Cumulative budgeting across
generation and coordinator work remains the trusted caller's responsibility;
passing the same sealed request to a new service does not account for an earlier
phase. General model-driven edit/test/retry scheduling, executable controlled
startup, durable recovery, approved Git and T-303/T-306 measurements remain open.
Socket tests use synthetic endpoint answers and creation provenance, including
native writes through separate protected HTTP approvals. They establish no
actual local-model result, native Git creation, test-worker success or score.

Preview preparation now compares the sealed workspace label with the original
repository's held created-worktree label before selecting sources, reading files
or replacing a preview. A foreign label or absent creation owner refuses with
`Scope`, including through generated-answer preparation. This addresses the
strict-preparation review's low workspace-binding finding; independent core
review of the repair is required. Generation observations survive this refusal.

`created_workspace_id()` exposes only that process-held label and performs no
file access. Equal labels establish neither root identity nor authority: the
original native reader still revalidates held roots and selected paths, and
`bind` still checks the original complete request and creation receipt. Callers
must still supply the correct live owner and prior source/disclosure consent.
Generation without a repository owner cannot establish workspace ownership.

# Native coding runs through the local API

The development coding host can now serve its existing coordinator through the
versioned local HTTP API and `agentmage-code`. The host owns plan/act/observe,
exact native tools, failed-test observations, bounded corrections, canonical
events, approvals, job control and durable checkpoints. The HTTP adapter adds
no model loop, command runner, store or approval responder.

This is a local API other programs can use. It is a development transport for
the existing native harness, **not an accepted everyday coding workflow**.
T-303 and T-306 remain open. Positive native execution is refused in the current
lane by unchanged executable-trust checks. HTTP and terminal tests against
synthetic peers do not establish native edits, tests or model quality.

## Start

Build the API, terminal and exact sibling host together with the locked offline
toolchain, under the shared build reservation:

```sh
bash /tools/build-slot cargo build --locked --offline -p agentmage-host --bin agentmage-host -p agentmage-local-api --bins
```

Use an existing marked disposable development Git workspace and its separate
private host state as required by the existing coding-development activation.
Do not point this at the person's active checkout. Starting the service
explicitly authorizes native repository inspection and canonical state access;
the host composes its repository map before accepting an HTTP request.

Initialize API authentication with `agentmage-local-api init-auth <new-private-auth>`
if needed. Starting a service does not approve inference, writes or commands.
Use the existing GPU reservation for a real-model service, with a bounded
lifetime and cleanup. For example, substitute the chosen existing private roots:

```sh
bash /tools/build-slot bash /tools/gpu-slot queue-run agentmage 1800 target/debug/agentmage-local-api serve-runtime-development 127.0.0.1:43129 <private-auth> <state-root> <disposable-root> <marked-workspace> muse new
```

The supported model labels here are `muse` and `gpt-oss`, using their existing
exact evaluation profiles and native family codecs. `scripted` is refused at
this entry point. Nothing is downloaded or bundled. There is no automatic
fallback. The chat API's separately configured OpenAI-style endpoint is **not
used by this native transport**. Connecting arbitrary configured endpoints to
the exact-profile coding host remains open; a chat connection does not prove
native profile or tokenizer admission.

Startup fixes the existing `failed-test-repair` scenario. The mutable file is
`src/calc.py`, validation runs the existing `tests/run_validation.py`, and the
acceptance criterion is "The exact synthetic validation passes after the bounded
repair". The task prompt does not replace these restrictions. `GET /v1/code/runtime`
reports the scenario, mutable files and criterion. Use the marked disposable
fixture required by that scenario; this entry point does not admit arbitrary
repositories or their test definitions.

`new` selects the existing new-run preparation. `resume` selects the host's
existing durable reconstruction path; it does not invent a checkpoint or replay
consumed grants. Positive recovery through this new HTTP path remains unmeasured.
SIGINT, SIGTERM and SIGHUP stop accepting work, close the authenticated IPC
channel, and terminate and reap only the owned sibling host through the existing
launcher. Interrupted effects remain subject to the canonical host's recovery
rules. A signal is not a claim that every effect was undone.

## Terminal workflow

Every command starts with:

```sh
target/debug/agentmage-code 127.0.0.1:43129 <private-auth> <command> <arguments>
```

`runtime-run 1800 '<task>'` connects the whole interactive transport in one
command. It uses the host's existing verified interactive client driver over
HTTP. It asks separately for inspection and disclosure of the exact sealed
request, presents verified canonical events, and asks for `allow <digest>` at
every consequential operation. Anything else declines that operation. It has no
automatic approval, model loop, tool runner or prompt retry of its own. The host
owns inference, tools, observations and bounded corrections.

The driver checks the request against the task and startup selection; verifies
the ordered event chain, pending approval and terminal outcome; and reads every
retained artifact in bounded pages to verify the complete payload digest. Only
then does it print `verified-outcome`. A verified refusal is still a refusal:
only canonical `SUCCESS` or `NO_OP` exits zero. Synthetic transport tests are
not evidence of native success or coding quality.

The `seconds` argument is 1–3600 and starts before inspection. Waiting for a
terminal answer is interruptible and deadline-bound. SIGINT, SIGTERM, SIGHUP or
the deadline requests canonical revision-bound job cancellation at the next
verified boundary; it never sends the API's generic cancellation route. A
pending HTTP request retains the existing 180-second response deadline, so this
argument is not a hard process-lifetime bound. If cancellation does not reach a
terminal result on the next boundary, the client stops with an error; inspect
the canonical job before further action. The owned host remains responsible for
cleanup. This client does not terminate the service or claim effects were undone.

The client prints `run-prepared` with the run ID and request digest before start
dispatch. A lost response or invalid boundary fails closed without resending the
start or approval. The existing driver may try to release a prepared or ended
run; an active run refuses release. Errors do not establish that a dispatched
effect did not happen. All terminal content is escaped; no transcript or artifact
file is automatically written. The complete verified result includes artifact
references and declarations. The underlying records remain owned by the host.

The individual transport commands remain available for explicit inspection and
recovery:

1. `runtime` displays the startup-bound workspace, entry digest and native model
   profile. No model or repository identity comes from a model response.
2. `runtime-prepare '<task>'` shows the selection and task and requires the exact
   `inspect <entry-digest>` confirmation in the controlling terminal. Its output
   is the host's complete sealed run request. No model starts yet.
3. `runtime-start '<prepared-request-json>'` verifies that request and requires
   `start <request-digest>`. This consents to model disclosure of task, repository
   context and subsequent tool observations. It grants no tool effect.
4. `runtime-step <run-id> <request-digest>` polls canonical events, artifacts,
   the pending approval or terminal outcome. Repeat explicitly as needed.
5. `runtime-approve <run-id> <request-digest>` fetches the current complete sealed
   challenge and shows its exact tool, arguments, effects, targets, bounds and
   expiry. Type `allow <challenge-digest>` in the controlling terminal. It sends
   that challenge's exact proposed grant once. `runtime-deny` sends a denial
   without a grant. Neither action selects a tool or runs a local shell command.
6. `runtime-job <run-id> <request-digest>` reads reconciled durable job status.
   `runtime-control <run-id> <request-digest> cancel` or `suspend` requests the
   corresponding transition using the observed revision. `resume` requires a
   fresh controlling-terminal confirmation. The IPC host derives the control
   request's authenticated client scope and records the decision.
7. `runtime-declarations <run-id> <request-digest>` reads the host's terminal
   declarations. `runtime-release` asks the host to release only an unstarted
   request or terminal run. It is refused while a run can still act.

There is no terminal auto-approve option or piped approval. Each invocation uses
the control credential. A lost HTTP reply never triggers automatic redispatch;
inspect events and job state before deciding what to do next. An accepted cancel
or suspend request is distinct from the host's acknowledgement that work stopped.
There is no automatic frontend polling loop or frontend repair orchestration:
the existing host coordinator owns model/tool turns and retry decisions.

## HTTP contract

All routes below require the control token, numeric loopback Host and the
existing exact bounded HTTP framing. Browser origins and proxies remain refused.
Only GET status/activity are also available to the ordinary client token.
The dedicated runtime service refuses the separate chat, manual workspace and
manual repository routes, including their generic `/v1/cancel` operation. Use
canonical job control for a native run. `/v1/status.runtime_attached` identifies
this service; its `busy` field describes the API's separate ordinary operation
lease, not whether a native run is computing. Inspect the exact run or job.

| Method and route under `/v1/code/runtime` | Closed request body and result |
| --- | --- |
| `GET /` (without trailing slash) | Startup selection; no effects |
| `POST /prepare` | `prompt`, `approve_inspection`; returns the host's sealed request |
| `POST /start` | `request`, `approve_model_context`; consumes only that exact host-prepared request |
| `POST /advance` | `run_id`, `request_sha256`, `after_event_cursor`, `response`; canonical step/replay, optionally one exact approval response |
| `POST /job` | `run_id`, `request_sha256`; reconciled job status |
| `POST /control` | `run_id`, `request_sha256`, `request` (existing versioned job-control request); host-scoped decision and status |
| `POST /declarations` | `run_id`, `request_sha256`; existing terminal declarations |
| `POST /artifact` | `run_id`, `request_sha256`, `reference`, `offset`, `maximum_bytes` (1 to 65,536); existing verified terminal artifact page |
| `POST /release` | `run_id`, `request_sha256`; release acknowledgement only after host acceptance |

The GET route is exactly `/v1/code/runtime`. JSON is decoded exactly with the
existing host decoder: nested objects cannot be arrays, duplicate or unmodelled
members are refused, and optional members must match their canonical encoding.
For polling, include both `after_event_cursor: null` and `response: null`.
A non-narrowing approval omits `selection`, as the existing contract requires.
The host independently validates identities, digests, cursors, expiry, grants,
preimages, budgets and state. An HTTP bearer never becomes a kernel grant.

Authenticated frame reads and writes each use one 30-second monotonic deadline,
covering both prefix and payload; partial progress never renews it. A failed
frame permanently closes the bounded authenticated channel, so a late reply cannot be mistaken for
a subsequent command. Shutdown can wait for the bounded current exchange before
the owned child is reaped. This is not an acknowledged native cancellation.

Requests retain the existing 256 KiB body limit; a larger sealed request cannot
be started through this adapter. Responses are capped at 4 MiB and artifact
pages at 64 KiB. An oversized response is a retrieval refusal, not evidence
that the operation had no effect. Concurrent IPC requests return an explicit
busy refusal instead of building an unbounded waiting queue. Status and activity
remain available while IPC is busy. Activity records contain fixed operation
and outcome labels; exact run details remain with the existing canonical owner.

## Remaining scope

The native composition remains the fixed `failed-test-repair` scenario: only
`src/calc.py` is mutable, with the synthetic validation acceptance criterion
reported at startup. It retains its exact profile and workspace restrictions. This transport does not generalize them or
connect the newer manual strict-edit/test routes to that coordinator. The real
model/host workflow, arbitrary endpoint composition, original-checkout apply,
approved commit/push, MCP, measured development reruns and row-closing held-out
scores remain open. Preserve both authority systems while completing that
composition; do not replace either with a frontend agent loop.

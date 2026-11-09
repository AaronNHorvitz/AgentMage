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
An explicitly marked [linked worktree](coding-linked-worktrees.md) can also bind
its exact external Git metadata for read-only inventory and inspection. An old
workspace-only marker cannot authorize linked metadata reads.
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

For a worktree created by the manual coding service, use
`runtime-attach <private-runtime-state> <private-disposable-root> <created-worktree> <model> <scenario> [source-paths...]`
to transfer its live owner after inspection/handoff confirmation. After explicitly
releasing every run, `runtime-recover` returns that owner with a separate terminal
confirmation. See [terminal ownership](coding-owned-runtime.md#terminal-handoff-and-recovery)
for prerequisites, exact selections and lost-reply handling. These commands use
the existing owned API; the separate-process launcher keeps its default sources.

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

The `seconds` argument is 1–3600 monotonic running seconds and starts before
the initial selection request. System suspend is excluded. One clock and stop
latch cover selection, inspection, model start, advances, artifact reads and
approval waits; a new HTTP request never renews the clock.
Writing the complete review and waiting for a
terminal answer are interruptible and deadline-bound. Large reviews retain
partial write progress while the terminal drains. EOF, terminal I/O failures and
overlong answers abort without sending an approval or a denial; only a complete
answer line records the person's decision. A stop signal or deadline instead
returns control to the existing canonical cancellation path. SIGINT, SIGTERM,
SIGHUP or
the deadline requests canonical revision-bound job cancellation at the next
verified boundary; it never sends the API's generic cancellation route. A
pending HTTP read now closes on that same stop latch or deadline, with the
existing short read polling timeout. Its per-request ceiling remains 180
seconds, capped by the remaining run budget. A stopped request has an uncertain
outcome: the client never resends a start, tool approval or model advance.

The existing driver's release and cancellation reconciliation have one shared
two-second grace period, starting at their first request and never renewed.
After stopping, that path may inspect the bound job, request its original
revision-bound cancellation, read its next boundary without a tool approval,
read terminal evidence and attempt release. It cannot prepare or start another
run. A stalled ordinary request instead exits through the existing error path;
it does not infer a verified boundary or invent cancellation authority. A
prepared run may still receive the original driver's one release attempt.
Active runs can refuse release, and a lost release reply is also uncertain.
This terminal sends at most one release request, including error cleanup.

This is cooperative stopping, not a hard process-lifetime bound: connect/write
system calls retain their existing two-second ceiling, and output consumers
must keep draining terminal output. If cancellation does not reach a
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

The default native composition remains the fixed `failed-test-repair` scenario: only
`src/calc.py` is mutable, with the synthetic validation acceptance criterion
reported at startup. It retains its exact profile and workspace restrictions. The explicit Python project selection below generalizes the selected source and test layout.
The separate-process launcher has no manual creation owner. The manual service
can instead use the explicit [live-owner handoff](coding-owned-runtime.md#http-handoff-in-the-manual-coding-service)
to connect this same coordinator to later separately approved
[original-checkout application](coding-original-apply.md). Positive native
create/run/apply observation, arbitrary endpoint composition, approved commit/push,
MCP, measured development reruns and row-closing held-out scores remain open.
Both authority systems retain their existing boundaries; there is no frontend
agent loop.

## Python project selection

An explicit final `python-project` startup argument selects the repository's own
Python tests through the same coordinator, in place of the synthetic repair
scenario. Omitting that argument preserves `failed-test-repair`. For example,
inside the same bounded GPU reservation:

```sh
target/debug/agentmage-local-api serve-runtime-development 127.0.0.1:43129 <private-auth> <state-root> <disposable-root> <marked-workspace> gpt-oss new python-project
```

By default, the marked disposable repository must have one to sixteen parsed,
admitted Python source files under `src/` and a unittest package under `tests/`
(including `tests/__init__.py`). Discovery modules whose basename starts with
`test`, package initializers named `__init__.py`, directories named `test` or
`tests`, and hidden components are excluded from writes in both layouts.
All selected source paths are exact existing files; tests, other files, descendants of a selected
file path and file creation are refused. The sealed request shows the mutable
files. Startup metadata reports the selection rule and leaves `mutable_files`
null until the host prepares the exact request. Repository text cannot change
the startup selection. An empty or excessive source selection fails closed.

Startup selects a closed test definition: the verified system Python runs
`-I -B -m unittest discover -s tests -t . -v`. Isolated startup imports the
standard runner; discovery then admits the explicit worktree top level for
project imports. The test directory must be a package. The existing ban on
inline interpreter code remains intact; no ambient user site or repository
file replaces the initial runner import. This is definition
confirmation, not an execution grant. The command inherits no environment,
network or shell authority and retains the existing 30-second, 256 MiB and
bounded-output limits. Each invocation needs the existing exact approval.
Verbose stderr is normalized with `python_unittest_text_v1`; at least one test
must pass, with no failures, errors or skipped tests. Zero tests never establish
success. Project tests and approved source edits run in the same interpreter and
can alter the reported outcome. These counts are not independent evaluation;
scored use requires the separate frozen grading machinery.

The existing native model sees the task, mapped context, strict structured edit
contracts and test observations; the coordinator owns subsequent turns and
bounded correction. Its unchanged limits are sixteen turns/model calls/tool
calls, four repeated tool calls and four turns without progress. There is no
scripted fallback for this selection. Exact-path semantics are bound into the
session profile digest, and a resume cannot change between this definition and
the synthetic scenario. Planning language facts may cite additional Python
files; those complete facts grant no write authority. The exact scope remains
the edit boundary.

Use the existing `runtime-run` command to inspect, start, answer individual tool
challenges and verify the terminal result. This selection still uses only the
existing exact native development candidate profiles, not the separately
configured chat endpoint. The current lane refuses native executable trust;
component and synthetic terminal tests do not establish a working native
project run. Arbitrary project layouts, source creation, arbitrary endpoints,
manual strict-proposal composition, positive native original-apply workflow
observation, commit/push, MCP and measured parent acceptance remain open.

## Root-level Python tests

Select `python-root-project` explicitly, either as the final
`serve-runtime-development` argument or as `scenario` in the owned runtime's
attach request. This uses the same coordinator, source-selection rules,
model profiles, tools and individual approvals as `python-project`.
The terminal `runtime-run` client accepts both selections.

The fixed command is `-I -B -m unittest discover -s . -t . -v`. It discovers
root-level `test_*.py` modules and tests in importable packages below the root.
A repository with a top-level `test_calc.py` needs no `tests/__init__.py`.
Directories without package initialization are not recursively discovered;
this selection does not make an unpackaged `tests/` directory importable.
Use `python-project` for the existing explicit `tests/` package selection.
Choosing the root may execute additional tests in other importable packages;
review the exact command before approving it. There is no automatic layout
fallback, inline interpreter code or editable runner added to the worktree.

The command identity, arguments and validation definition are different from
the package selection. A distinct sealed constraint rejects cross-layout resume,
including absent, duplicate and conflicting layout markers. Each layout keeps
the positive-test minimum, no skips or errors for full success, offline command
confinement and separate execution approval. Standard project test output remains
non-independent evidence; the frozen scorer decides coding scores.

## Explicit Python source files

The owned service's `POST /v1/code/runtime/attach` additionally accepts an
optional `source_paths` list for either project layout. For example:

```json
{"source_paths": [["calculator.py"], ["package", "core.py"]]}
```

This is an additional field in the complete attach body, not a standalone
request. The control credential and explicit inspection and handoff consent
are required as before. The list replaces the default `src/` selection and
can name Python sources anywhere in the held worktree. Each path is a relative
array of components, never an absolute path or a slash-separated string.
One to sixteen unique paths are required. The host sorts them before sealing.
Every selected file must be an admitted, read, parsed regular Python file in
the repository map; a missing, excluded, unread or symlink input refuses the
whole selection. Discovery test modules (`test*.py`), package initializers
(`__init__.py`), any `test` or `tests` directory and hidden path components are
refused. The conservative test-name rule also excludes ordinary source modules
beginning with `test`. Initializers can replace subtree discovery through
`load_tests`; excluding them does not make project output independent grading.
An older default scope that included initializers has a different profile digest
and refuses resume; start a new run with fresh consent.
The [independent initializer-scope review](../reviews/4f73d93ea3ac-local-coding-python-initializer-scope.txt)
accepts that narrow repair and records another low limit: an explicitly selected
existing top-level Python file with a standard-library module name can shadow a
module the runner imports lazily, without being imported by tests. This remains
backlog in the same row and applies to explicit selection in either layout.
Only the unchanged frozen scorer supplies independent coding scores.

Omission or null preserves the default selection. An empty array is an error.
An explicit list never adds other files automatically. Descendant paths, new
files and out-of-list patches stay refused by the existing exact-file scope.
No creation-parent context is added. Status shows `selected_source_paths` as
canonical component arrays (or null for the default). `mutable_files` stays
null until prepare; the sealed work packet contains the complete actual list.
The existing terminal driver displays this status and packet before consent.

The factory retains this immutable startup selection on every repository
refresh. An exact digest constraint additionally rejects resume when an
explicit list is changed, removed, inserted or duplicated. This does not make
the process-held worktree lease durable. The separate-process development
launcher still exposes only the default source rule; the optional list is an
owned-attach API and host-factory feature. No model or repository content can
supply it through prepare, and every model call and effect keeps its existing
approval boundary.

The root validation now describes `python-root-project` in its sealed source
definition, rather than the package label. Its hash changes deliberately.
Existing root requests sealed under the earlier label are refused on resume
as profile drift; start a fresh run after inspecting and consenting. The
package-layout source definition is unchanged. Both layouts now bind the
complete profile, including the exact default write scope and validation
registry, into a required digest constraint. Earlier project requests without
this binding also require a fresh run. These are development selections, not
model admission or independent coding-success evidence.

## Task-aware repository context

The existing native context owner ranks its prepared, nonessential repository
sources against the sealed task objective before budget allocation. Matching
path words take priority over matching source words. Matches are case-insensitive,
distinct ASCII words, split at punctuation and underscores; repeating a word
cannot raise its score. The query uses at most 128 words of 3–64 bytes from the
first 8 KiB of the objective. Each source scan reads at most 64 KiB. Native repository JSON excerpts are
decoded before matching their content, so escaped line breaks, tabs and Unicode
escapes cannot hide source words. The decoder accepts the complete serialized
envelope of an admitted source, including expansion beyond 64 KiB when JSON
escapes tabs, newlines or control characters. The matching scan still reads only
the first 64 KiB of decoded content. Original excerpt bytes remain unchanged.
Plain excerpts retain bounded text matching. This lexical heuristic does not establish semantic relevance;
it has no multilingual or dependency-graph ranking claim.

Only eligible supporting items with the native repository-source identity shape
participate. Ranking changes their allocation item IDs, with a deterministic
original-ID digest as the tie-break. The complete source ID, revision, content
hash, excerpt, sensitivity and admission remain unchanged. The original source
list is retained. Duplicate original item IDs still refuse through the existing
composer. Stale/denied items, essential material, instructions, evidence,
corrections and other context sources keep their existing treatment. Repository
words cannot become instructions, tool permissions or grants.

The same composer retains every inclusion/omission record and performs the
existing budget checks, rendered-token preflight and whole-source reflow.
Retrieved files remain untrusted snapshots; they do not prove current file
content or authorize an edit. The context packet and its omission notice show
what was actually supplied. Tools retain their separate current-source checks
and exact approvals. No source, evidence input or provenance binding is removed.

This behavior is shared by the existing native coordinator callers, including
the HTTP/terminal projection. It does not admit the configured chat endpoint as
a native coding model. The deterministic context fixture is not a local-model
coding score; native positive execution and parent acceptance remain open.

## Live-owner handoff

The manual `serve-coding` service can now hand its successfully created worktree
into the same native coordinator within its own process. The control-only
`/v1/code/runtime/attach`, `/v1/code/runtime/owner` and
`/v1/code/runtime/recover` routes are documented in
[the ownership guide](coding-owned-runtime.md#http-handoff-in-the-manual-coding-service).
They never decode creation authority. Attached runtime operations retain this
page's existing strict requests and individual approvals. The separate-process
launcher retains its prior behavior and cannot recover manual ownership. Native
positive workflow and model measurements remain open.

## Endpoint completion safety

The model-independent completion boundary requires an ordinary endpoint stop.
A response stopped by the output-token limit cannot become a completion claim,
even when it contains valid, complete-looking JSON. Its accepted transport
observation still records the bytes and finish reason; it proves no task result.

The deterministic verifier returns an inert candidate. A registry pinned to the
exact claim identity must accept that candidate before completion. Synthetic
tests check rejection between two endpoint runs and between the native and
endpoint families, with a passing control for each. Full endpoint coordinator
and API composition must preserve that binding. It remains open, along with
T-303/T-306; these boundary tests establish no executable coding workflow or score.

## Return the strict coding owner for later apply

An embedding host can explicitly choose
`serve_coding_endpoint_strict_edits_returning_repository` or
`serve_coding_endpoint_edits_and_checked_tests_returning_repository`. The latter
attaches the supplied checker to the same original edit/test owner. Existing
service functions and development commands keep their discard behavior.

Export canonical events first. The person-facing control client must release
the exact terminal run through `POST /v1/code/endpoint/release`. Running,
faulted, uncertain or incompletely published runs refuse return. A known
terminal failure, denial or cancellation can return ownership; return never
means task success. The service returns `Some` only after successful release
and joined HTTP shutdown. Stopping without release returns `None`.

The original repository retains successful edit lineage, validation history and
actual result records. Its pending decisions are abandoned without dispatch or
fabricated results. Remaining live previews, completion callbacks, verifiers or
poisoned strict-owner state refuse extraction. No path, serialized receipt or
HTTP field can supply a substitute owner or select the return policy.

Obtain fresh read consent for the original checkout and worktree before using
that owner to prepare an [original apply preview](coding-original-apply.md).
Show the complete diff and obtain its separate exact approval before writing.
Continuation restores no old run, grant, disclosure consent, budget or endpoint
connection. It is process-local: service errors can drop ownership and process
termination cannot reconstruct it. Commit/push and durable recovery remain open.

## Review the original diff in the endpoint service

For explicit development continuation, choose either new command instead of
its ordinary discard variant:

```sh
agentmage-local-api serve-endpoint-edit-review-development <loopback-ip:port> <private-auth-directory> <private-launch.json> --approve-inspection-state-and-endpoint
agentmage-local-api serve-endpoint-coding-review-development <loopback-ip:port> <private-auth-directory> <private-launch-with-validation.json> --approve-inspection-state-and-endpoint
```

Launch configuration, native admission, stdin creation approval, per-turn
model disclosure and exact edit/test approvals stay unchanged. After exporting
canonical events, explicitly release the exact terminal run. Only successful
release makes its same original repository available through the three
[original apply routes and terminal commands](coding-original-apply.md).
Before release, those routes have no repository. No other manual tool or fresh
model run becomes available. The listener, authentication and process stop latch
stay the same; shutdown does not start another service.

Use `agentmage-code <loopback-ip:port> <private-auth-directory> original` to
inspect eligibility, then `original-prepare` and `original-apply` as described
in the linked guide. Preparation needs fresh read consent for both displayed
root identities; application needs its complete diff and exact approval.
A declined or cancelled run may have earlier successful edits; release does not
certify those edits, any test result or the task. Empty lineage cannot transfer
arbitrary worktree bytes. No commit or push occurs.

Embedding callers may choose
`serve_coding_endpoint_strict_edits_with_repository_review` or
`serve_coding_endpoint_edits_and_checked_tests_with_repository_review`.
These explicit variants leave the ordinary service and returned-owner variants
unchanged. The run and its connection are dropped on release; pending decisions
are abandoned. No old approval, grant, budget or disclosure is renewed. Export
events first, inspect after a lost reply, and never retry an effect automatically.
Extraction errors may lose ownership; this remains process-local continuation,
with durable recovery, native/model measurements and both parents open.

If release consumes the terminal run but extraction refuses, the service returns
HTTP 409 with `error: code.endpoint.original-review-unavailable`, `released: true`
and `original_review_available: false`. The old event record is gone and the
original apply routes remain unavailable. This is a failed continuation, not a
successful original review. The terminal displays the refusal and stops without
retrying or sending an apply request. Inspect state and disk; repeating release
cannot restore ownership. A lost response remains ambiguous and requires the
same inspection. Publication-state checks run before terminal consumption;
ordinary discard and returned-owner services keep their existing behavior.

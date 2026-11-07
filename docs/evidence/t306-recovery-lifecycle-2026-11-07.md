# T-303/T-306: worker shutdown and retained recovery

Base: `eb0e03a8d1cd4d05c4afd56c3abd7035a5a70360`. This is one same-row
repair under [Decision 0159](../decisions/0159-live-worktree-handoff-through-local-http.md).
The [independent HTTP ownership review](../reviews/eb0e03a8d1cd-local-coding-owned-http.txt)
is retained byte-for-byte with **Verdict: PASS** for that exact prior commit.
Its low L1 worker/owner loss and L3 operation-exclusion race are addressed
together here. Both mission parents remain open; independent core review of
this repair is required. L2 factory allocations and descriptor lifetime remain
backlog, disclosed in the [ownership guide](../guides/coding-owned-runtime.md).

## Behavior

Failed startup and continuation exits now use the session's existing draining
shutdown and worker join before discarding a session that never entered the
active registry. Registered embedded sessions keep the existing explicit service
shutdown; ordinary session destruction retains its prior behavior. A failed start
or continuation cannot return after discarding a session while that session's
worker still owns a composition. Shutdown keeps its prior
cancellation semantics: preserve an earlier accepted signal, signal only when
the correlation is known, drain bounded result channels, wait for native bounded
work to end and join the event pump. A slow startup refusal can take longer while
that bounded work returns. Discarded results establish no terminal outcome or
acknowledged job-control cancellation; incomplete histories stay incomplete.

Recovery stores the original live creation lease before attempting manual-owner
conversion. A busy conversion returns HTTP 409 `code.runtime.owner-held` and
retains that lease. A later explicitly consented recover request retries the
same slot. Success consumes it once. Poison still removes creation authority and
pending decisions. No path, serialized owner, marker or receipt can populate the
slot. Status adds the content-free `runtime_recovery_pending` field. Pending
recovery excludes manual operations and new attachment, with status/activity
still available. No grant, model permission or original-apply decision is issued.

Operation admission now checks runtime/recovery ownership while holding the
same mutex as ownership publication. Both ordinary and controlled manual
operations refuse after attach, even if their request passed the earlier HTTP
guard. The control-only ownership operation remains exclusive and available for
recovery. Settings and workspace mutations, inspection and manual cancellation
also recheck under their state lock.

## Verification

Locked/offline checks used two build jobs and four test threads, honoring shared
reservation waits (the first was approximately seventeen minutes). Each attempt
held one outer build slot through its focused checks and original fast-gate
planner/executor. The helper verifies the inherited lock descriptor and prefixes Cargo argv with `env` to
avoid a recursive reservation. Check selection, complete targets, cleanup
supervisors and timeouts are unchanged. Times below exclude the outer wait.

| Check | Observed result |
| --- | --- |
| Platform ownership/recovery focus | 9 passed / 312 filtered, no failures/ignores; 1.514 seconds including compilation |
| Host lifecycle focus | 3 passed / 588 filtered, no failures/ignores; 6.031 seconds including compilation |
| API coding focus | 20 passed / 5 filtered, no failures/ignores; 4.043 seconds including compilation |
| Rebuilt API binary | PASS; 1.309 seconds |
| Actual manual service/status/SIGTERM | Authenticated HTTP 200, no attached runtime, clean exit 0; 0.216775 seconds |
| Gate formatting / Clippy with warnings denied | PASS; 1.00 / 6.46 seconds |

The new failed-start fixture keeps its worker behind a gate beyond the 250 ms
startup visibility deadline, confirms startup has not returned an empty registry,
then releases the worker. Startup returns the original refusal only after joining.
Its job ledger remains `Running`, with no invented terminal decision; a duplicate
start refuses. The measured fixture takes 753,087 microseconds.

The new recovery-slot fixture refuses twice while a real lease clone remains
live, then records an actual authorized native write. After that clone ends,
recovery consumes the original slot once. Old apply consent refuses; the original
checkout changes only after a fresh full diff and separate apply decision.
The native write/apply component takes 409,579 microseconds. The existing poison
fixture now uses the same slot conversion and still clears creation authority
and pending approvals. The intentional caught poison panics are passing tests.

The API fixture checks late operation admission directly against an attached
synthetic transport, for both client scopes, and confirms only the control-only
ownership transition can acquire the exclusive lease. Its actual loopback
exchanges retain control authentication, runtime forwarding, refusal of mixed
manual requests and available status/activity. These are component/fixture
measurements, not model latency or coding scores.

The initial complete test run exposed a deadlock in the first implementation:
joining every session in `Drop` blocked an existing abrupt-host-restart fixture
that deliberately holds its worker behind a gate while dropping the host. The
interrupted log is retained without a complete-result claim. Only that batch's
own gate executor was signalled; its original supervisor stopped the owned test
process. No other worker or reservation was touched. The correction joins at
failed-start and failed-continuation exits, preserving the existing abrupt-host
fixture semantics and explicit embedded-service shutdown. Final checks below
must use this corrected source; the initial source manifest/logs remain retained.

The complete corrected target run is **FAIL**: 916 passed, 72 failed, 63 ignored,
0 filtered, across 22 target summaries; 123.45 seconds including compilation.
All 72 failure identities match the prior batch exactly: 53 host and 19 Linux
tests under deferred T-301.1. The restart fixture that exposed the initial
approach passes on the corrected source. Both new tests and the extended API
admission/poison checks pass in the complete run. No ignore or threshold changed;
the gate remains FAIL. Formatting, Clippy, review retention and the initial
three-file Markdown check pass. Final Markdown/source-policy validation covers
the completed evidence text without changing any of the seven frozen Rust inputs.

Raw initial and corrected logs, both source manifests, the complete policy-source
input manifest, helper hashes and the exact failure comparison are retained in
the private review request. The source-policy check is source validation, not the
T-310 acceptance gate. No SBOM or historical hash-bound evidence is regenerated.

## Limits

The failed-start check uses a scripted coordinator and actual worker threads.
The recovery-slot check uses synthetic creation provenance, real held directories,
the existing kernel-authorized atomic writer and a separately approved original
application. It does not prove native Git creation, model inference or a complete
HTTP create/run/recover/apply workflow. HTTP admission checks use a synthetic
transport peer, not a native coordinator. Successful native HTTP attachment and
the API's positive pending-recovery branch remain unobserved under T-301.1.

The configured chat transport does not supply the coding coordinator's exact
model and rendered-token admission facts. It is not attached as a coding model
by this repair. Durable ownership, new-file transfer, approved commit/push, MCP,
native failed-test correction and parent measurements remain open. No GPU,
model, evaluation task, hidden test, held-out input, threshold, score, dependency,
SBOM, lifecycle status or release authority is changed.

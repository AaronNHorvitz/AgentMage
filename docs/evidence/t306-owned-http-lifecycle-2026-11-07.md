# T-303/T-306: live ownership through the coding HTTP service

Base: `b915775f5bb7cf21301878d8f00d920781c590e7`. This is one non-closing
implementation batch under [Decision 0159](../decisions/0159-live-worktree-handoff-through-local-http.md).
Both parent rows remain open. The prior [independent core review](../reviews/b915775f5bb7-local-coding-live-owner-bridge.txt)
is retained unchanged with **Verdict: PASS** for its exact commit, accepting only
the prior library bridge. Its low L1 is repaired here; its low L2 host boundary
execution gap remains open. The prior evidence's direct-path refusal statement
is a property established by code reading, not a newly observed coordinator run.

## Implemented behavior

The existing manual `serve-coding` service now has control-only attach/recover
routes. Attach consumes that service's live successful creation owner, checks the
displayed worktree identity before activation, validates the exact private
marker, selects only the existing native model/scenario profiles and constructs
`CodingDevelopmentRuntimeFactory`. It attaches the retained lease and serves
`LiveCodingRuntimeService` through the existing runtime routes. There is no
second loop or serialized ownership. The separate-process launcher is unchanged.

This startup needs explicit inspection and handoff consent. Runtime state cannot
overlap the manual authority store or held repositories. Missing creation,
concurrent owner references, poison, root mismatch and invalid activation refuse.
Ordinary handoff refusals return the owner intact. Creating a lease invalidates
pending manual decisions even if later composition refuses. A poisoned owner
stays poisoned. No startup approval becomes a model call, tool grant or original
application approval.

While attached, manual repository/chat/workspace effects refuse, and status and
activity remain available. Embedded job control derives a fixed authenticated
control-client scope inside the host and uses its existing canonical ledger.
Recovery requires explicit consent, no concurrent handler references and explicit
release of all prepared/active runs, including terminal runs. Dropping the empty
service releases the factory's lease before returning the same manual owner.
Subsequent original application still needs fresh read consent, a complete
current diff and a separate exact decision.

The manual coding process now registers the same interrupt, termination and
hangup stop flags as the existing runtime launcher. Embedded shutdown signals the passive cancellation probe, preserves an existing
accepted cancellation, drains worker results/events and joins workers before
returning. It waits for native bounded calls to end; it cannot safely detach a
Rust worker and report completion. It fabricates no terminal outcome or accepted
job-control decision. Unreconciled canonical records stay open for existing
recovery rules. Hard process termination still loses process-only ownership.

## Verification

All checks used locked/offline dependencies, two build jobs and four test
threads. Shared reservation waits were honored; no GPU or model was used.

| Check | Observed result |
| --- | --- |
| Local API compilation | PASS; 10.14 seconds of compilation after the reservation wait |
| Native-write/ownership focused tests | 8 passed, 312 filtered, no failures/ignores; 1.27 seconds |
| Scripted host lifecycle tests | 2 passed, 588 filtered, no failures/ignores; 0.29 seconds |
| Actual loopback API refusal tests | 4 passed, 21 filtered, no failures/ignores; 0.59 seconds |
| API binary build | PASS; 3.30 seconds |
| Real manual service startup/status/SIGTERM | Authenticated HTTP 200, no attached runtime, clean exit 0; 0.235947 seconds |
| Formatting / Clippy with warnings denied | PASS; 1.10 / 12.07 seconds |
| Complete affected-crate targets | **FAIL**: 914 passed, 72 failed, 63 ignored, 0 filtered; 131.96 seconds including compilation |

The complete failure identities match the prior batch exactly: 53 host and
19 Linux tests, with no new identity. They remain failures under T-301.1. All
seven newly added tests pass in the complete run. The synthetic poison cases
intentionally catch panics and pass; they are not extra test failures. The native
write/original-apply component prints 421,786 microseconds, and four HTTP refusal
exchanges print 2,346 microseconds. These are fixture/component timings, not agent
quality or model latency measurements.

The real process check rebuilt the actual API executable, initialized private
test credentials/state, started `serve-coding`, read authenticated status and
sent SIGTERM. It exercised the stop-flag path and clean process exit, with no
model, native worktree or synthetic success claim for the handoff workflow.

To avoid repeated reservation waits, the focused driver held one shared build
slot through these checks and the unchanged fast-gate plan/executor. It verified
the inherited reservation descriptor and prefixed Cargo commands with `env` to
avoid recursively acquiring the same lock. The selected arguments, complete
targets, timeouts and original cleanup supervisors were unchanged. Per-check
times above exclude the outer wait. This is a reserved execution of the original
plan, not a claim of an uninterrupted ordinary CLI invocation passing.

The initial Markdown step exited 127 because the pre-existing shared Node path
disappeared during the long reservation wait. That failure remains in the raw
gate log. The same already-installed Node 24.15.0 binary and licence were retained
in task-owned shared scratch, with the unchanged pinned SHA-256
`d1de76d8edf2fededf6f8b30d244e2c0529ac607923a018283b77e9c74bd932c`;
no download occurred. The exact five-file Markdown step then passed. The complete
Rust target result remains FAIL. Later Markdown/policy validation covers this
final evidence text; no passing full-gate result is invented.

The final audit binds all eight Rust sources, the complete policy-source input
manifest and raw logs; it compares every failure identity and preserves ledger
statuses and default ignores. The exact private paths, checksums and commands
are retained in the independent review request. The source-policy run is source
validation, not the T-310 long gate.

## Limits

API tests exercise actual loopback HTTP refusals without creating fake ownership.
The host lifecycle tests use a scripted coordinator and real worker threads; the
native-write tests use synthetic creation provenance with actual held roots,
kernel grants and atomic writes. These are distinct from native Git creation,
full host boundary execution, actual model inference and a complete HTTP
create/run/apply observation. The lane's executable-ownership blocker remains
T-301.1; no ownership or sandbox check is relaxed.

No GPU/model process, evaluation task, hidden test, held-out task, threshold,
score, dependency, SBOM or status-model input changes. No new row or ignore is
added, and no row closes. Native positive execution, arbitrary configured model
composition/rendered-token facts, fail/repair observations, durable ownership,
new-file transfer, approved commit/push, MCP and parent measurements remain open.
Held-out evaluation remains reserved for row closes. No release or owner-ready
claim is made.

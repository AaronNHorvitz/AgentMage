# Decision 0093: Bound Development Host Startup and Direct-Child Cleanup

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0054, 0061 and 0081; current owner restart |
| Scope | AMR-01 development host startup, exit and process-local ownership |

## Finding

The existing development launcher passes a blocking child stdout pipe to the
bounded-size launch-envelope parser. An incomplete envelope can keep its
`read_exact` calls waiting without a deadline. The direct-child exit and destructor
paths also call blocking `Child::wait`. This is a source finding, not a measured
indefinite hang or a native workflow result.

The CLI installs its stop-signal handlers only after reading the launch envelope.
A signal during that read can therefore exit the CLI before owned cleanup. Treat
startup cancellation as part of this same lifecycle work unit.

## Decision

Keep the exact sibling-host validation, closed arguments, launch-envelope parser,
peer authentication and existing platform process owner. Bound the inherited
stdout reads with a nonblocking adapter and one monotonic startup deadline of
120 seconds, established before launch. Fragments and repeated reads do not renew
that deadline. Consume the direct pipe once. Malformed or incomplete frames never
produce credentials or a fallback connection. Timeout has a static diagnostic.

Install the existing SIGINT/SIGTERM flag handlers before spawning the host and keep
their registrations owned through child cleanup. Observe the same flag between
nonblocking reads without resetting it; a stop request refuses even a buffered
complete envelope. Recheck before connecting and before runtime preparation. Reuse
the existing runtime cancellation port after startup. Unregister this invocation's
callbacks on exit, including partial installation failure. A cancelled startup
returns the existing cancelled exit class only after the directly owned child has
been reaped; cleanup uncertainty takes precedence. This does not claim that every
later blocking prompt or kernel syscall can be interrupted cooperatively.

Allow at most one development host handle per calling process. Reserve its slot
before spawn; ordinary pre-spawn refusal releases it. A successful spawn keeps the
reservation until the exact direct child is reaped. Failed or uncertain cleanup
refuses replacement admission for the remainder of that process. Use private,
non-cloneable ownership; no caller-selected program, PID, unit or process group.

After normal IPC shutdown, allow ten seconds for direct-child exit. Expiry requests
termination of that exact owned child and allows at most three additional seconds
for polling/reaping. An exit observed after the graceful deadline does not convert
the timeout into success. Error/destructor cleanup likewise uses a three-second
polling budget. Never block in `Child::wait`, signal by name, detach a reaper or
accumulate an orphan list. If final cleanup cannot be proved, retain the one child
handle and refuse replacement admission until process exit; do not advertise a
released slot. Retention is deliberately bounded to this one owner.

These are cooperative polling deadlines, not a hard-real-time guarantee for a
kernel-stalled filesystem or process syscall. They prove only the directly owned
host's disposition. Native workers, model descendants, manager jobs and cross-host
crash recovery retain their separate owners and open verification gates. Killing
a host does not prove those descendants stopped or reconcile canonical effects.
No persisted recovery store, new supervisor, IPC schema or effect authority is added.

## Verification

Use actual private pipes for empty, partial, fragmented, malformed and closed-frame
cases; keep the existing parser and exact bounds. Verify that progress cannot renew
the original deadline. Exercise normal and failing direct-child exit, forced
termination, repeated observations, destructor cleanup and subsequent admission
with test-owned synthetic child processes. Test concurrent reservation and uncertain
admission without leaving a live child. A synthetic child is component evidence,
not a native host/model workflow. Retain failures and exact source identities.

Exercise the actual CLI with a synthetic incomplete-envelope sibling, including
SIGINT and SIGTERM while that child is alive. A test-only lifeline may clean up a
failed observation, but a passing result must establish the child's exit before
closing that lifeline or sending any test cleanup signal. Keep this separate from
runtime cancellation, native tool cleanup and model qualification.

Rebuild the actual CLI/host and repeat the relevant startup diagnostic/preservation
checks under the existing lane limits. No unavailable native prerequisite is
bypassed. Manual, real-model, native workflow, independent and release acceptance
remain separate. Finish all source edits before one evidence regeneration pass.

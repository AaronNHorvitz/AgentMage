# T-306: Bounded dormant signer process capture

This same-row increment addresses the inherited-output-pipe limitation recorded
under Decision 0191 on immutable base
`6033f03afb9e19874cdc67df22fc13f23c83e817`, still under independent core review
when work begins. T-303/T-306 remain open; no row or sub-row is added or closed.
Its independent report arrives during verification with **Verdict: PASS**, no
high or medium finding, and is retained byte-identically. Low L1 (SHA-256
signature-header selection) and L2 (exact signing-key selection) remain same-row
backlog, alongside the prior candidate scratch and descriptor findings.
Retaining that report does not change the frozen sources.

Both dormant direct-process boundaries now reuse the supervisor's existing
nonblocking capture and exact-child polling primitives. Signed Git no longer
joins blocking reader threads. Signer inspection no longer calls unbounded
process output collection or checks the byte limit only after allocation.
Each stream retains its existing four-MiB ceiling. Polling bounds bytes and
syscalls so continuous output cannot starve cancellation or the deadline.
Complete stdout, stderr and immediate-child status are required; partial output
is discarded on failure. Exit status remains separate from captured bytes.

Listing and export share one thirty-second inspection deadline. Signed creation
and verification retain the original commit-session deadline. These deadlines
also apply after the immediate child exits while an inherited pipe remains open.
Failure permits at most half a second of exact-child kill/reap polling, without
blocking waits, output-thread joins, PID/group lookup or detached reapers.
The opaque child enters a private retained slot before pipe initialization.
Unresolved reaping, a busy owner or a poisoned owner refuses subsequent direct
capture admission. Initialization failure follows the same bounded teardown.
The existing supervised control subprocesses keep their unchanged byte ceiling.

Eight new actual local child/pipe methods exercise separate streams and exit
codes, exact limits and overflow on either stream, open writers after immediate
child exit, live cancellation, output pressure, retained/busy/poisoned owners,
pre-cancellation, expired/shared deadlines, failed spawn, recovery after failure
and missing pipes. The open-writer experiment uses a separately owned fixture
child, checks that it is still alive when capture refuses, and reaps it in the
fixture. It deliberately demonstrates that bounded capture does not establish
descendant cleanup. Initial focused verification passes the first seven methods;
the missing-pipe method is added before the complete affected gate.

The fixed-source complete affected gate remains **FAIL**: 1,354 Rust passes,
the exact same 72 inherited failure identities and 64 existing ignores across
26 unfiltered targets, with zero measured or filtered tests. All eight new
methods and five affected controls pass. Formatting and Clippy pass. Both source
hashes stay unchanged. The run takes 835.03 seconds including shared reservation
waits: formatting 562.82 seconds, Clippy 29.11 seconds and Rust 241.59 seconds.
No command times out. Its retained raw log SHA-256 is
`93f349beb1c3baa8bcfaed7983dbfb1bc3b8d255375201b96358cc9b92ec3f21`.
Exact per-target comparison returns zero as data only, never a passing full gate.
Final prose lint, whitespace, public-content checks, unchanged task-table
comparison and byte-identical retained-review checks pass after verification.

This is not signer confinement. Signing, inspection and signature verification
remain dormant direct subprocesses; the capture helper cannot establish that
their descendants terminated, prevent external-keyring access or prove offline
execution. They require the separate confined signer boundary and whole-unit
cleanup before activation. Existing exact plan, signature, approval and
preservation checks remain required. Candidate activation, reviewer/artifact
provenance, separately approved commit/push, durable recovery, native admission
and model measurements remain open. No model/GPU process, evaluation task,
hidden/held-out test, prompt, threshold, dependency, supply-chain output,
lifecycle, scoreboard or release changes. Independent core review is required;
neither parent closes.

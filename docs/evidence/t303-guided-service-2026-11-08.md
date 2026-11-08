# T-303/T-306: actual-service coding consumers

Date: 2026-11-08. Tier: core. Immutable predecessor:
`8c652954e4bf73eafc7220cb8ff2bcdcb6d831ff`.
This batch adds tests and documentation only. No row is added or closed;
independent exact-commit core review is required.

## Observed behavior

The actual terminal executable talks to the actual authenticated coding API
executable, with its original canonical repository owner and permission paths.
The fixture provides an unowned directory, not fabricated worktree creation
provenance. No native admission rule is bypassed.

Three new service-consumer methods cover:

- Both guided commands, with and without model-context consent, before and
  after restarting the service: eight refusals before any generation or
  approval prompt. A nonblocking upstream listener observes no connection;
  activity contains no proposal or repair operation.
- The repair command, before and after restart: refusal before a failure-share
  prompt. Repeated direct repair requests with valid current selection, root
  and endpoint pins and claimed disclosure consent still cannot invent an
  owned failed receipt. No upstream connection or repair capacity is created.
- The individual worktree proposal command through the original credential-owning
  HTTP transport: no generation without disclosure consent; exactly one request
  after consent, for each complete and output-limited stream. The synthetic
  upstream observes the exact model, token ceiling, streaming request and
  selected source. An unselected sentinel is absent from the request. The real
  service forwards context and terminal proposal/error events; even complete
  generation cannot adopt the unowned directory or prepare an edit.

All five inspected repository/edit/command/validation/staged-change states
remain unchanged. Sources and the empty management directory are preserved.
Each fixture stops and joins its owned service or upstream worker. Upstream
request headers, bodies, socket waits and lifetime are bounded.

A fourth method uses the existing recording synthetic API peer. For both the
initial proposal and the repair, it substitutes the original prepared reply's
operation, worktree root or creation receipt, keeping both pending inspections
identical to that substituted reply. All six cases stop immediately after
preparation, before another inspection or edit consent. Earlier attempts are
not repeated. The synthetic fixture never writes source or launches tests.

## Verification

The unchanged ordinary API baseline is **10 targets, 166 passed, zero failed,
ignored or filtered**, run through the shared build reservation. The first
focused run failed all three new service methods in their shared fixture:
the selection PUT reply deliberately omits the disclosure-status field.
The fixture now checks that field through the original GET route and binds
its selection identity and revision to the PUT reply. The failed log remains
retained. No product behavior or existing assertion was changed.

The corrected focused runs pass all three service methods and the separate
original-preparation method. Captured guided-refusal observations took
34,598 through 41,505 microseconds each; the three individual proposal cases
took 51,136 through 53,945 microseconds. These component timings include their
explicit state inspections, not fixture setup or reservation waits, and are
not local-model latency or a coding score.

Two isolated discrimination probes remove one original production guard at a
time: worktree root, then creation receipt. The unchanged new assertion fails
each probe with one failed test and exit 101. The original production bytes
are restored in `finally` and SHA-256 checked, within the shared reservation.
No history is changed. Final validation must use the restored sources.

The final ordinary scoped gate is **PASS: 10 targets, 170 passed, zero failed,
ignored or filtered**. The identity audit retains all 166 baseline passing
identities and exactly the four new methods. Formatting, strict Clippy and the
six recorded gate commands exit zero. The gate took **502.13 seconds**, including
individual shared reservations; the formatting command spent most of its
462.70 seconds waiting for a slot. All three Rust inputs stayed hash-frozen,
with no source edit after the gate began. This final gate establishes the
committed test-source coverage; focused observations supplement it.

Current documentation, retained-review format, privacy, exact scope and
unchanged ledger rows receive separate lightweight checks after recording
these results and retaining the newly arrived report. This is a scoped API
gate, not whole-workspace or native execution acceptance. Raw baseline, first
failure, corrected focused observations, discrimination probes, source freeze,
full gate and identity audit remain private for independent reproduction.

## Independent predecessor acceptance

The deadline increment's [independent core review](../reviews/8c652954e4bf-local-coding-workflow-budget.txt)
arrived during validation: **Verdict: PASS** for the exact immutable predecessor,
with no high or medium finding. Its report is retained byte-identically.
The reviewer independently reproduced all 166 tests, formatting, strict Clippy
and documentation checks with the installed runtime available.

Its low findings remain same-row backlog: complete arguments in invalid-budget
tests, second-signal escalation, complete monotonic-clock wording, and prior
focused-run hash traceability. The guide now makes the clock's exclusion of
system suspend explicit. The prior final frozen gate, rather than mismatched
focused-input metadata, establishes predecessor source verification. No review
is edited and neither parent row closes. The current test batch still requires
its own independent exact-commit review.

## Limits and remaining work

The guided service tests establish refusal on missing ownership; they never
reach successful native test preparation. The individual proposal test uses
synthetic generation through the real transport. The successful guided repair
sequence remains covered by the existing synthetic API peer only. This
partially addresses actual-service consumer coverage; it does not close that
same-row backlog. Direct coverage of the original prepared root and creation
pins is submitted for independent acceptance.

Native executable-ownership admission remains blocked in this lane and is
unchanged. Successful native/model edit/test/repair, autonomous endpoint
composition, durable recovery, approved Git and parent measurements remain
open. The in-flight predecessor remains immutable. No evaluation inputs,
hidden/held-out tests, scorer, threshold, dependency, model/GPU, SBOM, status
model, scoreboard, release or push change. Deferred T-301.1 is not rerun or
waived. No independent acceptance or parent closure is claimed.

# T-303/T-306: Report release without original review

Date: 2026-11-09. Base:
`c86aabe0bb9cc47c515a2a6fb278c2b239d161f6`.
Same-row core repair; both parents remain open. No new row or sub-row.

The existing endpoint continuation could consume a terminal run and then return
an internal service error if original-repository extraction refused. The HTTP
worker discarded that error without a response. The client could only observe
a lost reply, although release had already happened.

The review service now responds with HTTP 409 and a content-free stable error,
`code.endpoint.original-review-unavailable`, plus `released: true` and
`original_review_available: false`. The message requires inspection rather than
another release or approval. Internal diagnostics and private state are not
exposed. The publication mutex and already-published state are checked before
consuming the terminal run. A repeated release after publication still refuses
as not held. Successful responses and ordinary discard/return services retain
their existing contracts. Extraction can still discard process-only ownership;
this repair reports that failure and does not reconstruct it.

## Verification

The actual HTTP/coordinator regression uses a synthetic boundary whose converter
refuses with either configuration or I/O error. A known cancelled terminal is
released once; both cases receive the complete explicit refusal. The event
record, advance and repeated release then refuse as not held. Conversion occurs
exactly once; original-apply routes stay unavailable and the listener remains
usable. Observer, foreign-binding and unstarted-release attempts never reach
conversion. No model request, tool dispatch, native repository extraction or
file transfer is claimed by this synthetic-boundary test.

A separate actual-terminal regression uses a synthetic HTTP peer returning the
new refusal. It observes the visible error, nonzero exit, one release confirmation
and exactly the selection/release requests, with no retry or follow-on request.
The existing complete native-transfer/terminal tests remain part of the
unfiltered affected API gate. These separate tests do not establish native
extraction refusal through a terminal in one combined workflow.

The complete affected API gate is **PASS**: 240 tests across eleven unfiltered
targets, with zero failed, ignored, measured or filtered tests. Formatting and
all-target Clippy pass. Three Rust hashes are unchanged across the gate. Both
new methods and five existing ownership/transfer regressions are checked in the
raw log. The initial focused compile failure (missing test-only standard I/O
import) is retained privately; the import was added and the focused service and
terminal regressions passed. No existing test, threshold, hidden test or frozen
evaluation task was changed. Earlier host/Linux failures are outside this
API-only selection and are not cleared.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 0.30 |
| Clippy, all API targets | 0 | 1.80 |
| Tests, all API targets, no fail-fast | 0 | 51.18 |
| Complete affected gate | 0 | 54.96 |

Raw log SHA-256:
`6b45f563bdde64ddfbe0671ecb07cd74c3c52d5bb016084718eee83f86882d1c`.
The private reproduction bundle binds the complete log, source hashes and
focused results. Every Cargo command reserves the shared build slot with two
build jobs and four test threads. Final prose and retained-review checks run
separately after these source checks.

## Review and remaining work

This repairs the existing low extraction-error response finding from the
original-review increment. Its independent report remains unchanged. The base
terminal-transfer increment now has an independent
[Verdict: PASS](../reviews/c86aabe0bb9c-local-coding-terminal-original-transfer.txt)
for the exact base commit, with no high or medium finding. That report is
retained unchanged. Its low root-alias preflight and standalone recovery-client
coverage findings remain same-row backlog. Neither verdict closes a parent or
sub-row. Independent core review of this repair is required.

Native executable trust remains unchanged. Positive native/model workflow,
durable crash recovery, approved commit/push, coding measurements and both
parents remain open. No model or GPU work, download, permission expansion,
dependency or supply-chain regeneration, status promotion, scoreboard entry,
long gate or release is part of this repair. Lost replies remain ambiguous;
inspect state and disk before any later action.

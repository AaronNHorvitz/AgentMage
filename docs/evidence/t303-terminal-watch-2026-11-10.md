# T-303/T-306: bounded terminal observation

Status: scoped fast gate PASS; independent core review required.
T-303/T-306 remain open without new rows or sub-rows.

The existing terminal client gains `endpoint-watch` for an explicitly delegated,
exactly pinned run. It uses only the ordinary credential and the existing fixed
request and event routes. It renders each checked event without advancing,
approving, disclosing to a model, cancelling or releasing the original run.
Its full-prefix validation reuses the original kernel event state machine.

A fully framed busy response may wait and poll the same observation index.
All other refusals and transport errors stop. Effect callers retain their
original no-retry behavior. The original monotonic deadline, event/page/output
budgets and signal latch bound observation. Blocking stdout writes remain
outside preemption. Output is observation data, not a replay record or an
independent task-success verdict.

The preceding commit returned independent PASS during this batch, without high
or medium findings. Its exact report is retained unchanged. Low L1 is corrected:
release removes event pages, while the sealed request remains readable for the
service lifetime. Low L2, native launch-to-service forwarding coverage, joins
same-row backlog. No row closes.

## Validation

The complete affected fast gate **PASSES**: **266 passed, zero failed,
ignored, measured or filtered**, across all **11 unfiltered local-API targets**.
All five new integration methods pass. Formatting, Clippy with warnings denied,
Markdown lint, retained-review format and whitespace checks pass. All four Rust
source hashes remain fixed through the complete run and documentation recording.
No check times out. The broader inherited host/Linux gate remains FAIL with its
72 failures, outside this selected scope and not rerun here.

The full gate takes **55.19 seconds** including reservations: formatting 0.30
seconds, Clippy 1.90 seconds and Rust tests 51.47 seconds. Raw log SHA-256:
`1e7d00ecc57618c02a74331bfa31ddd7d760f6fe2d53aa0b4a79693357681df0`.
The preceding focused run passes all five new methods (39 existing methods
filtered only for that preliminary run). Its raw log SHA-256 is
`97c820b02f4f11b03cad7a32ccc986ae5821d3ee10048d58983adf8952c232ab`.
The private review bundle retains both logs, source hashes, selected commands and
exact-count validation. Its full-gate runner reserves heavy checks internally;
do not outer-wrap it. Final documentation recording gets separate light checks,
with no source changes or repeat of the complete Rust suite.

The actual terminal subprocess starts before coordinator advancement, then follows
another caller's original cancellation and coordinator terminal publication. Its events
match every original HTTP page, and the kernel verifies their complete chain.
A second watcher reads the identical retained terminal chain. A deliberate
invalid neighboring control-token symlink demonstrates that only the ordinary
credential is opened. These tests use synthetic model/tool ports; no endpoint
model connection or tool execution occurs.

Other integration methods refuse missing service delegation, foreign/stale pins,
invalid duration or sharing flags and unavailable released event pages. Actual
SIGINT, SIGTERM, SIGHUP and timeout yield exit one while leaving the original
run held and its event history empty. A scripted peer tests individually sealed
wrong start digests, broken links, foreign identities, wrong sequence, shrinking
totals, both local and run event ceilings, and advertised trailing events after
a terminal event. It verifies ordinary credentials, fixed routes and exact
indices. Explicit framed busy replies preserve that index; authority, binding
and stopping refusals stop, and malformed HTTP-200 busy lookalikes refuse.

Two early focused runs exposed test-side races with the service's explicit busy
response (two passes/two failures, then three passes/two failures). The watcher
now waits only on framed observation-busy replies. Test-side controls wait only
when an explicit `api.busy` response proves dispatch did not begin; transport
errors and other refusals are never retried. The final focused and unfiltered
gates pass. One initial light check also found an extra blank line in TASKS.md;
it is corrected before the complete gate. No threshold or existing test changes.

## Remaining work

Native executable admission remains blocked by the lane's unchanged ownership
mapping. Native/model coding, independent candidate checker/artifact integration,
complete original-owner commit approval, signer/repository recovery, approved
push and durable execution recovery remain open. This observation adapter changes
no model prompt, coordinator, evaluation inputs, hidden or held-out tests,
threshold, scoreboard, dependency or supply-chain output. No GPU, download,
public push, release or owner-readiness action is performed.

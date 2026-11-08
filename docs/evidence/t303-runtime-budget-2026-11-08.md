# T-303/T-306: Native terminal HTTP stopping budget

Date: 2026-11-08. Tier: core. Builder evidence; independent acceptance pending.
Predecessor: `ae0edbd912b24d5739521d5bab7baa299f83b8b3`.
Both parent rows remain open. No row is added or closed.

## Running change

The existing native `runtime-run` driver previously checked its running-time
budget at approvals and coordinator boundaries, while a pending HTTP read could
wait for the separate 180-second request ceiling. The terminal now shares the
original signal latch and monotonic clock with every ordinary API request,
including the first selection. An observer shares the latch and clock without
registering new handlers or owning the original registrations. No request or
approval renews the clock.

The original interactive driver still owns event verification, approvals, job
control and release. Its existing cleanup path receives one two-second grace
period shared across requests. After stopping, only job control, terminal
evidence and a response-free cancellation advance can use that path. A prepared
or ended run can receive the original release attempt. No second runtime,
permission grant, automatic retry or generic service cancellation is added.
The terminal refuses a second release dispatch even if the shared driver's
error cleanup asks again after an uncertain first reply.

Stopping a pending request reports an error and leaves its dispatched outcome
uncertain. It never proves cancellation, undoes effects or replays approval.
An active run may refuse release. Reconciliation can also fail or time out;
inspect the original job and disk before another action. The
[guide](../guides/coding-runtime-api.md) describes the scope and cooperative
limits, including system suspend, connect/write bounds and output backpressure.

## Verification

Eight executable terminal methods cover 48 synthetic cases: stalled header and
body reads during selection, preparation, start and an approved advance;
deadline and all three stop signals; bounded release after a lost start reply;
a single clock across inspection and start approval; interrupted artifact and
optional terminal-metadata reads; canonical cancellation on every stop signal
and deadline; a shared grace period across delayed job/control replies and a
stalled next boundary; and no second release after a lost or refused reply.
No stopped request produces a final success report. Verified cancellation still
uses the original revision-bound job control and event verifier, without a tool
approval response. The peer executes no model or native effect.

The actual terminal against the original API refuses its absent native runtime
with the same 428 response as direct inspection. There is no approval prompt,
model disclosure or effect; all six inspected states, original bytes and the
empty management directory remain unchanged. This is refusal evidence only.

The first focused development run passed six methods and failed one fixture
call-count assumption: the original driver reads optional job metadata after
unavailable declarations. The fixture now supplies that response. Its raw
failure remains retained. A gate started before the queued formatter completed
was deliberately interrupted before compilation; it exited 1 and is not a pass.
Neither run certifies the final source snapshot.

After formatting and the last source edit, the ordinary complete scoped gate
passes **197 tests across ten source targets, 0 failed, 0 ignored, 0 filtered**.
Exact identity comparison retains all 188 independently accepted predecessor
tests and adds exactly nine methods: the eight synthetic terminal methods and
the actual-service refusal method. All four changed Rust inputs were frozen
before this gate and remain unchanged; no source edit followed it.

All seven gate commands exit 0: whitespace, untracked additions, retained-review
format, documentation lint, Rust formatting, strict Clippy and complete package
tests. Total elapsed time is 586.97 seconds including shared reservations.
Formatting including its reservation takes 531.33 seconds; Clippy takes 2.41
seconds and complete tests 50.53 seconds. The already installed Node runtime and
its licence are shared through hard links; no download or binary copy. Current
scope/privacy, unchanged-ledger, retained-review, source-freeze and lightweight
checks also pass after this evidence update. This is the scoped development
gate, not full-workspace or native acceptance. Deferred T-301.1 is unchanged.

## Independent review and remaining work

The predecessor's [independent core review](../reviews/ae0edbd912b2-local-coding-guided-original-transfer.txt)
has **Verdict: PASS** for its exact commit, with no high/medium finding. The
report is retained unchanged. Its low L1 (created/mixed-kind transfer usability)
and L2 (additional refusal and stopping coverage) remain same-row backlog.
Earlier lows remain. This increment requires separate core review.

Native successful creation/model/edit/test/transfer, autonomous endpoint
edit/test composition, durable recovery, approved Git and parent measurements
remain open. No evaluation, hidden/held-out test, scorer, threshold, model,
GPU, dependency, SBOM, status, scoreboard, release or push change occurs.

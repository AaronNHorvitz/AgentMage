# Decision 0200: Bounded terminal run observation

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decisions 0162 and 0199 |
| Scope | Ordinary-credential terminal observation of an explicitly shared run |

Add `endpoint-watch` to the existing terminal client. Require an exact run and
request pin, an explicit sharing flag and a time budget between one and 3,600
seconds. Use only the ordinary client credential. The service must independently
have enabled observation sharing; the watcher cannot enable it. This command
works without a controlling terminal because it supplies no approval.

Read the sealed request and poll the existing fixed event route from index zero.
Verify individual pages and the complete growing prefix with the original kernel
state machine. Require the start event's exact request digest, stable identities,
contiguous links and legal transitions. Refuse decreasing totals, excess event
budgets and a terminal event with advertised trailing events. These checks prove
self-consistency of service observations, not authenticity or task acceptance.

A fully framed HTTP 409 with the existing `api.busy` error may wait and poll the
same event index. This exception applies only to the ordinary-credential event
observation route. Other refusals, malformed responses and transport uncertainty
stop the watcher. Existing command, approval and effect callers retain their
no-retry behavior. No control credential, new HTTP route or model connection is
introduced.

Bound the watcher by its original monotonic deadline, 4,096 events, 40,096 page
requests and four MiB of actual escaped output. Wait 100 milliseconds after an
empty page or busy observation. Use the existing signal latch for interruption
and socket reads. Flush each rendered event; stdout backpressure and individual
blocking writes are not forcibly preempted. Emit only escaped observation
objects, with no implicit file retention. A valid terminal observation ends the
watch successfully even when the recorded run failed or was cancelled.

Stopping, timing out or refusing leaves the original run held. Advance, model
disclosure, effect decisions, cancellation, release, diff application, commit
and push remain separate controls. This adds no execution recovery, independent
checker, native admission or model measurement. T-303/T-306 remain open with no
new rows or sub-rows.

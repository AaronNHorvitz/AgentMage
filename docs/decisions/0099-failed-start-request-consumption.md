# Decision 0099: Failed Start Request Consumption

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-28 |
| Authority | Decisions 0061, 0081 and 0088; current owner restart |
| Scope | AMR-01 and Tasks 48.2.4.2–48.2.4.3 |

## Finding

Both native Chat and live coding services remove a prepared request only after
their factory composes a coordinator. The live service also waits for subscription
and worker setup. A failure before removal leaves that exact request available for
another composition attempt. Factories can perform startup work before failing.

The current development factory separately consumes its own prepared request and
refuses a second composition. No repeated real-model work has been observed.
Both services already allow explicit release of unstarted requests; this is not
a finding that retained entries cannot be released. The service boundary should
enforce its own single-use start contract for every trusted factory.

## Decision

In both services, consume the exact prepared request immediately after request
verification, equality and active-run checks, before entering the factory. Keep
malformed, substituted and unprepared requests from consuming an existing valid
request. Preserve the existing error for each service's missing request.

Never restore the request after composition, subscription, worker setup or initial
advance fails. Return the original startup error. Preserve owned coordinator and
worker cleanup, terminal release, run ceilings, native reconciliation, durable
state, model admission and all effect authority. Releasing an already consumed
failed request remains unavailable. The CLI's best-effort release must preserve
the original startup failure.

A fresh preparation remains a separate trusted operation. It cannot authorize
replay of an old effect, refund a consumed grant, clear uncertain cleanup or prove
that a prior model stopped. No new retry mechanism, coordinator, model owner or
wire contract is introduced. Broader cancellation during synchronous factory
composition remains a separate open prerequisite.

## Verification boundary

Retain a failing pre-fix run with recording, non-model factories. Verify one
composition entry per prepared request, preserved valid requests after malformed
and mismatched starts, capacity after failed starts, exact explicit release,
subscription failure cleanup and initial advance failure. Exercise the CLI's
failure path through the actual service adapter. Use the existing in-memory
request fixture; these tests do not qualify native tools or model execution.

Batch both service changes and their tests before renewing applicable evidence.
Keep any earlier failures and full historical bindings. Native coding, real-model,
manual-user, independent-review, human-only and release acceptance remain open.

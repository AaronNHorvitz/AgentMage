# Decision 0172: Failed validation observations in the endpoint coordinator

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decisions 0169 and 0171 |
| Scope | Explicit bounded failure feedback, without retry dispatch or success |

The default tool-failure path terminates. Add a trusted process-local opt-in for
an original endpoint command boundary to admit one completed failed validation
as a later model observation. Only the endpoint controlled-write family and a
command with known unchanged workspace state can use it. Native and durable
families, denied effects, cancelled or timed-out workers and uncertain results
keep their existing terminal handling.

The native command boundary must compare the complete execution against its
privately retained result, including elapsed time and the original authority
receipt. Reuse the existing parsed validation receipt, repair eligibility,
consumed-selection check and three-attempt ceiling. Assertion and compilation
failures can be eligible; compilation does not assert any executed tests. An
ordinary command failure, malformed or skipped-only result is not eligible.
Consume the original repair-feedback selection before admitting the failure
to coordinator history; loss, stop or exhaustion cannot restore a parallel
proposal selection. No additional test attempt is spent.
No replacement executor, grant issuer, manifest exception or receipt adoption
is introduced. The opt-in is not a model or HTTP field.

Retain the actual `Failed` result and original receipt in the existing
coordinator. Publish the original `ToolFailed` event, charge the bounded output
through the unchanged output route and pass the unchanged failed result to
later context and verification. Successful calls and eligible failed calls
share completed-call history; callers must inspect each result's outcome.
Never turn a failed check into successful completion evidence. Preserve one
cumulative ledger, clock, cancellation path and event sequence. Further effects
still require fresh exact approvals; no automatic replay or authority renewal
follows from an observation.

The generic hook refuses by default. A classifier error fails closed and latches
reconciliation after the observed effect. Default prepared admission remains
unchanged; trusted factories can explicitly select failure observations before
admission. This is an execution prerequisite for the coding repair loop, not its
scheduler: returning the original owner to editing, fresh source context,
repair prompts, subsequent tests, durable recovery, approved Git and workflow
measurements remain open. T-303/T-306 remain open pending independent acceptance.

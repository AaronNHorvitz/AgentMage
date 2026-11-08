# T-303/T-306: cooperative endpoint dispatch control

Date: 2026-11-07. Non-closing core work under Decisions 0153 and 0154.
Both parents remain open. Independent exact-commit review is required.

## Change

The existing prepared endpoint transport accepts borrowed `ModelOperationControl`
without inventing a native model profile. Controlled execution sends the same
single-use immutable body through its original credential-owning transport.
Every positive observation can narrow a local remaining-time allowance; even a
broken owner cannot renew an earlier observed allowance. Original total/idle
limits and the independent chat cancellation flag remain in force. Polling
covers pre-connect, socket waits, decoded fragments, callbacks and complete EOF.

A stop returns its closed `model.operation.*` diagnostic and no completed model
result, token usage or proposal. It does not set the chat cancellation flag,
acknowledge canonical runtime cancellation, retry or refund a repair attempt.
Callbacks remain synchronous and must return promptly; a stop cannot preempt
caller code. Their time counts against the deadline, and late completion fails.

Actual snapshot, workspace and repair proposal dispatch uses this method under
a local monotonic control, anchored before context publication rather than
restarted at model entry. Its allowance comes from the original prepared total
transport limit. Preparation and approvals remain outside that deadline. Positive
fractional milliseconds round up in the run-control observation while the exact
transport deadlines remain independent. Legacy chat and model listing keep their
existing interfaces. No credential, prompt, scope, grant or tool changes occur.

The preceding exchange-record [independent core review](../reviews/6bee4c9d55be-local-coding-endpoint-exchange.txt)
has **Verdict: PASS** for exact commit `6bee4c9d55be4120721ff29b5c0cea002953e380`
and no high/medium finding. It is retained byte-identically. Its low L1 (temporary
serialization reallocations) and L2 (API finish-string assertions), and the earlier
native failed-test receipt route fixture, remain backlog in these same parents.
That report does not accept this new controlled-dispatch batch.

## Verification

The corrected complete source gate selected three affected workspace crates:
`agentmage-host`, `agentmage-local-api` and `agentmage-platform-linux-inference`.
Its seven unmodified commands ran under one shared build reservation in
**111.16 seconds**, excluding the reservation wait. Formatting passed in 0.95
seconds and Clippy with warnings denied passed in 12.97 seconds. Complete
all-target tests took 95.79 seconds: **27 targets, 903 passed, 53 failed,
19 ignored, 0 filtered**. Cargo returned 101 and the source gate returned 1.
The source gate remains **FAIL**, without any allowlist, skip or filter change.

The comparison audit matches all 53 failure identities and normalized diagnostics
and all 19 ignores against the same complete predecessor targets. All failures
are in the unchanged host library. Those same targets previously had 897 passes;
this batch adds six passing methods. The wider predecessor selected ten crates;
this batch's unmodified reverse-dependency selection requires only these three.
The deferred executable-trust owner-action blocker stays open.

All six added methods pass unfiltered. Actual synthetic socket cases cover eight
initial/pre-connect owner-stop refusals; twelve stalled-header, truncated-body
and held-open-EOF cases across all four owner stop labels; independent original
transport expiry; a broken owner repeatedly renewing its reported time; a later
narrowed allowance followed by renewal; a callback returning after expiry; and a
successful frozen request. Stops supply no completed exchange and leave the
separate chat cancellation flag unset. The success case independently hashes
the raw transmitted body and returned UTF-8 text, checks full request equality,
and keeps missing provider usage and verified tokens/server memory null.

The API control unit method checks an already elapsed deadline and positive,
nonincreasing remaining time. Existing actual API success, incomplete-stream,
cancellation and four-case repair fixtures pass through the new dispatch path.
They do not constitute a route-level test of expiry during backpressured context
publication: that integration ordering was checked in the source. Existing
exchange tests also preserve missing/zero/nonzero usage and legacy callers.

The first complete gate attempt failed compilation in Clippy and tests: two new
test assertions used a hexadecimal formatter unsupported by the pinned digest
output type. Both were corrected to bytewise hexadecimal encoding; no case,
expected value, threshold or assertion was removed. Its complete failed log and
three-source hash freeze are retained alongside the corrected run. Only the
test source hash changes between those two freezes; production sources match.

Final light checks require the seven exact paths, byte-identical retained review,
unchanged Cargo inputs and source commands, all three corrected source hashes,
unchanged ledger table rows, added-content privacy and no new ignores. They run
no build or full suite. Logs, plans, freezes and comparison helpers remain in
private lane state.

## Limits

Synthetic socket peers exercise transport execution, not a real model or native
coding workflow. Native configured-endpoint coordinator admission, its request
and context envelopes and resource ledger, fail-and-repair execution, durable
ownership, approved commit/push and parent measurements remain open. No ledger
row, hidden or held-out input, threshold, scorer, ignore, dependency, SBOM,
implementation-status, model download, push or public release changes here.

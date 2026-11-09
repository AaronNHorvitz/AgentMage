# T-303/T-306: strict responses to live native previews, 2026-11-09

Status: builder verification complete; complete scoped gate FAIL on inherited
failures; independent core review required. T-303 and T-306 remain open. No row
or sub-row is added or closed.

## Change

A trusted prepared endpoint turn can retain a single-use tool-preparation
callback. The original coordinator calls it only after a normally completed
model response has been accounted, under the original borrowed control and
after tool/event capacity checks. The callback receives original derived call
identities and returns only an inert tool proposal. Ordinary turns retain their
tool-or-completion behavior. Partial, overflowing, token-limited and stopped
responses cannot enter the callback. Refusal cannot fall back to a completion
claim or model-authored tool envelope.

The host's turn and tool sides privately reference one original created
repository. The callback checks the complete original request and controls,
re-reads consented source snapshots, checks strict edits and complete preimages,
then prepares the original complete diff. The existing native boundary alone
asks for an exact approval, issues and consumes the original grant and runs the
atomic writer. Only an observed successful native edit returns the same owner
to preview preparation; pending, failed, uncertain and spent effects remain held.
Lock poisoning refuses. Original lineage, clock, control and cumulative ledger
remain with the same run across later strict turns and approvals.

Read/disclosure consent, current source selection, the correct original owner,
shared epoch-clock semantics and independent completion checks remain trusted
caller responsibilities. The content-free workspace label cannot distinguish
created owners. No owner is restored from serialized metadata or a path.
See [Decision 0171](../decisions/0171-strict-response-native-preview-in-one-coordinator.md).

## Verification

Both added regression methods pass in focused development runs: the kernel
method covers nine cases (1 passed, 1,563 filtered, 0.03 seconds); the host method
covers thirteen cases in each connection ownership form (1 passed, 688 filtered,
5.00 seconds). Both pass again in the complete unfiltered gate below.

The kernel checks the original derived identities and full accounted answer,
then exercises exact approval and ordinary completion. Refused interpretation,
unoffered tools, invalid arguments and original deadline expiry execute no tool.
Failed, limited and overflowing responses and exhausted tool capacity never
enter the callback. Each resulting event sequence verifies.

The host actually executes two strict generations, complete native previews and
separately approved native writes in one coordinator. It asserts the exact frozen
user messages on the wire, original bytes before each approval, inert tampered
and earlier approvals, distinct new approval identities and both resulting files.
Other cases cover cumulative model-call exhaustion, malformed, overflowing,
incomplete, stale, protected and outside-path responses, explicit denial, exact
cancellation, actual challenge expiry and a model-authored tool envelope. A later
context including an already edited file refuses on its stale complete preimage.
Cumulative input equals actual frozen bodies plus native arguments; completed
output equals actual model text plus native results. Incomplete output counts
remain absent. Exact terminal states, actual tool counts, verified event sequences
and inert terminal rereads are asserted. No fixture certifies task completion.

The predecessor review's low exact-prompt assertion is strengthened in its
existing coordinator fixture. The earlier text-ceiling fixture now checks the
exact delivered prefix for every overflow case, using its fixed SSE boundaries.

Development logs retain the initial callback return-type compile error, host
closure inference and slice-type compile errors, and fixture assertion failures
for an absent completed-output counter, a denial carrying a grant ID and an
assumed approval expiry. Fixtures now use the existing API's exact semantics;
production denial, expiry, cancellation and failure behavior is unchanged. The
first documentation check could not start because the base PATH lacked the
already retained Node executable; rerunning with that existing executable passes.
No dependency is downloaded or installed.

The first complete scoped run retains 45 unfiltered targets: 2,796 passed,
exactly the 72 inherited host/Linux failures with none new or missing, and 71
existing ignored. It also fails Clippy on generic-bound placement and two test
iterator expressions. Those style issues are corrected without lint exceptions
or behavioral changes, after that run finishes with its nine Rust hashes
unchanged. Its full failed log is retained. The corrected complete gate is
reported below; no passing gate is claimed from the inherited-failure comparison.
An intervening corrected run is explicitly interrupted during lint after it was
started before the separate formatter had returned. That run reports no pass;
its source hashes happen to remain unchanged. The formatter then finishes zero
before the fresh final run starts. Every interrupted/failed log is retained.

The corrected complete scoped fast gate remains **FAIL / exit 1**: 45 targets,
2,796 passed, exactly 72 inherited host/Linux failures with none new or missing,
71 existing ignored and zero measured or filtered. Formatting exits zero in
2.11 seconds; all-target Clippy zero in 12.28 seconds; all-target tests 101 in
303.80 seconds. Total elapsed time including reservations is 319.70 seconds.
All nine Rust input hashes remain unchanged throughout. The comparison of
inherited failure identities is data only, never acceptance or a passing gate.

```sh
python3 -B scripts/local_agent_fast_gate.py --base f4aa4b43f39821c47c1ee300e8999a3b23d467b0 --timeout 600
```

Each Cargo command reserves its own shared build slot, with two build jobs and
four test threads. Host, kernel, local API and Linux platform formatting,
all-target lint and all-target tests run without filters. The unchanged inference
package is not selected. Only documentation results are updated after the final
source gate; final whitespace, Markdown, review-text and public-hygiene checks
run separately. Task-row lines remain identical to the immutable base.

Final full-gate raw log SHA-256:
`a034dd148efb07f2e5d6b94256f09ecc1607a7ce58aeb9de262112390f3b85ca`.

## Scope

Creation provenance and endpoint answers in regression fixtures are synthetic;
native strict parsing, transport, coordinator and file transactions actually
execute in the passing regression methods. These fixtures cannot establish real-model coding,
native Git creation, test-command success, independently graded tasks or a score.
Automatic service factory selection, general edit/test/retry, durable recovery,
approved Git and parent measurements remain open. Hidden tests, scorers, task
pins, held-out data, thresholds, dependency/SBOM outputs, the status model and
scoreboard are unchanged. The long acceptance gate and GPU inference do not run
in this batch. The in-flight predecessor commit and its history stay immutable.

## Predecessor review

The [independent coordinator-generation review](../reviews/f4aa4b43f398-local-coding-coordinator-generation.txt)
arrived during implementation. It has **Verdict: PASS** for exact commit
`f4aa4b43f39821c47c1ee300e8999a3b23d467b0`, with no high/medium finding.
Its bytes are retained unchanged. Its low L1 exact-prompt assertion is addressed
in this batch; the preceding overflow-prefix low is also strengthened. Earlier
schema-ID and command-resource lows remain same-row backlog. This predecessor
verdict does not accept the new preview connection or close either parent.

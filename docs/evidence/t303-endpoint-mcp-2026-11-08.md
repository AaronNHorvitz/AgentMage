# T-303/T-306: explicitly delegated endpoint observations through MCP

Date: 2026-11-08. Tier: core. Immutable predecessor:
`c73c5f9f8b4255286773d216df31d90bbf309279`.
The predecessor's [independent core review](../reviews/c73c5f9f8b42-local-coding-endpoint-terminal.txt)
arrived during this batch with **Verdict: PASS**, no high/medium finding. It is
retained byte-identically. Its low L1 (post-cancellation finalization without
disclosure consent) and L2 (exact readable numeric argument text) remain backlog
in these same rows, alongside the prior HTTP regression-coverage low. This
increment adds no row or sub-row and closes neither parent. Its own independent
core review is required.

## Change

The original endpoint service remains control-only. A new explicit trusted
startup policy can delegate only request inspection and bound canonical event
paging to the ordinary client credential. Delegation shares potentially
sensitive task and tool observations. It cannot advance the coordinator,
consent to model disclosure, decide an approval, cancel or release the run.
The same retained owner, HTTP workers, authentication, parser and scoped event
projection remain authoritative. No wire argument can change the policy.

The existing MCP executable can share one exact run after an explicit
`--share-selected-endpoint` launch. This selection offers only
`endpoint_request` and `endpoint_events`, and reads only `client.token`.
Every call rechecks the original sealed run identity and request digest.
Request and event checks are shared with the trusted terminal implementation;
terminal operations continue to use their original control credential.
Event validation checks individual hashes and page identities, without claiming
a complete stream, accepted completion proof or durable recovery cursor.
The existing MCP lifecycle and fixed bounded HTTP client are reused. There is
no second loop, retry or model/tool authority. The workspace MCP selection
retains its original six tools and permissions.

## Verification scope

Three new integration methods exercise the actual MCP binary and retained HTTP
owner. Ports, permissions and completion verifier remain explicitly synthetic;
no native filesystem/grant execution, local inference or coding score is
claimed. The tests use a private delegate directory with a valid client token
and an invalid control-token symlink: loading the control credential would
refuse. They cover default refusal and explicit observation delegation, exact
run/digest pinning, a replacement service's fresh identity, the two-tool
catalog, refused control and workspace tools, extra arguments, index bounds,
missing launch consent, canonical cancellation-event export and release.
Tool executions remain zero; nonblocking upstream listeners observe no model
connections. Existing terminal tests cover the shared request/event checks.

The first focused run passed both executable MCP methods and failed the new
HTTP authorization fixture because it sent a JSON body with GET. The existing
bounded parser correctly returned 400 before route authorization. The new
fixture was corrected to send a bodyless GET when testing the exact 403 route
refusal. No product boundary, old assertion or threshold was weakened. Initial
and repaired development logs are retained privately. The first complete gate
passed all 138 tests but strict Clippy refused a nonminimal boolean expression
in the new delegation check. The condition was simplified without changing its
truth table. That failed gate and its original source hashes remain retained;
the complete final gate is rerun against the repaired, frozen source batch.
Final unfiltered verification is recorded below.

## Limits

A trusted host must explicitly choose observation delegation. No executable
activates the retained endpoint service yet. Actual native filesystem/grant
composition, edit/test/retry, durable recovery, approved Git and both parents'
measurements remain open. The bridge cannot govern a calling program's
subsequent handling of received text. Explicit launch consent shares potentially
sensitive run observations; it does not authorize a model call.

No evaluation task, hidden or held-out test, scorer, threshold, ignore,
dependency, native admission, model, GPU, scoreboard, SBOM, status or release
change. Existing evidence stays bound to historical inputs; no supply-chain
regeneration occurs during this batch. No independent self-approval or closure.

## Final unfiltered verification

The complete ordinary local-API fast gate passes: **nine targets, 138 passed,
zero failed, zero ignored, zero filtered**. All 135 predecessor passes remain;
exactly the three new integration methods pass. Formatting and strict Clippy
pass. All seven changed Rust files remained hash-identical throughout the final
gate. The seven commands took 25.82 seconds including shared reservations
(0.10, 0.10, 0.10, 1.15, 0.20, 1.81, 22.36).

The baseline uses all nine targets from the complete predecessor API gate,
without selecting outcomes. This is verification of the API increment only;
pre-existing broader native T-301.1 gate failures are untouched and are not
claimed revalidated. The failed initial gate remains separate. No source
failure is waived. The retained predecessor report arrived after the initial
source plan was captured; its text lint is included in the successful final
gate. Final documentation and hygiene checks run after this evidence update,
without repeating Cargo or changing source.

The private validation retains both complete gate logs and input snapshots,
initial and repaired focused logs, the complete predecessor baseline, final
command plan, all source hashes, exact test-identity comparison and review
helpers. Final light checks cover the exact twelve-file change scope, unchanged
ledger tables, retained review identity, newly authored public content and
committed Git blobs. Independent core review of this exact increment remains
required; neither parent closes.

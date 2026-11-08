# T-303/T-306: endpoint binding to the kernel coding policy

Date: 2026-11-08. Tier: core. Immutable predecessor:
`974505fb1f2d2b8bbebdfefa5348239cb603cc74`.
Its [independent core review](../reviews/974505fb1f2d-local-coding-endpoint-mcp.txt)
arrived during this session with **Verdict: PASS**, no high/medium finding.
It is retained unchanged. Low findings remain backlog in these same rows;
the next private request uses an installed-tool lookup in Python. No row or sub-row
is added or closed. This increment requires independent core review.

## Change

The existing native policy builder now delegates its envelope construction to
a private shared helper after all original profile, task, workspace and guidance
checks. Its approval, durable grant and execution implementations are unchanged.
The endpoint entry checks an original sealed request against a held workspace
and complete exact tool catalog, then uses that same helper. It returns the
matching policy and a resealed request. Only the policy id, policy digest and
whole-request seal change. It refuses non-read tools, persistent/write modes,
event cursors, substituted catalogs and tampered or foreign workspace requests.
Actor/task identities, bounded coordinator action identities, exact platform
authorization scopes and every canonical exclusion enter the original policy.
Network, credential and publication scopes remain denied.

This removes the model-profile prerequisite from policy construction only.
It creates no source consent, destination approval, grant, coordinator or file
access. No native request/profile is synthesized. Existing native admission,
authority, approval and consumption boundaries remain unchanged.

## Component verification

Five new methods use directly framed endpoint requests and real Linux file
descriptors through the existing test-only workspace selector. They check exact
framing preservation, exclusion canonicalization and idempotent rebinding;
tampered seals, foreign workspaces, replaced catalog definitions/digests,
empty catalogs, persistent/write modes and event cursors; exact-digest non-read
tool refusal; protected approval rendering for real held file targets, refusal
of excluded objects, exact response checks, bounded actions and single-use
kernel grant consumption. Policy probes cover actor, session, task, tool,
arguments, expiration, path, preimage and external scopes. A separate failed
consumption case checks terminal invalidation and refusal of later valid retry.

The grant cycle uses the kernel's in-memory issuer; it is not durable runtime
authority or an actual tool dispatch. No model, command worker or HTTP service
runs in these new tests. Files are temporary synthetic fixtures. Native platform
admission is not asserted. This is component evidence, not a coding score.

Initial development compilation found two new fixture mistakes: a private
platform selector and a missing cursor event id. They were corrected to use the
existing test-only selector and complete cursor. The next run passed two methods
and failed two new fixtures: controlled-write framing lacked its existing
required budgets, and a grant had correctly become terminal after attempted
invalid consumption. The tests now supply complete controlled-write budgets
and check successful consumption and failed-consumption latching in separate
cases. A further focused run passed four methods and caught the new fixture
trying to reseal an inconsistent catalog digest. Both that digest case and the
ephemeral-cursor case now assert the original sealer's refusal before checking
the binder's refusal. Product checks and original assertions were unchanged.
Raw logs remain private. Final unfiltered verification is recorded below.

## Remaining work

The endpoint HTTP factory still needs actual native tool and grant-owner ports
and executable activation. This policy helper does not enable its native-profile
Linux boundary. Edit/test/retry, durable recovery, approved Git, parent model
measurements and both T-303/T-306 remain open. No evaluation, hidden or held-out
input, threshold, scorer, model, GPU, dependency, status, scoreboard or release
changes. Source changes are batched; no supply-chain regeneration occurs here.

## Final verification

The complete ordinary host/API fast gate finished all seven commands over
**19 targets: 739 passed, 53 unchanged failures, 11 unchanged ignored, zero
filtered**. All 734 predecessor passing identities remain, with exactly the
five additions. Every failed/ignored identity and every normalized failure
diagnostic matches the retained baseline. Formatting and strict Clippy pass.
The source gate remains **FAIL** on the existing deferred T-301.1 failures;
none is waived. This gate covers the affected host/API packages, without a
claim that the other packages' historical failures were revalidated.

All three changed Rust inputs stayed hash-identical throughout the final gate.
The seven commands took 136.56 seconds including shared reservations
(0.10, 0.15, 0.10, 1.30, 0.90, 9.22, 124.79). The original native authority
test module remains byte-identical. A source comparison also verifies that the
shared policy envelope matches its original body, apart from extracting the
profile-exclusion accessor into the native entry point.

Baseline selection uses all ten host-package targets from the complete prior
HTTP gate and all nine API targets from the complete predecessor MCP gate.
Host sources are unchanged between that HTTP commit and this predecessor.
No test or target is selected by outcome. Raw logs, baseline records, complete
source copies and hashes remain private. Current documentation, report and
hygiene checks run after this measured record is added, without repeating Rust
verification. The independent report is retained unchanged, and every ledger
table row remains unchanged. No independent self-approval or parent closure.

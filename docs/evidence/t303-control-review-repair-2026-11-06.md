# Coding cancellation and repair follow-up

Date: 2026-11-06. Scope: one non-closing T-303/T-306 batch under Decisions
0153 and 0154. This is builder evidence, not an independent verdict or a model
score. No row or sub-row is added. Both parent rows remain open.

## Changes

The [exact prior independent review](../reviews/d0bbe2546b94-local-coding-command-cancellation.txt)
accepts the cancellation batch at commit
`d0bbe2546b9408ad6025272bf5105e46e4a04c54`. Its original bytes are retained.
This follow-up addresses its low findings L1 through L3 and the earlier
[repair review](../reviews/33cd39e21481-local-coding-model-repair.txt)'s low L1.
It makes no independent finding-closure claim.

The existing command cancellation handle now supplies the passive observer to
`CommandEffectDriver`. It attempts at most eight nonblocking reads, yielding
between unavailable reads. Persistent unavailability still fails closed;
invalid signals and scope mismatches return immediately. The kernel still
checks scope against the consumed permit. No timeout, cleanup, command,
permission or authority rule is relaxed. This reduces brief signal/poll mutex
collisions; it cannot guarantee availability or successful native termination.

Repository operations and repair proposals atomically acquire a lease that
requires the control token for cancellation. A client-token request returns
403 and records denial without signalling. The terminal cancel command sends
the control token without adding an approval prompt. The control token and
orderly service shutdown retain cancellation. Ordinary chat/source operations
retain client cancellation. The lease persists through cleanup; non-interruptible
steps still do not acknowledge cancellation. No receipt or approval is restored.

Repair preflight now uses the same complete message constructor as dispatch,
including additional system instructions and serialized diagnostic data. It
encodes all three messages before consuming the one-request repair consent.
The model, token and encoded-byte checks apply to that complete request.
Ordinary proposals retain their original two messages and source selection.
The guide now distinguishes connected command cancellation from still-open
cancellation of worktree creation and write effects.

## Verification

Initial focused checks passed: 31 repository-owner cases and all then-current
API targets. The first complete source run had 834 passes and exactly the 72
retained baseline failures, 62 existing ignored tests and zero filtered tests.
Consumer tracing then found that terminal cancellation still selected the client
token. The same-batch repair switches it to control authentication and adds an
external terminal request test before final verification. The initial log and
failure-set comparison are retained. No original assertion or threshold changed.

Final verification after the terminal repair and source freeze:

| Check | Observed result |
| --- | --- |
| Native repository-owner cases in complete source run | All 31 pass |
| Complete API targets | All 67 pass: 11 library, 10 terminal, 11 worktree and 35 socket/API cases |
| Selected formatting | PASS; 6.41 seconds including reservation wait |
| Selected all-target Clippy, warnings denied | PASS; 0.80 seconds including reservation wait |
| Complete selected source targets | FAIL; 835 passed, exactly 72 baseline failures, 62 existing ignored, zero filtered; 91.19 seconds including reservation wait |

All 53 host and 19 Linux failure identities equal the unchanged baseline set,
with no additions or omissions. The complete source gate remains **FAIL**.
Shared reservations were honored; unavailable capacity was queued without
stopping other workers or raising limits. No source edit follows this run.

The added checks exercise a fixed bound on passive observations, persistent
unavailability, immediate signal/scope errors, refusal to cancel a controlled
lease with a client token, retained leases, ordinary-client cancellation,
non-interruptibility and shutdown. Real authenticated HTTP requests check 403
for the client and 200 for control. A separate actual terminal executable test
inspects its request at a synthetic peer, without injecting native ownership.
An external request probe reproduced the earlier client-token selection and
then observed the rebuilt terminal selecting control, with no approval prompt.
Its final terminal binary digest is `2784171f5888393042746843fd69ba9dd8e011f265f0a23c1a2500a2e132f449`.
These are separate transport/owner checks, not native command termination.

The shared repair-message constructor is checked against the encoded system,
source and diagnostic messages, including escaped controls and Unicode. A
synthetic diagnostic larger than the production cap proves that the complete
request can refuse where an ordinary two-message request fits. Production
source, diagnostic, wire and token ceilings are unchanged. Existing socket
repair proposals and strict edit checks also pass.

All 30 ordinary development contexts match both historical model campaigns
through the final compiled API, in 2.86 seconds. This uses the
unchanged synthetic-upstream driver, with no inference, hidden tests, grader,
held-out inputs or new score. The final API binary digest is
`f2b16ed6df9d3121b3858b297f46d8a0ab516f7c992ba9f3787351551bbcbbb6`; the driver digest is
`8e2b1b5443c8521d03979ff80e969c335fcf611f71230a4ac7acef9010281366`.

Final Markdown, policy and hygiene checks cover the complete changed file set.
Raw initial/final source logs, terminal probes, development-context records,
source freezes and audits are retained privately under
`tmp/t303-control-review-repair/`. The audit checks exact prior-review bytes,
source hashes, baseline failure identities, untouched evaluation/schema/fixture/
dependency/SBOM inputs, unchanged ledger statuses, local links and public hygiene.

## Remaining work

Positive native execution remains refused by the unchanged executable-trust
boundary in this lane. This batch does not establish native HTTP command
termination, coordinator composition with the configured model, automatic
failure-to-repair execution, durable coding runs, original-repository apply,
approved commit/push, MCP or parent acceptance. Shared error prefixes,
refused-edit latest-result retention, the same-user window and unsupported
runner/compile-failure cases remain backlog before parent acceptance.

No frozen task, hidden test, grader, threshold, baseline, model profile, schema,
producer fixture, dependency, SBOM, product status or checkbox changes. No
inference, held-out evaluation, GPU work, public release or owner-ready claim.

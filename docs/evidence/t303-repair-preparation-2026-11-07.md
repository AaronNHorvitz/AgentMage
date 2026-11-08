# T-303/T-306: prepare repair requests before spending disclosure consent

Date: 2026-11-07. Core follow-up under Decisions 0153 and 0154. Both parents
remain open; independent exact-commit review is required. No new ledger row.

## Change

Repair preflight now returns the same immutable, credential-bound request that
dispatch executes. The prepared proposal owns its source snapshots, checked
context, feedback and exact combined message-byte count. Its transport request
borrows the original configuration; it cannot be cloned or rebuilt from its
inspection binding. The repository owner stays locked through preparation,
receipt consumption and generation, with the existing operation lease and
independent status/cancellation access.

The exact endpoint approval and complete JSON encoding are checked before the
single-use failed-test receipt is consumed. A preparation refusal or cancellation
observed before consumption preserves that attempt without opening a connection.
A receipt refusal drops the prepared request. After consumption, backend errors,
late cancellation, invalid edits and lost replies still spend the attempt. There
is no refund, retry, new test allowance or additional effect authority.

Snapshot and selected-workspace proposals use the same prepared dispatch path.
Their successful request bytes, strict edit checks, completion/accounting fields
and source/disclosure rules remain unchanged. Two API fixtures now independently
hash the raw upstream body and compare its SHA-256 with both events' bindings.
The synthetic repair fixture covers valid and invalid edits, with usage enabled
and disabled, including escaped diagnostic text. The test-only hash uses the
existing pinned SHA-256 dependency; no new package or version is introduced.

This addresses low L1 and L2 from the retained endpoint-binding
[independent review](../reviews/0e5ed222cc36-local-coding-endpoint-binding.txt)
in their existing rows. That PASS accepts its original commit only.

The preceding runtime model-binding [independent review](../reviews/e5cb5c53a4ad-local-coding-runtime-model-binding.txt)
arrived during verification and is retained byte-identically. It has
**Verdict: PASS** for exact commit `e5cb5c53a4ad588ba55784bd24ec73c3259dfff3`
and no open high/medium finding; both parents remain open. Its low L1, native
accounting wrapper test strength, remains backlog in these rows. Its low L2 is
addressed by clarifying that `response_complete` controls completed accounting;
the older `partial` error flag can also describe a rejected completed inference.
That review does not accept this follow-up.

## Verification

The complete source gate selected all eleven workspace crates because the
test-only dependency changes Cargo.lock. Six commands ran under one shared
build reservation in **279.63 seconds**, excluding reservation wait. Formatting
passed in 2.50 seconds and Clippy with warnings denied passed in 1.60 seconds.
Complete all-target tests ran in 274.18 seconds: **77 targets, 3,233 passed,
72 failed, 79 ignored, 0 filtered**. The gate remains **FAIL**: Cargo tests
return 101 and the gate returns 1. No failure allowlist, filter, new skip or
threshold change is used.

Both new regression methods and both strengthened fixtures pass unfiltered.
Preparation refusals, cancellation before consumption and receipt refusal open
no socket. A synthetic consumption callback stays unused on preparation or
cancellation refusals. Backend status failure observes exactly one consumption
before receiving a request; cancellation after consumption still spends one
attempt while opening no socket. Both dispatch cases produce incomplete error
events without completed accounting or a candidate, and make no second
connection. The callback establishes order only, not native receipt admission.

All 72 failure identities and normalized diagnostics match the retained complete
predecessor source gate: 53 host and 19 Linux-platform failures. All 79 ignore
identities also match. The extra workspace crate contributes four passing tests
and one target; the two new repair methods account for the remaining increase
from the predecessor's 3,227 passing tests. The original native executable-trust
blocker remains open; no gate result is converted into a pass.

The predecessor review arrived after source checks began. Final light checks
therefore add its unchanged report-format check and include the clarified guide
in Markdown lint. The reviewer gate has seven commands to the builder's six;
Cargo selection and all five source/manifest/lockfile hashes stay identical.
Final checks verify the exact ten-path scope, byte-identical retained review,
unchanged ledger rows, whitespace and added-content privacy. The parsed lockfile
diff is only the existing SHA-256 dependency on the API package; all package
versions and other dependency lists remain identical. These final checks run
no build or full suite. Raw logs, source freeze, plans and comparison helpers
remain in private lane state.

## Limits

The new receipt-order tests use an observation callback, not native creation
authority or a fabricated failed-test receipt. They exercise the production
preparation/admission/dispatch functions with real loopback sockets and synthetic
peers. The native repair route still requires its original ownership, scope,
settings, failure-receipt and disclosure checks. No positive native workflow,
real model execution, GPU work, new score or parent closure is claimed.

Endpoint-specific coordinator request/context/result contracts and execution,
native fail-and-repair measurements, durable ownership and approved commit/push
remain open. No hidden tests, frozen suite inputs, scorer, thresholds, grants,
sandbox, dependency versions, SBOM or implementation status is changed.

# Original command cancellation across endpoint handoff

Scope: T-303/T-306, core, non-closing. Both parent rows remain open.
Base: `5d4250ae61241565d3567b391f3dd7b054ed107f`.

## Change

Each original pending command now owns its attempt's cancellation token.
Handle lookups clone that token rather than create disconnected roots. The
existing direct execution and endpoint supervisor bridge already obtain their
observers from the pending attempt, so they now retain the same signal across
handoff without a new cancellation mechanism or execution path.

Validation refines the preview digest before exposing any handle while keeping
the same attempt token. Each fresh preparation, including a validation retry,
creates a separate token. Exact preview/attempt checks still reject foreign
handles. The first accepted cancellation reason remains authoritative.
Cancellation grants no execution authority and restores no spent decision.
Already issued durable grants remain recorded until ordinary expiry.

The original continuous run control and scoped cancellation probe still apply;
their unavailable or foreign observations still fail closed. Native admission,
policy, exact grants, supervisor confinement and cleanup remain authoritative.
Acknowledgement is a request, never proof of process termination.

## Verification scope

Five added component methods exercise both command and validation paths:

- Repeated public lookups share the first shutdown reason; default direct
  execution with no explicit handle observes cancellation without executor entry.
- A handle issued before handoff cancels after exact grant issuance, before
  endpoint native preflight. Dispatch retires, no command receipt is fabricated,
  no validation attempt is spent, and the decision cannot be reused.
- A separate thread signals the original handle after synthetic executor entry.
  Original supervisor observations receive the exact attempt identity and reason;
  real authority/command receipts report cancellation. Validation never passes.
- Abandoned handles, both before and after grant issuance, cannot cancel a fresh
  command or validation preparation; explicit stale attachment still refuses.
- A failed validation's old handle cannot cancel its fresh, separately approved
  retry. The original parser and receipt binding retain the actual second attempt.

Creation provenance and executor observations are synthetic. Held roots, token
propagation, native command driver, durable authority transactions and validation
parsers are real. No child-process stop, native Git creation, real-model coding,
independent task grade or coding score is established by these tests.

## Validation

The focused development run passed all 21 endpoint command methods, including
the five added regressions (8.83 seconds for the library tests). It filtered
unrelated tests and is not the complete gate. No development failure was hidden
or changed into a passing claim.

The unchanged complete fast gate ran over the Linux platform, host and local
API with two build jobs and four test threads. Each Cargo invocation used its
own shared build reservation. No test execution filter, new ignore, allowlist,
validator change or failure waiver was used.

| Check | Actual result |
| --- | --- |
| Whitespace, untracked-file lint and Markdown lint | PASS |
| Scoped formatting | PASS, 1.15 seconds |
| Scoped all-target Clippy with warnings denied | PASS, 8.87 seconds |
| Scoped unfiltered all-target tests | FAIL, exit 101, 197.74 seconds |
| Complete fast gate | FAIL, exit 1, 209.19 seconds including reservations |

Across 25 targets: 1,147 passed, 72 failed, 63 inherited ignored, zero measured
and zero filtered. The failure identities exactly match the recorded T-301
baseline: 53 host and 19 Linux, no new or missing identities. All five added
methods pass in that unfiltered run. A separate data audit checks those identities,
unchanged source hashes, scope, ledger and privacy; its success does not make
the gate passing. Both Rust source hashes stayed unchanged throughout execution.

Raw commands, logs, exits, the source freeze and data audit are retained in the
private review bundle. The complete `fast.log` SHA-256 is
`50b2e99c7f039b8636bb6ea2cabf5b636af7369110d0a1942f14ab0e81188b6c`.
Its owned temporary namespace retains four tiny inherited host-fixture files
(36 bytes); new command fixtures are absent. No broad cleanup was performed.
No model/GPU process, download, supply-chain regeneration or score run occurred.

## Remaining work

Executable controlled startup, general model-driven edit/test/retry, durable
recovery, approved Git and parent measurements remain open. Conservative
output/CPU/disk preflight coverage remains same-row backlog. This repairs the
pre-handoff cancellation limitation recorded as low L1 in the command adapter's
independent review; it does not close a parent row. No new row or sub-row is added.
Independent core review is required.

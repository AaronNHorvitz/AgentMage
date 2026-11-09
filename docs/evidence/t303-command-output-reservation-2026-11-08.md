# Command output reservation — T-303 / T-306

Date: 2026-11-08. Tier: core. Both parent rows remain open. No row or sub-row
is added or closed. Independent core review is required.

## Change

Original endpoint command construction now refuses a sealed per-result output
limit below `4 * (stdout_bytes + stderr_bytes) + 16384`. Captured channels are
serialized as decimal-byte JSON arrays; four bytes per captured byte covers the
array's digits and separators, with a fixed 16 KiB metadata allowance. Arithmetic
is checked. This is an additional minimum admission reservation, not a proof
that all possible normalized validation metadata fits the result limit.

Refusal returns the same live repository with its exact pending preview,
cancellation signal, authority receipts and validation state unchanged. There is
no grant, command dispatch or attempt consumption. A separately admitted wider
request still needs its original protected approval. Original native manifest,
worker confinement, authority transactions, parsers and retry ceilings remain.

The post-dispatch complete projection check still refuses oversize results as
uncertain and retains the actual result without replay. Arbitrarily large parsed
validation metadata and exhaustion of the coordinator's aggregate output budget
remain possible. CPU and scratch/disk preflight remain backlog; this change makes
no claim about them. It addresses the output portion of the command-adapter
review's low L2 in the same parent rows.

## Verification

Three added regression methods cover both commands and validations: refusal
one byte below the reservation without authority/attempt changes; fresh admission
at the boundary with the same original cancellation signal; actual projection
of both maximum retained channels containing three-digit bytes, without byte
loss or replay; malformed validation output cannot pass; arithmetic overflow
refuses. Creation and command observations are synthetic. Durable authority
transactions and original normalization/projection are real. No native child,
model run, independent task grade or coding score is claimed.

The focused development run passed all 24 endpoint command methods, including
the three added methods (10.59 seconds for the library tests, 349 unrelated tests
filtered). That run is not the complete gate. No development failure occurred.

The unchanged complete scoped fast gate ran without a test filter, new ignore,
allowlist or failure waiver. Each Cargo command acquired its own shared build
reservation, with two build jobs and four test threads.

| Check | Actual result |
| --- | --- |
| Whitespace, new-file lint and Markdown lint | PASS |
| Scoped formatting | PASS, 92.52 seconds |
| Scoped all-target Clippy with warnings denied | PASS, 20.88 seconds |
| Scoped unfiltered all-target tests | FAIL, exit 101, 195.01 seconds |
| Complete fast gate | FAIL, exit 1, 310.26 seconds including reservations |

Across 25 targets: 1,151 passed, exactly 72 inherited failures, 63 existing
ignored, zero measured and zero filtered. The failure identities exactly match
the recorded T-301 baseline (53 host, 19 Linux), with no new or missing identity.
All three added regression methods pass in that unfiltered run. A data audit
checks scope, exact failure identities, unchanged source, ledger and privacy;
its success does not make the gate passing. Both Rust hashes stayed unchanged.

Raw logs, every command/exit and the source freeze are retained in the private
review bundle. Complete fast-log SHA-256:
`d599826e2e9a25cc0625a63481bd0fe6132956ed2338be9038e073f6421ab7ed`.
The owned temporary namespace retains four tiny inherited host-fixture files
(36 bytes, 22 entries total); new command fixtures are absent. No broad cleanup,
model/GPU run, download, supply-chain regeneration or score run occurred.
Documentation-only final checks run again after recording these observations.

## Remaining work

Executable controlled startup, positive native workflow admission, general
model-driven edit/test/retry, durable recovery, approved Git and parent
measurements remain open. This is a non-closing safety increment. No hidden or
held-out task, scorer, threshold, dependency, SBOM, status or score changes.

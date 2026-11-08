# T-303/T-306: coordinator tool and verifier request scope

Non-closing core implementation under Decisions 0153 and 0154. Both parents
remain open; no new row or closure. Independent exact-commit core review is
required. The predecessor remains immutable. Its
[independent core review](../reviews/164fc97773d8-local-coding-endpoint-events.txt)
has **Verdict: PASS** for exact commit
`164fc97773d8ab82820dd04af504302950ad23f5`, with no high/medium finding.
Low L1 (record preflight/refusal closure in full coordinator composition) and
L2 (digest-shape documentation, failure-turn binding and completion clocks)
remain backlog with earlier lows. This accepts the predecessor increment only.

The existing coordinator now uses model-independent scoped entry points for
its non-transactional tool evaluation, approval resolution, execution and
completion verification. A borrowed scope verifies the original complete native
or endpoint request and retains its distinct identity. It cannot fabricate a
native profile or tokenizer observation. Existing native-only ports refuse an
endpoint scope before entry and otherwise forward the original native request
and arguments. The transactional correctness/persistence paths remain native.

The existing coding completion verifier explicitly supports the scope while
retaining its deterministic snapshot, receipt, ordered validation and repository
inspection checks. It creates an inert verifier candidate; proposal admission,
proof validation, permissions and tool confinement remain with their existing
owners. This does not authorize or execute endpoint tools.

Four added kernel methods cover exact common observations and original request
identity, stale/resealed-invalid framing, native-only port refusal before entry,
and the actual existing coordinator's scoped approval/execution/verification
path. An altered approval is refused before resolution or effect. Two added
host methods compare successful changed/no-op native and endpoint verifier
candidates and reject missing receipts, stale evidence, wrong snapshots and
missing validation. Existing test methods and assertions are preserved.

The complete scoped gate ran all six unmodified planned commands under one
shared reservation in **360.12 seconds**, excluding queue wait. Formatting
passed in 2.10 seconds, Clippy with warnings denied in 30.31 seconds and tests
took 326.36 seconds: **43 targets, 2,558 passed, 72 failed, 71 ignored, zero
filtered**. All six new methods pass. Every predecessor passing identity is
preserved with exactly those six additions. Failure identities, normalized
diagnostics and ignored identities match the predecessor. The failures remain
53 host and 19 Linux library cases under deferred T-301.1. Cargo tests return
101 and the source gate remains **FAIL**; a matching baseline is not a green gate.

All five Rust input hashes remained fixed throughout the gate. The final light
checks cover the final documentation and newly retained predecessor review,
original-test-body preservation, unchanged ledger rows and Cargo commands,
source-copy bindings and public hygiene. Raw logs, immutable tested source copies
and reproducible audits are retained privately. No real model run or coding score
is claimed. No owned validation or model job remains.

Full endpoint model/context/proposal orchestration and live API composition,
native edit/test/retry, exclusive/durable ownership, approved Git publication
and parent scoreboard runs remain open. The current coordinator still admits
native runs only. No GPU/model run, evaluation, hidden/held-out access, scorer,
threshold, ignore, dependency, SBOM, status or release change is made.

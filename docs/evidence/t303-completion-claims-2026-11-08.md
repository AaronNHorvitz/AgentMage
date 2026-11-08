# T-303/T-306: model-independent completion verification

Non-closing core implementation under Decisions 0153 and 0154. Both parents
remain open; no new row or closure. Independent exact-commit core review is
required. The predecessor remains immutable. Its
[independent core review](../reviews/155621ea5260-local-coding-coordinator-scope.txt)
has **Verdict: PASS** for exact commit
`155621ea5260bbbc2c7e4037a4b44b6eccec98f5`, with no high/medium finding.
Low L1 (bind registry candidate admission to the owning run/request family)
and L2 (repeated scope verification) remain backlog with earlier lows. The new
endpoint claim binds its complete owning run, but full coordinator/registry
composition and cross-family candidate-admission coverage remain open.

The existing coordinator now calls the completion-claim verifier entry point.
The default adapter preserves original native request/proposal observations and
refuses endpoint claims before legacy ports. Native model admission, proposal
admission, permissions, state transitions, answer evidence, journals, artifacts,
checkpoints and verifier-registry proof validation are unchanged.

Endpoint completion claims require an accepted explicit turn and exact original
stream bytes. Complete sealed request, selected model, response length and
response SHA-256 bind the payload before bounded copying. The caller supplies
the trusted schema. The digest and derived proposal identity bind complete turn,
schema and payload. No native profile/codec or token/memory claim is synthesized.
The coding verifier shares its existing deterministic checks across inputs;
returned candidates remain inert until the existing registry verifies them.

New synthetic tests exercise the actual coordinator's new verifier entry,
original native pointer identity, approval rejection before effect, native
profile/codec mismatch, endpoint stream/run/model/policy/objective/schema drift,
legacy refusal before entry, changed/no-op candidates and registry verification,
and missing/duplicate receipts, stale evidence, missing validation and snapshot
mismatch. Existing test methods and assertions are preserved.

The complete scoped gate ran all six unchanged planned commands under one
shared reservation in **370.66 seconds**, excluding queue wait. Formatting
passed in 2.11 seconds and Clippy with warnings denied in 29.72 seconds. Tests
took 337.43 seconds: **43 targets, 2,564 passed, 72 failed, 71 ignored, zero
filtered**. All six new methods pass. Every predecessor passing identity is
preserved with exactly those six additions. Failure identities, normalized
diagnostics and ignored identities match the predecessor. The failures remain
53 host and 19 Linux library cases under deferred T-301.1. Cargo tests return
101 and the source gate remains **FAIL**; matching a failed baseline is not a
green gate. No real model run or coding score is claimed.

All five Rust input hashes remained fixed during the final gate. Static checks
preserve every original test body and ledger row, and prove the deterministic
verifier predicates, candidate construction and evidence selection differ only
by descriptive field access. Final light checks cover documentation, public
hygiene and immutable source-copy bindings. Private raw logs, source copies and
reproducible audits retain the complete results.

The initial gate failed compilation because its private digest preimage used a
versioned-contract-only serializer. The correction replaces that call with the
ordinary stable JSON serializer and removes its unused import; digest fields
are unchanged. A queued rerun was cancelled before Cargo to strengthen one new
negative fixture: add its missing receipt evidence and a passing control, then
vary one condition at a time. These exact source transformations are retained
and audited. No existing test body or assertion changed. No owned validation
or model job remains.

Full endpoint model/context/tool-proposal orchestration and live API composition,
answer-evidence provenance, exclusive/durable ownership, native edit/test/retry,
approved Git publication and parent scoreboard runs remain open. There is no
endpoint coding workflow or row closure. No GPU/model run, frozen-task,
hidden/held-out, scorer, threshold, ignore, dependency, SBOM, status or release
change is made.

# T-302.1: Carriage-return repair, 2026-10-25

Builder evidence for medium M2 in the independent review of
`23b3fe8ee67dc0f27190fa813e7ebae10e46cb49`. T-302.1 stays open pending its third
exact-commit review. No mission row is closed by this evidence.

## Review disposition

The [second independent report](../reviews/23b3fe8ee67d-local-agent-t302-1-protocol-repair.txt)
is retained byte-for-byte. Verdict: FINDINGS; no high finding. It confirms the
leading byte-order-mark repair M1 but identifies medium M2: a lone carriage
return can join a text-bearing data field to a preceding comment or inert
metadata, silently dropping text while allowing later completion.
Report SHA-256:
`5dd4f9edfa9bda83554be9c8f4a3f955b3f877d21ba8325f1c8e0623e83a478e`.

The decoder now removes the one CR belonging to CRLF, then refuses any remaining
CR before interpreting the line. It supports LF and CRLF as documented and
refuses lone CR framing. No budget, hash binding, permission or test threshold
changes. Model content with a JSON-escaped carriage return remains text.

The malformed-body regression test covers lone CR before data at stream start and in
the middle of an answer, after comments, id/retry/unknown metadata and inside a
data field; repeated CRs before LF; both LF and CRLF surroundings; every two-way
split and byte-at-a-time delivery. Each malformed body must return Protocol,
never produce Finished, and remain poisoned for end and subsequent feed.
A separate positive test preserves escaped CR/CRLF model content at every split.
These are codec tests, not transport, model, sandbox or application observations.

Low L6 (other malformed ASCII field names silently ignored) joins L1, L4, L5 and
the shared-slot timeout observation in the T-309 backlog. L2/L3 remain documented
consumer obligations. No consumer implementation is claimed. A third non-PASS
review requires a remaining-findings sub-row under Decision 0151; the builder
must not close the row or begin a fourth review loop.

## Verification

The complete touched-crate fast gate finished **FAIL**, exit 1. Command times
sum to 116.78 seconds including shared-reservation waits. Locked/offline Cargo
ran with two build jobs and four test threads, no test filters and no fail fast.
All 53 failed identities exactly equal the host appendix in the
[retained baseline](t301-baseline-failures-2026-10-25.md), with none added or
missing. Matching a baseline does not turn failures into a pass.

| Package | Passed | Failed | Existing ignored | Filtered |
| --- | --- | --- | --- | --- |
| Inference | 185 | 0 | 8 | 0 |
| Host reverse dependent | 532 | 53 | 11 | 0 |
| Total | 717 | 53 | 19 | 0 |

All 19 protocol tests passed, including the two new regressions. System Git and
sandbox executables still appear owned by overflow UID 65534. The unchanged
trust checks correctly refuse them; T-301.1 remains an owner-action blocker.
No test was newly ignored, skipped or allowlisted. No trust check was weakened.

| Fast-gate command | Exit | Seconds including reservation wait |
| --- | --- | --- |
| Range whitespace | 0 | 0.10 |
| Untracked whitespace | 0 | 0.10 |
| Immutable-review format | 0 | 0.10 |
| Markdown lint, three files | 0 | 1.15 |
| Touched-package formatting | 0 | 6.71 |
| All-target warnings-denied Clippy | 0 | 31.25 |
| All-target tests, no fail fast | 101 | 77.37 |

With Cargo idle, the document/link/policy validator passed for 683 Markdown
files and all policy invariants (183.72 seconds including reservation wait).
This is not the long acceptance gate. Final evidence text was then checked with
scoped Markdown lint and whitespace checks. Immutable-review comparison,
added-text public-path hygiene, historical-ledger comparison and unchanged global
inputs were checked. Tested source hashes remain unchanged. Raw results remain
in private lane state; independent review is required for acceptance.

## Scope limits

The component remains effect-free. No socket, credential, file access, process,
provider route, model qualification or UI is added. No external product source or
documentation was consulted, dependency added or model acquired. No GPU/model
run, long acceptance gate, public release or owner readiness is claimed.
T-301.1 remains open. The historical ledger, status model and supply-chain outputs
are unchanged. Source changes belong to the T-302 batch; no per-edit regeneration
or evidence-binding reduction was performed.

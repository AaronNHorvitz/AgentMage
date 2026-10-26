# T-302.1: Protocol review repair, 2026-10-25

Builder evidence for the repair of medium M1 in the independent review of
`f18838af658e232a2f2ed9564f1ad6c5d0731760`. T-302.1 stays open pending exact-commit
re-review; the parent and all other mission rows stay open.

## Review disposition

The [independent report](../reviews/f18838af658e-local-agent-t302-1-protocol.txt)
is retained unchanged. Verdict: FINDINGS. No high finding; medium M1 blocks
closure. A leading UTF-8 byte-order mark made the first field unrecognizable,
silently dropping that event while allowing later text and completion to succeed.
Report SHA-256:
`80a6bc228d7c412865b458dcaecb41cd5c22f0067d2f0332e20ac5f0d4615335`.

The decoder now strips exactly one mark at body offset zero from the buffered
first line. Buffering already spans arbitrary fragments, including splits inside
the mark. All input bytes still count toward both budgets. Non-ASCII field names
fail closed, so a repeated or misplaced mark cannot silently hide another field.
JSON text is preserved, including a mark inside content. No ceiling changed.

Three regression tests cover a text-bearing first event with a leading mark at
every two-way split and byte-at-a-time delivery, both LF and CRLF; repeated and
misplaced marks and another non-ASCII field prefix at every split; preserved
in-content marks; incomplete mark prefixes, poisoned state and exact event-budget
accounting. These are synthetic codec tests, not model or transport observations.

Low L2 and L3 are addressed as consumer obligations in the
[protocol guide](../guides/local-chat-protocol.md): completion remains provisional
until successful transport EOF; later errors revoke it; neutralize terminal
controls, expose bidirectional overrides and keep model output out of logs.
There is still no consumer implementation here. Low L1 (duplicate JSON keys),
L4 (pasted full route as API base), L5 (debug formatting exposes output), and the
pre-existing shared-slot wait timeout observation remain backlog for T-309.

## Verification

The complete touched-crate fast gate ran with the shared build reservation,
two build jobs and four test threads, using locked/offline Cargo and no test
filters. It finished **FAIL**, exit 1, in 82.98 seconds including reservation
waits. All 53 failed identities exactly match the host appendix in the
[retained baseline](t301-baseline-failures-2026-10-25.md), with none added or
missing. Identity equality does not turn a failure into a pass.

| Package | Passed | Failed | Existing ignored | Filtered |
| --- | --- | --- | --- | --- |
| Inference | 183 | 0 | 8 | 0 |
| Host reverse dependent | 532 | 53 | 11 | 0 |
| Total | 715 | 53 | 19 | 0 |

All 17 protocol tests passed, including the three new regressions. The host's
pre-existing standalone process fixtures passed but do not exercise this codec
or a language model. System executables still appear owned by UID 65534 in the
lane; the unchanged root-ownership checks correctly refuse them. T-301.1 remains
an owner-action blocker. No tests were newly ignored, skipped or allowlisted.

| Fast-gate command | Exit | Seconds including reservation wait |
| --- | --- | --- |
| Range whitespace | 0 | 0.10 |
| Untracked whitespace | 0 | 0.10 |
| Immutable-review format | 0 | 0.10 |
| Markdown lint, three files | 0 | 1.15 |
| Touched-package formatting | 0 | 3.86 |
| All-target warnings-denied Clippy | 0 | 7.31 |
| All-target tests, no fail fast | 101 | 70.28 |

With Cargo idle, the document/link/policy validator passed for 682 Markdown
files and all policy invariants (164.73 seconds including the shared reservation
wait). This is not the long acceptance gate. Final evidence text was then checked
again with scoped Markdown lint and whitespace checks. The immutable report byte
comparison, new-text public-path hygiene, historical-ledger comparison and checks
that manifests, lockfile, status model and supply-chain outputs are unchanged
passed. No source changed after testing. Raw results remain in private lane state.
No row is accepted by this builder evidence; independent re-review is required.

## Scope limits

The component remains effect-free: no socket, key, file access, process, provider
route, model qualification or UI. No third-party implementation or documentation
was consulted, no dependency added and no model acquired. No GPU or real model
run, public release, owner readiness or long acceptance gate is claimed.
The inherited full-Rust-gate blocker T-301.1 remains open. Source changes belong
to the T-302 batch; no SBOM or historical evidence cascade has been regenerated,
and no hash binding or trust check has been weakened. The historical ledger and
status model remain unchanged.

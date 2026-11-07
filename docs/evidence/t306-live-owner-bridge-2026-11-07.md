# T-303/T-306: Live creation authority in native writes

Date: 2026-11-07. Same open parent rows; no new row or parent acceptance.

## Scope

The [prior independent repair review](../reviews/637cffbc7846-local-coding-linked-worker-repair.txt)
has **Verdict: PASS** for exact commit
`637cffbc78460d5dfbcd0cd14fc21f97617c7277`. Retain its bytes unchanged. It accepts
only that repair; low L2 (descriptor identity at final linked admission) remains
backlog. Complete native linked startup is still unverified under deferred
T-301.1.

[Decision 0158](../decisions/0158-live-worktree-authority-in-the-native-coordinator.md)
adds a typed in-process handoff from the existing manual repository owner into
the existing coordinator. Only live successful creation provenance can construct
the shared lease. Pending manual decisions are discarded. Concurrent writers
refuse; manual recovery requires every other lease to end. Poisoned recovery
discards creation ownership and pending decisions.

The coordinator factory checks its independently activated workspace against
this owner before preparation, refresh and composition. Its structured existing
file, hunk and rollback writes use the same native atomic driver and exact
kernel approvals. A wrapper removes old transfer eligibility before actual
application and promotes cumulative lineage only after a successful native
application, matching verified committed result and terminal publication.
Failed, unpublished or mismatched completion leaves attempted paths ineligible.
The original checkout still requires a fresh complete diff and separate exact
approval. No caller path, marker or receipt can adopt ownership.

The ordinary unowned coordinator path uses the same native writer through an
inert wrapper. The manual pending writer extracts its unchanged grant issuance
and transaction into one private helper, allowing tests to exercise the exact
kernel/native writer with the owned wrapper. The test-only adapter is absent
from production builds and exposes no HTTP route or authority constructor.

## Verification

All commands used the pinned toolchain and locked offline dependencies, two
build jobs and four test threads. Heavy checks honored the shared build
reservation. Seven changed Rust inputs were frozen after formatting; their
hashes still match after the complete gate.

| Check | Observed result |
| --- | --- |
| Reserved focused `coding_repository::apply::tests::runtime_` | PASS: 7 passed, 0 failed, 0 ignored, 312 filtered; 1.21 seconds of tests, 14.65 seconds of compilation |
| Cumulative native-write/original-apply case | PASS: 478,690 microseconds including fixture setup, authority, writes and assertions; synthetic creation, no coordinator/model |
| Fast gate formatting | PASS; 12.02 seconds including reservation wait |
| Fast gate Clippy, all selected targets, warnings denied | PASS; 29.65 seconds including reservation wait |
| Complete selected-crate tests, no fail-fast | FAIL: 907 passed, 72 failed, 63 ignored, 0 filtered; 374.31 seconds including reservation wait and compilation |
| Failure identity comparison | Exact same 53 host and 19 Linux failures as the prior complete gate; no new failure |
| Diff, untracked-source, retained-review and initial Markdown checks | PASS |

The seven focused methods exercise cumulative manual/native edits followed by
separately approved original transfer; rejection of reopened owners, wrong roots
and substituted directories; unpublished native writes; foreign/concurrent
writers; broken lineage with competing bytes preserved; wrong transaction,
consumed-grant and receipt-chain completion; and panic/poison recovery with no
creation authority or pending approval. The intentional worker-thread panic is
caught by its regression; that method passes. The native coordinator refuses an
owned structured write on the legacy direct path without an atomic tool-terminal
publication owner.

The complete gate ran once over the final source. Every new method also passes
in that complete run. All default ignores are unchanged. The 72 failures remain
FAIL under the previously recorded T-301.1 environment; neither matching the
baseline nor passing these components establishes native acceptance. Two earlier
queued check wrappers were canceled before Cargo execution while arranging the
bounded source-check sequence. Their logs are retained; they establish no build
or test result. Only owned wait processes were cleaned.

Post-measurement Markdown lint passes for all four changed Markdown files;
retained-review lint also passes. The final private review request binds the
complete source-policy manifest and exact staging audit to the reviewed commit.
These checks do not change the full Rust gate's FAIL result.

Raw logs and frozen source hashes are retained under
`<lane-state>/tmp/t306-owned-runtime/`:

| Artifact | SHA-256 |
| --- | --- |
| `fast.log` | `dcae63357ff234f2d1ac9e80dee069ebc575841f832f6fa7f6bafa0b7c02fa13` |
| `component.log` | `35f728aead7153a986b93c35290f37b00c6b65493ec173eb3c77c635027f8fc8` |
| `source-freeze.json` | `1be852c8fabcb020c1d10f11025a16fda5ebf28c90aceaa35570b514b40f00bb` |
| `test-verification.json` | `a8e755ba7d0542e334790a4bc250b2d55955e56041f206e04136a6cfbe5eb6be` |

## Limits

The regression fixtures inject synthetic creation provenance inside the owning
module and use real held directories, kernel grants and descriptor-relative
atomic writes. They do not demonstrate native Git creation, a complete native
coordinator, model inference or a coding score. The existing executable-trust
checks remain unchanged.

This is library composition. The separate-process HTTP native-host launcher
cannot transfer the lease and still has no manual-owner lifecycle integration.
No lease can be serialized or restored after restart. New-file transfer,
configured-endpoint coding admission, positive native fail/repair, durable HTTP
recovery, approved branch commit/push, MCP and parent measurements remain open.

No hidden input, held-out task, threshold, dependency, SBOM, status model,
scoreboard or release state changes. No GPU or model process starts. All ledger
checkboxes remain unchanged. Independent core review is required.

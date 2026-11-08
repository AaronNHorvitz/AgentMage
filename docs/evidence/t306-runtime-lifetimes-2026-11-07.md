# T-303/T-306: runtime resource lifetimes

Date: 2026-11-07. Non-closing core batch under Decisions 0153 and 0154.
Both parents remain open; independent exact-commit review is required.

## Change

The development runtime factory permanently allocated its platform adapter,
immutable coding profile and descriptor-bound workspace with `Box::leak`.
Repository refresh allocated another profile and held workspace without releasing
the old ones. Dropping an attached factory could not release these objects.

The factory now shares these objects through `Arc`. An owned workspace binding
retains its platform and profile; the existing borrowed binding remains available.
Active coordinators retain the same binding through a shared workspace handle.
Refreshing the factory replaces its references, without replacing a live run's
binding. Failed construction drops partially built owned objects normally.

Prepared operations retain the exact already-held authorized root through `Arc`,
along with their original held targets, preimages and inert plans. They do not
reopen a path or reconstruct creation authority. Effect drivers borrow or own
that same root for the bounded operation; write/create drivers keep their existing
live worktree lease. Existing target revalidation, approvals, grant consumption,
terminal publication, ownership checks and native confinement still apply.
The last owner releases each object and its descriptors.

The existing native-boundary test fixture now shares its workspace instead of
leaking it; its synthetic profile is still the same borrowed static test fixture.
An explicit fixture profile field keeps its original test-only lifetime. No
production profile is given a fabricated static lifetime. No unsafe code or
new dependency is introduced.

## Verification

The final reserved source gate ran all seven selected commands in 121.71
seconds, excluding the reservation wait. It remains **FAIL**: Cargo tests
return 101 on the same 53 host failures, so the gate returns 1. Formatting
(0.80 seconds), Clippy with warnings denied (11.47 seconds), whitespace,
changed-document lint and retained-review format checks all pass. The complete
host and local-API source tests take 107.99 seconds across 18 targets:
**685 passed, 53 failed, 11 ignored, 0 filtered**. All four new methods pass
unfiltered. Both failure identities and normalized messages match the host
portion of the previously retained created-file-transfer source gate; all eleven
host/native-cancellation ignore identities are unchanged. That witness included
the Linux platform package as well; the current reverse-dependency selection
includes only the host and local API. This comparison describes the baseline;
it never turns a failing gate into a pass.

Private logs retain the exploratory compiler errors, a first focused run with
three passes and one incorrect new-test expectation, and the first complete
source gate's Clippy errors. A fresh scoped read is allowed to observe changed
bytes; the corrected assertion requires a different target observation while
preserving the stale-operation refusal. The first full run passed all four
new tests; its five Clippy diagnostics were resolved with a named composition
aggregate and removal of redundant test-fixture conversions. No existing test
assertion, rejection, threshold or ignore was weakened. One builder-owned
reservation waiter was cancelled before formatting or test execution to combine
the corrected tests with the complete source gate.

Final lightweight checks cover the changed prose and report after these result
paragraphs were added. They verify the same four Rust hashes and identical Cargo
selection, the exact eight-file scope, unchanged ledger rows, byte-identical
review, whitespace and privacy. They run no build, source suite or evidence
regeneration. Raw logs, plans, source hashes, diagnostics comparison and the
review-input manifest remain private under the lane state directory.

Four new tests use real private directories, descriptor-bound source reads and
the real development binding/operation planner, with synthetic profile and map
provenance. They execute no tool effect, Git command or model. They cover:

- Owned and borrowed bindings produce the same operation-plan digest. A pending
  read survives release of the binding, profile and platform; its root remains
  held until the operation drops. Unique-tree descriptor counts then return to
  zero and the source bytes remain unchanged.
- Repository binding refresh releases the old binding and profile while its
  pending read retains the old root. Changed source bytes refuse that old
  operation. A fresh read binds a different target observation; it does not
  reuse the old operation or authorize a write.
- A shared runtime handle is moved into an actual worker thread after the
  factory-like references drop. The worker prepares a read and ends; the read
  still revalidates, and dropping it releases the last descriptors.
- Sixty-four binding, pending-read and wrong-root refusal cycles leave no held
  descriptor under the test's unique tree and no retained profile or platform.
  The check excludes unrelated concurrent-test descriptors.

The complete factory still cannot reach native Git composition in this lane:
trusted system executables and ancestors appear under the overflow UID.
No native trust check is relaxed. These tests are binding and lifetime evidence,
not a positive native attach/run/recover workflow or durable ownership recovery.

The earlier approval-clarity [independent report](../reviews/b716ab5d34f1-local-coding-approval-clarity.txt)
is retained unchanged, with **Verdict: PASS** for exact commit
`b716ab5d34f18f804b9da26cb7052387e0f11fb3`. It closes neither parent and does
not accept this lifetime batch. Its two low findings remain backlog in-row.

## Limits

No prompt, model admission, profile schema, tokenizer, endpoint, scorer, hidden
test, held-out input or threshold changes. The prior coding scores remain the
only measured model scores. No new development or held-out coding score is
claimed: the native workflow retains its host-trust blocker. No GPU work ran.
There is no serialized lease, restart adoption, commit/push capability, parent
closure or release in this batch. Historical hash-bound evidence and SBOM outputs
are unchanged; no broad documentation or acceptance gate is claimed.

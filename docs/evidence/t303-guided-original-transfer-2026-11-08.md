# T-303/T-306: Guided original-checkout transfer

Date: 2026-11-08. Tier: core. Builder evidence; independent acceptance pending.
Predecessor: `c53ed1742a51a7827a70e65a26c2fcdce70180ff`.
Both parent rows remain open. No row is added or closed.

## Running capability

The explicit `code-repository-apply` command uses the repository setup and
original guided proposal/edit/test/repair workflow, then offers the original
native owner's transfer operation. The ordinary `code-repository` behavior
stays available. Every action uses its existing authenticated local HTTP route
and native owner. The command introduces no tool executor or permission grant.
See the [guide](../guides/coding-guided-worktree.md) for arguments and approvals.

Transfer begins only after the original verifier observes a full passing
worktree test receipt. It requires a nonempty sorted unique selection of at
most sixteen transferable paths, all from the workflow's approved source set.
The repository and worktree must still match the original setup bindings.
The original prepare helper asks for separate source-read consent, keeping both
root identities in the original API request. The returned complete preview
must retain the original repository, worktree and creation receipt. The final
approval helper must observe that exact preview, including its full body.
Returned result identity is checked before reporting transfer success.

All requests and approvals share the original monotonic running-time budget
and stop latch. Denial, expiry, signal, a changed binding or an uncertain reply
stops further requests. The command never repeats an effect or sends implicit
cancellation, denial or cleanup. The existing native preimage and postimage
checks remain authoritative. An approved operation can complete after a lost
reply; inspect files and the apply record before continuing.

The final report explicitly says the original checkout was not tested. A prior
passing receipt describes its worktree execution, not current source freshness
or an independent task grade. Commit and push remain unfinished work.

## Observations and verification

The terminal success case connects setup, an assertion failure, separate failure
disclosure, a repaired passing test, original read approval and exact original
diff approval. Additional methods cover refusal after exhausted, zero-test and
flaky results; each transfer denial; fifteen changed root, path, ownership and
preview cases; lost prepare/decision replies and three substituted results;
expiry and all three stop signals at each new approval; complete invalid-budget
arguments and piped execution. A cumulative-clock case spends 0.9 seconds at
initial inspection and waits at original-read approval under a two-second total
budget. It stops near two seconds without renewing the setup clock.

These terminal observations use a recording synthetic API peer. Fixture
worktrees preexist; the peer runs no actual write, command or model. The complete
case verifies original human bytes and worktree fixture bytes remain unchanged.
Eight transfer methods pass in the final complete terminal target. An initial
focused development run passed the first seven methods before adding the
invalid-budget/piped method; its raw log is retained, without treating it as
verification of the final source snapshot.

The actual service/terminal consumer reaches the unchanged native inspection
refusal, matching the direct original route. No source discovery, model request,
creation or later approval follows. It preserves the original file, empty
management directory, and all six inspected states, including original apply.
This is refusal evidence only; it establishes no successful native worktree,
edit, command, model request or original transfer.

The ordinary complete scoped gate passes **188 tests across ten source targets,
0 failed, 0 ignored, 0 filtered**. Exact method comparison retains all 179
predecessor identities and adds exactly nine: eight terminal transfer methods
and the actual-service refusal method. All seven changed Rust inputs were frozen
before the gate and remain unchanged; no source edit followed that gate.

All six gate commands exit 0: whitespace, untracked additions, documentation
lint, Rust formatting, strict Clippy and complete source tests. Total elapsed
time is 35.65 seconds including each shared reservation. Formatting takes 0.25
seconds, Clippy 1.55 seconds and complete source tests 32.41 seconds. The already
installed Node runtime and its licence are shared through hard links; no
download or binary copy. Current documentation/privacy/scope/unchanged-ledger
checks also pass after this evidence update. This is the scoped development
gate, not full-workspace or native acceptance. Deferred T-301.1 is unchanged.

The predecessor's [independent core review](../reviews/c53ed1742a51-local-coding-repository-setup.txt)
arrived after the source gate. It has **Verdict: PASS** for its exact commit,
with no high/medium finding; the raw report is retained unchanged. Low L1
(creation-reply root/outcome substitution coverage) remains same-row backlog;
low L2's manual-setup guide transition is clarified. Earlier lows remain.
All four current lightweight checks, including the newly retained report's
format check, pass after evidence and review updates. The Rust freeze is
unchanged. Independent core review is required for this increment and neither
parent row closes. Native success, local-model scores, autonomous endpoint
composition, durable recovery, approved Git and parent measurements remain
open. No evaluation, hidden/held-out test, scorer, threshold, model, GPU,
dependency, SBOM, status, scoreboard, release or push change occurs.

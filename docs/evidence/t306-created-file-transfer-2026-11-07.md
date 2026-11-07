# Verified new-file transfer — 2026-11-07

This is a non-closing T-306/T-303 implementation increment under
[Decision 0160](../decisions/0160-created-file-original-transfer.md).
Independent core review is pending; both parent rows stay open. The preceding
[recovery review](../reviews/198af125dfdc-local-coding-bound-recovery.txt) has
**Verdict: PASS** for exact commit
`198af125dfdc7a22d46b8905c6a78da5907c0311`, covering only that earlier increment.
Its low L1, inconsistent terminal status/owner snapshots, is recorded as backlog;
it does not permit recovery of a different handoff.

The live worktree owner now wraps the existing controlled-filesystem creation
driver as well as the existing-file atomic writer. The owner lock spans native
creation, reconciliation and terminal publication. Creation eligibility requires
that wrapper's actual successful apply and matching committed, verified receipts,
including path, mode, postimage, transaction and consumed grant. A manually added
file, a zero preimage, a caller receipt or a model answer creates no eligibility.
Subsequent native or manual approved edits preserve absent-before provenance.

After explicit read consent, the existing original-apply API/CLI prepares a
complete escaped creation preview, with the original native mode. Preparation
observes bounded parent-directory names and requires an absent destination.
The terminal discloses these reads. The existing kernel filesystem plan,
single-use grant, native no-overwrite creation and reconciliation perform the
separately approved transfer. Inspection labels each path as an edit or creation.
Mixed-kind previews refuse rather than committing two transactions under one
approval. Missing parents are not created. Existing-file transfers retain their
original shadow-write machinery. Both kinds share the sixteen-path limit.

## Verification

The final batch contains nineteen paths, including thirteen Rust files and the
unchanged predecessor report. All thirteen source hashes remained identical
through the complete source gate. The original fast-gate planner and executor
selected formatting, Clippy and complete unit/integration/binary targets for
`agentmage-platform-linux`, `agentmage-host` and `agentmage-local-api`. One
verified held build reservation covered the focused checks and full gate; the
Cargo environment prefix prevented recursive reservation acquisition only.

| Check | Actual result | Elapsed, excluding reservation wait |
| --- | --- | --- |
| Focused native original-apply/ownership tests | 29 passed, zero failed/ignored, 301 filtered | 9.151 s including compilation; 7.31 s test execution |
| Complete terminal worktree target | 13 passed, zero failed/ignored/filtered | 34.677 s including compilation; 7.24 s test execution |
| Complete per-change gate, seven commands | **FAIL**; 22 targets, 945 passed, 72 failed, 63 pre-existing ignored, zero filtered | 183.59 s summed command time |
| Gate formatting / Clippy with warnings denied | Both exit zero | 1.00 s / 28.45 s |
| Gate complete test command | Exit 101; no failure allowance or test filter | 152.64 s |

All eight new regression methods pass in both the focused and unfiltered full
runs. They cover native creation followed by native and manual edits; exact
contents/modes and separate original approval; an occupied later destination,
empty files, directories, links, source/parent/root drift and sibling changes;
expiry, clock rollback, denial, foreign decisions and no replay; manual/reopened
file refusal; mixed selections; shared locking and the combined sixteen-path
limit. Valid rehashed receipts for different paths or modes still cannot create
lineage. Unpublished or malformed results leave native-created files ineligible.
Existing terminal tests additionally exercise complete escaped creation previews,
exact typed approval, original-root replacement, and the parent-name read
consent disclosure. These are synthetic success peers and real service refusals.

The positive component case body measured **434,456 microseconds**: two native
file creations, a native edit, a manual edit, a two-file original transfer, then
a further existing-file edit and fresh existing-file preview. Both original
modes/contents are checked; unrelated human bytes and the synthetic index
sentinel remain unchanged. This timing excludes compilation, reservation wait
and fixture teardown. It is not an agent task score.

The 72 failure identities and all 63 ignore identities equal the retained
bound-recovery baseline exactly. All 72 failure diagnostics also match after
normalizing only thread IDs, source locations and checkout prefixes. The
executable-trust blocker remains: 53 host and 19 Linux failures. The source gate
remains **FAIL**, not a qualified native workflow. No ignore, threshold or test
selection was changed to make the comparison pass.

Earlier raw attempts are retained. The first stopped at two typed path-component
conversion errors before tests. The second compiled and ran 22 passing and seven
failing methods: the new creation plans supplied an empty verification list,
which the existing kernel correctly refuses before effects. The composition now
declares `project-tests` as an unexecuted follow-up and preserves its explicit
separate-command-grant requirement in the result. No command runs or receives a
grant from this label. The kernel rule and all assertions remain intact. A
redundant initial compile-only reservation waiter was cancelled before Cargo
started; its wrapper exit is not a test outcome.

Final documentation checks cover all five changed Markdown files and the
unchanged retained review. Policy validates 799 Markdown files and all
invariants, with all 9,062 current source-input hashes retained. Privacy,
exact-path, ledger and source-hash audits pass. Raw logs and complete input
manifests are retained in lane state and supplied in the private review request. No further Rust check is claimed by a documentation
follow-up; it verifies that the tested source bytes are unchanged.

## Limits

Worktree-creation provenance in component fixtures is synthetic; held roots,
grants, filesystem transactions and resulting file writes are real. The component
harness chooses whether to call completion; it does not run host journal
publication. The host's post-publication call ordering is covered by source
inspection, not an observed owner-bound coordinator creation workflow.
These tests are not native Git worktree creation, model inference, connected
agent execution or a coding score. The lane's executable-ownership blocker
remains deferred. Native create/run/apply integration must still be observed
before parent acceptance. This batch changes no model, prompt, endpoint,
evaluation input, hidden test, held-out input, threshold, dependency, SBOM,
status model or release gate. No new model campaign or parent closure is claimed.

Ownership, eligibility, previews and latest results remain process-only.
Restart does not restore them; uncertain or lost replies require disk inspection,
never automatic replay. New-file content must meet the normal nonempty UTF-8,
NUL-free and aggregate 128 KiB selected-source bounds. Mixed edit/creation
selections require separate previews and separate approvals.
Configured-endpoint composition, durable ownership, approved commit/push, MCP
and measured parent completion remain open. The existing lifecycle/continuation,
factory-allocation, standard-library-shadowing and terminal snapshot-consistency
low backlog remains recorded. No public release or owner-test readiness is
established.

# T-303/T-306: keep package discovery initializers outside mutable sources

Base: `1fde68be241fa33f31b75b255b62f82be9778444`, whose terminal ownership
increment subsequently received an [independent core PASS](../reviews/1fde68be241f-local-coding-terminal-ownership.txt)
with no open high or medium finding. That report is retained unchanged. Its low
L1 overstatement in the prior source-selection record is corrected here; low L2
(recovery consent does not bind the displayed owner) remains backlog. This is a
same-row repair of low L1 from the [source-selection review](../reviews/a8bdb80dc506-local-coding-explicit-python-sources.txt).
Neither parent closes; independent core review is required.

## Behavior

The existing Python source predicate now excludes `__init__.py` basenames in
both project layouts and both default and explicit selections. This prevents
an editable package initializer from using `load_tests` to replace subtree test
discovery. Refuse all initializers conservatively, including a package with no
visible tests: a partial repository map cannot establish the absence of tests.
The initializer remains available to repository inspection, imports and the
test runner. No file is removed or rewritten by admission.

An explicit list containing an initializer refuses as a whole, even when it
also contains an allowed source. The shared constructor rejects it before
HTTP owner extraction; the terminal parser refuses before requests or consent.
Default selection skips initializers and refuses when no allowed sources remain.
The existing exact-file scope enforces the resulting paths for patches. Nothing
changes the interpreter command, parser, positive-test minimum or tool approvals.

The existing complete profile digest binds the narrowed default scope on
resume. A prior request whose scope included an initializer refuses as drift;
start a new run with fresh consent. Explicit selections of other files retain
their existing identity. No checkpoint or request is migrated or re-sealed.

The trusted project runner still is not independent grading: another imported
source can monkeypatch test behavior. This closes only the specific initializer
admission gap when accepted by the independent reviewer. Coding scores still
come only from the unchanged frozen scorer. No model or evaluation input is used
or changed by these regressions.

## Verification

Locked/offline checks used two build jobs and four test threads under the shared
build reservation, after waiting for capacity. Formatting preceded the source
freeze; the same reservation covered focused checks and the original fast-gate
planner/executor. Cargo alone receives an `env` prefix to avoid recursively
acquiring the already-held reservation; no target or failure handling changes.

| Check | Observed result, including compilation |
| --- | --- |
| Focused host Python project tests | 14 passed, 587 filtered; 10.025112 seconds |
| Focused API coding contracts | 22 passed, 5 filtered; 4.699249 seconds |
| Complete terminal runtime contract target | 21 passed, none filtered; 5.586778 seconds |
| Fast-gate formatting / Clippy with warnings denied | PASS; 0.75 / 7.06 seconds |

All 57 focused methods pass with zero failures or ignores. The new interpreter
regression took 170,722 microseconds; the expanded eight-exchange real loopback
refusal probe took 3,163 microseconds. Both remain component observations, not
native confined execution, model inference or an independent score.

The new interpreter regression reproduces the review's failing test, changes
only the initializer to make discovery falsely report one passing test, then
restores it and observes failure again. Source and test bytes stay unchanged.
Default and explicit scopes refuse initializer and test edits while keeping the
ordinary source writable. Mixed selection, nested initializers, initializer-only
defaults, API refusal before owner extraction, real loopback refusal and terminal
refusal without a request extend the existing contracts.

The complete gate remains **FAIL**: 666 passed, 53 failed, 11 pre-existing
ignores, zero filtered, across 17 target summaries. The test command took
87.91 seconds, including compilation; the six gate commands total 97.12 seconds,
excluding the shared reservation wait. Its 53 host failure identities exactly
match the retained prior full source-selection gate. The new regression and all
expanded API/terminal methods pass inside the unfiltered run. No failure is
repaired, removed or allowlisted by this batch. The untouched Linux crate is not
selected; its separately retained 19 baseline failures are not in these totals.

Final Markdown, whitespace and source-policy checks cover the finished report.
The private review request binds the raw logs, unchanged baseline failure list,
complete source-policy input manifest and source freeze. The read-only audit
checks the exact ten-path scope, every other ledger line unchanged, both
parents open, all four Rust source hashes unchanged since formatting, full test
totals and exact failure identities. No Rust source changes after verification. The source gate covered eight paths
before the predecessor verdict arrived; the final ten-path documentation scope
adds its unchanged report and the older evidence wording correction. Final
checks also lint that raw report, verify its byte identity and check the final
plan; they do not repeat source tests or promote the complete gate to PASS.

## Remaining scope

Native positive execution, configured coding endpoint admission, durable
ownership, new-file transfer, approved commit/push, MCP and parent measurements
remain open. The T-301.1 executable-trust blocker is unchanged. Recovery consent still targets
the service's current owner rather than binding the displayed owner; that low
finding, lifecycle/continuation coverage and factory allocations remain backlog. No hidden or
held-out test, threshold, benchmark score, dependency, SBOM, lifecycle status or
release authority changes. Historical hash-bound evidence is not regenerated.

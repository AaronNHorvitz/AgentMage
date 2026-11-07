# T-303/T-306: explicit Python source files

Base: `be214302706ac1caee2b7c0e54d2009f7864e8c8`. Same-row implementation
under the accepted coding mission; both parents remain open. The preceding
[root-selection core review](../reviews/be214302706a-local-coding-root-python.txt)
is retained unchanged with **Verdict: PASS** for that exact commit and no open
high/medium finding. This batch addresses L2 and partly addresses L1 (see the review note below)
while adding explicit source selection through the existing owned HTTP handoff.

Subsequent [independent core review](../reviews/a8bdb80dc506-local-coding-explicit-python-sources.txt)
has **Verdict: PASS** for exact commit `a8bdb80dc506603b0a8335013e586214b894be69`,
with no open high or medium finding. It clarifies that prior L1 is partly fixed:
a writable package initializer can still alter root discovery through its test
loading hook. That remainder is low backlog and must be resolved before citing
root-layout output as coding-success evidence; prior L2 is fixed. The point-in-time
measurements below are unchanged and close neither parent row.

## Behavior

The owned attach route and native host factory accept an optional list of one
to sixteen unique existing Python source files, including sources outside
`src/`. The list replaces the default, is canonicalized and frozen at startup,
and must resolve completely to admitted, read, parsed regular repository-map
files. No partial fallback is allowed. Invalid syntax, duplicates, empty,
missing, unread, excluded and symbolic-link entries refuse. Inspection and
handoff consent remain separate from subsequent model and tool approvals.

Both project layouts now exclude discovery modules matching `test*.py`, test
directories named `test` or `tests`, and hidden components from their writable
sources. This partly addresses review L1, including the original `src/test_calc.py`
case (see the review note above). A conservative refusal also covers ordinary source modules starting
with `test`. Exact-file write scope, strict patch planning, creation refusal,
command confinement, bounds and positive-test minimum are unchanged.

The same list is checked on every repository refresh and bound into an exact
resume constraint. A changed, inserted, removed or duplicated marker refuses.
The prepared work packet discloses actual mutable files. Startup status shows
the canonical selected paths; prepare cannot replace them. The standalone
launcher retains the default `src/` rule. This optional selection is exposed by
the owned attach API and host constructor, not a new launcher flag.

The root validation source definition now labels its actual layout, addressing
review L2. A mismatched layout/command is refused. Root definition hashes change
intentionally: older root requests require a fresh run and consent; package
source definitions retain their previous bytes. A changed source scope also
requires a fresh run. A separate required project-profile digest binds the
complete scope and validation registry on resume, including the default source
selection. Earlier project requests without this binding require fresh consent
as well. Nothing migrates or re-seals an existing request.

## Verification

Locked/offline verification used two build jobs and four test threads after
waiting for the shared build reservation. One verified outer reservation covered
the final focused checks and unchanged fast-gate planner/executor; only Cargo's
executor prefix was changed to avoid recursively acquiring the held lock.
The complete target selection, supervisors and failure handling were unchanged.

| Focused check | Observed result, including compilation |
| --- | --- |
| Host Python project composition and interpreter fixtures | 13 passed / 587 filtered; 1.983215 seconds |
| API coding contracts | 22 passed / 5 filtered; 11.155190 seconds |
| Existing terminal run driver | 1 passed / 14 filtered; 4.659708 seconds |
| Fast-gate formatting / Clippy with warnings denied | PASS; 0.80 / 9.42 seconds |

All 36 focused methods have zero failures or ignores. An earlier host-only
compile/check also passed all 13 methods; its compilation took 14.43 seconds.
The terminal method retains six synthetic HTTP/PTY approval/denial cases. The
owned API's expanded actual loopback test executes seven refusal exchanges,
including explicit source syntax, client-credential refusal and content-free
errors. Other contracts check optional/default shape, invalid input before owner
extraction, selection disclosure and refusal of a prepare-body override.
No successful native handoff is inferred from these refusals or scripted peers.

The new trusted interpreter fixture maps a top-level `calculator.py`, observes
an actual failing assertion, builds a strict exact-preimage syntax-node patch
within the selected scope, writes its proposed postimage and observes one passing
test. Patching `test_calc.py` is refused and its bytes remain unchanged. This
two-interpreter-run fixture took 670,256 microseconds in the final focused run.
It executes outside the native sandbox and writes from trusted fixture code;
it is component evidence, not native approval, model or workflow evidence.
Unit checks additionally cover source exclusions, path and count bounds,
canonical ordering, all-input admission, no fallback, exact scope, explicit
selection resume markers and the complete project-profile resume binding.

The complete gate remains **FAIL**: 659 passed, the same 53 host failures,
11 pre-existing ignores and zero filtered tests over 17 target summaries;
94.92 seconds including compilation. Its host failure identities exactly equal
the previous batch's host failure set. Failures still report the deferred
T-301.1 native sandbox manifest/trust blocker. The untouched Linux crate is not
a target in this batch; its previous 19 failures and 52 ignores are not part of
these totals. This target-selection difference is not a repaired failure or an
added skip. There is no new or removed host failure, new ignore, threshold
change or failure allowlist. All nine new test methods pass in the complete run.

Final four-file Markdown, unchanged-review, whitespace and source-policy checks
cover the finished report. The private request retains raw logs, exact source
freeze, complete policy-input manifest and commit-scope audit. No historical
hash-bound evidence or SBOM is regenerated. No development or held-out model
score is published: native positive execution remains unobserved, so no parent
row closes and no fixture is substituted for a model campaign.

## Remaining scope

The T-301.1 executable-ownership blocker, native positive create/run/recover/apply
workflow, arbitrary configured coding endpoint and rendered-token admission,
durable ownership, new-file transfer, approved commit/push, MCP and parent
measurements remain open. Prior continuation/API-recovery coverage and factory
allocation lifetime findings remain low backlog. No evaluation task, hidden
test, held-out input, threshold, coding score, dependency, SBOM, lifecycle or
release authority changes.

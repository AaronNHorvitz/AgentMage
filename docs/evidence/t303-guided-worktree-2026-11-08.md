# T-303/T-306: guided configured-endpoint worktree coding

Date: 2026-11-08. Tier: core. Immutable predecessor:
`7604c037e470c3d3f8e08eb5f019e4892becbca2`.
No row is added or closed. Independent exact-commit core review is required.

## Delivered behavior

The terminal's `code-worktree` command connects existing configured-endpoint
proposals, read-consented owned diff preparation, separately approved native
edits and project tests. Retryable failures lead to the original separately
consented repair proposal, another reviewed edit and another approved execution,
within the unchanged three-test-attempt ceiling. See the
[guide](../guides/coding-guided-worktree.md) and
[decision](../decisions/0164-guided-worktree-coding-client.md).

The first native test preparation precedes generation. The user supplies the
complete test definition; model output cannot select tests or change their
minimum count or parser. The command requires an initially clean validation
series and no pending edit. It pins settings, source selection, worktree root,
creation provenance, complete test template, attempt and exact prior failure.
A later preview substitution refuses before approval. Existing workspace-root
checks run around every read/write/test confirmation. Each API request is sent
once. Denial, native refusal, malformed observations, uncertain results and
lost replies stop the sequence. There is no automatic effect replay.

Returned command and validation receipts pass the original kernel verifiers
against the exact prepared command and test template. The original rerun
planner decides repair eligibility. A normalized full pass ends successfully;
malformed, zero-test or flaky results cannot prompt another model request.
The printed summary explicitly disclaims independent grading and current
workspace verification.

All consequential work remains inside the existing API/native owners. No model
transport, sandbox, grant issuer, runtime coordinator or recovery format is
added. The retained endpoint coordinator remains read-only. This guided client
has process-only state and relies on existing per-operation records; it cannot
resume an interrupted sequence. Existing individual terminal commands retain
their behavior. Original-checkout transfer is separate; approved Git, native
positive execution, autonomous endpoint edit/test composition, durable run
recovery, local-model measurements and both parent closures remain open.

## Verification

Before source edits, the complete ordinary affected-package baseline ran through
the shared build reservation: **10 targets, 150 passed, zero failed, ignored or
filtered**. The first focused executable tests passed three methods; a complete
four-method focused run also passed. Those used the actual terminal executable
against a recording synthetic HTTP peer, with synthetic proposal/write/command
observations. No model, native worker or project test process was executed.

The first full gate, including the fifth binding method, reported **10 targets,
154 passed, one failed, zero ignored or filtered**. The new pending-edit case
expected refusal before reading validation state; the implementation refused
after that read. Production code now refuses the pending edit immediately.
The assertion remains unchanged. Raw failure and source hashes are retained;
no existing assertion or threshold was weakened.

Five executable methods cover first-pass success, assertion failure then repair,
compilation failure followed by another failure and repair, three failures at
the ceiling, every consent position, piped consent, malformed/duplicate
definitions, native preflight refusal, lost edit/test replies, altered receipts,
preview substitution, changed settings/selection, existing series/pending edits,
wrong root/command/template, creation drift and stale failure disclosure.
Separate nonretryable results cover malformed, zero-test and flaky observations.
Tests compare every observed route, explicit decision payloads, preserved test
definitions and distinct repair receipts. All fixture source remains unchanged:
the peer records requests and returns synthetic effects.

The final ordinary affected-package gate is **PASS: 10 targets, 155 passed,
zero failed, ignored or filtered**. All 150 baseline passing identities remain,
plus exactly the five new methods. Ledger table rows are unchanged. Formatting,
strict Clippy, current Markdown checks, retained-review format, whitespace and
privacy checks pass. Seven gate commands completed in **107.02 seconds** of
wall time including individual build reservations. All four Rust input hashes
stayed frozen throughout; no source changed after this gate. Final documents
and the retained review receive separate lightweight checks. Raw logs, the
initial failure, source freezes and identity comparison remain private.

This is a passing scoped API/client gate, not a whole-workspace pass. The
unchanged deferred host/Linux gates under T-301.1 were not rerun or waived.
No successful native Git creation, native command execution or real-model
coding campaign is established by these synthetic client tests.

## Independent predecessor acceptance

The strict batch-bound fix's [independent review](../reviews/7604c037e470-local-coding-atomic-edits.txt)
arrived during this session: **Verdict: PASS** for its exact immutable commit,
with no high or medium finding. The report is retained unchanged. Its low
committed-coverage finding remains backlog in these same parent rows. The
reviewer's Markdown lint exit 127 remains recorded as a failure; the report's
PASS accepts only that predecessor increment and closes neither parent.

No evaluation inputs, hidden/held-out assertions, scorer, thresholds, model/GPU
work, dependency, SBOM, status model, scoreboard, release or push changes.

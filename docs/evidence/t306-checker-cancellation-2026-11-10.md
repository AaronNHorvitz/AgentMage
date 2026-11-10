# T-303/T-306: cancellation inside the installed reviewer

Status: independent core review required. T-303/T-306 remain open without new
rows or sub-rows. Native admission and the product commit route remain open.

The installed original-owner checker now receives a borrowed passive
`EffectCancellationObservation` through its original evidence. The view wraps
the caller's existing cancellation scope, with no replacement token. It offers
no cancellation mutation, token creation, execution or disclosure authority.
An installed checker must stop as unavailable on observation errors and remains
responsible for bounded work, deadlines and cleanup. The synchronous callback
is still not forcibly preempted.

One new regression runs a synthetic checker worker on a real scoped thread.
Another thread cancels the original caller only after that worker observes a
live scope. The worker receives only the passive view and observes the exact
original signal. All three callback outcomes are covered: success followed by
cancellation refusal, rejection and unavailable. The latter two retain their
existing failure-code precedence. Every case stops after the two pre-check Git
observations; no post-check read or checked candidate is released. The original
owner status, validation, both source files and absent candidate index are
checked after refusal. Existing successful-checker coverage also requires a
live uncancelled observation. Creation, Git and validation provenance in these
fixtures remain synthetic; this is no independent verdict or native workflow.

## Validation

The initial focused selection passes all nine checker methods. A temporary
mutation replaces the original view with a fresh detached token; the new worker
regression fails with the expected detached-cancellation diagnostic. The exact
source is restored before final verification. The mutation log is retained.

The first complete affected run reports 1,438 passes, the exact inherited
72 failures and 64 ignores over 26 unfiltered targets. All 35 direct methods
in the two changed sources pass, including the new method. Formatting, Clippy
and light checks pass. This run is retained. Subsequent source inspection finds
a possible race in the new test: polling can coincide with the canceller's
token lock, which must correctly fail closed as unavailable. The final test
uses bounded channel handshakes to observe before cancellation and after its
locks are released. No retry of an unavailable observation is introduced.
The detached-token mutation is repeated against this final test and is caught.

The final complete affected gate remains **FAIL**: **1,438 passed, 72 failed,
64 ignored**, zero measured or filtered, across **26 unfiltered targets**. All
failed and ignored identities equal the exact parent baseline. The new method
accounts for the single additional Linux pass. All **35** direct methods in the
two changed sources pass. Formatting, Clippy with warnings denied and all final
light checks pass. The two final Rust source hashes remain unchanged throughout
the final full run and afterward. The earlier full run binds its own pre-handshake
test snapshot and is not evidence for the final test bytes.

The final run takes **241.08 seconds**, including reservations:
formatting 1.35, Clippy 8.97 and Rust tests 229.34 seconds.
No timeout occurs. Final raw log SHA-256:
`593745f0ccbd0c89ae1c7dbef5cc7e7423ad3adce0a4741c58677fd9cf3def48`.
The exact failure/ignore comparison exits zero as data comparison only, never
gate PASS. Final documentation follows source freeze and is checked separately
by the final light pass. No native workflow or model result is claimed.
Private reproduction material is retained under
`/lane/state/tmp/t306-checker-cancellation`. The runner reserves heavy commands
internally; never outer-wrap it. Source hashes are fixed before the full run.

The parent commit `7f618ee04ad4c65c631cae9299bb9aea8402d981` starts
under independent review and receives independent **Verdict: PASS** during the
final full run, with no high or medium finding. The
[exact report](../reviews/7f618ee04ad4-local-coding-read-cancellation.txt) is
retained unchanged after source freeze. Its three lows concern candidate/commit
token-forwarding coverage, original collector wiring coverage and post-return
test precision; these and the carried lows remain same-row backlog. This
increment does not claim to address those three findings. Final light checks
cover the report and documentation. Preserve all ancestry. No row closes.
Native admission, trusted checker/artifact integration,
complete original-owner approval composition, repository/signer recovery,
model measurements and approved push remain open. No evaluator, hidden or
held-out test, prompt, threshold, dependency, SBOM, lifecycle, scoreboard,
GPU, host key, model download, push, release or readiness change.

# T-303/T-306: trusted clock at signed-commit dispatch

Status: independent core review required. Both parent rows remain open; no row
or sub-row is added. Existing ancestry is immutable.

The dormant signed-commit driver now takes the existing trusted host clock port.
It samples that clock at construction and again at actual effect dispatch, then
checks complete packet admission, exact call and held target, manual approval
expiry and the consumed grant's original lifetime. Clock failure, rollback before
construction, a time before grant consumption, or either exclusive expiry refuses
without invoking the executor. Cancellation retains its separate refusal.
Post-effect reconciliation uses the checked launch time; this does not extend
any permission or establish authority for a later operation.

The change addresses the construction-time clock and dispatch-test gap in low
L-1 of the retained complete-packet review. A test-only composition reuses the
actual issuer, policy engine and transaction coordinator to consume exact
GitCommit grants. It never constructs an EffectAuthorization directly. The held
object, consent and executor results are synthetic, so these observations do not
establish native admission, a trusted approval channel, signing or a coding score.

Seven focused preparation/dispatch methods pass, including three new methods.
Positive controls dispatch exactly once and reconcile the synthetic result;
expiry, unavailable clock, rollback, pre-consumption time, packet integrity and
material-finding drift, changed bindings, mismatched arguments and cancellation
refuse without invoking the executor. Separate approval and grant lifetime cases
prevent one expiry check from masking the other. Test-only retained-field mutation
reaches actual dispatch and cannot be satisfied by construction-time checks alone.

## Validation

The focused run passes all seven preparation/dispatch methods: 13.52 seconds
compilation and 0.21 seconds testing, after the shared reservation. Removing the
actual dispatch packet recheck makes the new drift test fail. Removing consumed
grant lifetime verification makes the new timing test fail. Both mutations were
restored before final source freeze; the subsequent complete run uses the
unmutated sources. Raw mutation logs and their hashes are retained privately.

The complete affected gate remains **FAIL**: **3,079 passed, 72 failed,
72 ignored**, zero measured or filtered, across **46 unfiltered targets** of the
kernel, Linux adapter, host and local API. Every per-target failure and ignore
identity matches the preceding complete baseline exactly. The parent source-pair
increment adds three Linux passes, and this increment adds three kernel passes.
No affected target is omitted. All three new methods and 31 existing methods in
changed test sources pass, as do formatting and Clippy with warnings denied.
All three source hashes remain fixed, with no later source edit.

The full run takes 446.85 seconds including shared reservations: formatting
2.51, Clippy 28.57 and Rust tests 414.20 seconds. No timeout occurs. The raw log
SHA-256 is
`f5cded5d39e26d7894ebcc2364dfde3a6dfc1d933c73f4ef44016349102c7b19`.
The comparison checker exits zero; that is a data comparison, never a gate PASS.
Commands, exact failure identities, raw logs, source freeze and guard-removal
observations are retained in `/lane/state/tmp/t306-dispatch-clock`. The full
runner reserves heavy commands internally; never outer-wrap it in build-slot.

Full original-owner approval composition, trusted independent checker and
artifact integration, native admission, durable recovery, model measurements and
approved push remain open. Low L-2 on public raw post-effect reconciliation and
earlier same-row lows remain backlog. No API, MCP or terminal commit route is
enabled. No evaluator, hidden/held-out task, scorer, prompt, threshold, dependency,
SBOM, lifecycle or scoreboard changes. No GPU, host key, download, push, release
or readiness claim.

The preceding source-pair commit received independent **Verdict: PASS** during
this run, with no high or medium finding. Its exact report is retained unchanged.
Its low L-1 on explicit existing-empty-blob source-pair coverage joins same-row
backlog; neither parent row closes. Only documentation and this retained report
were added after the final source freeze.

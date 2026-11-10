# T-306: Production signer capture and cleanup diagnostics

This same-row repair addresses low L1 and L2 in the retained
[signer-capture review](../reviews/69ead8457746-local-coding-signer-capture.txt).
The immutable base is `53a86bfc297a0a951c835fc6972bd07b09d1bcfe`, whose
independent review is still in flight at session start. Reviewed ancestry stays
unchanged. Its independent review arrives during final-gate execution with
**Verdict: PASS** and no high or medium finding, and is retained
[byte-identically](../reviews/53a86bfc297a-local-coding-exact-signer.txt).
Low L1 still requires native `run_plan` signatures; this repair covers the
production-runner portion only. Low L2 fixture-agent teardown assertions and
L3 usable signing-key inspection remain same-row backlog, alongside retained
child availability and the earlier candidate scratch/descriptor lows. No new
row or sub-row is created; T-303/T-306 remain open.

The process owner now distinguishes a retained signer child from an ordinary
preflight failure. Both production inspection and signing callers preserve
`linux.local_commit.cleanup.uncertain`. Signed creation and verification map it
to `linux.git.commit.signer_cleanup_uncertain`, with failed execution, no commit
object and unverified cleanup. Other failures retain their existing redacted
classes. No child output, key material or keyring path enters these diagnostics.
The immediate-child owner still refuses a retained, busy or poisoned slot;
there is no new spawn, retry or implicit retained-child recovery.

The existing disposable-signature regression now calls the production
`SignedGitRunner` for signed creation and cryptographic verification, rather
than constructing independent commands and collecting them with blocking output.
Listing and public-key export use the production `run_gpg` capture helper with
one inspection deadline. Both Git object formats, exact primary/subkey choices,
the two unpinned controls, all twelve exact-key message cases, wrong-key refusal
and unchanged fixture branch assertions remain intact. The private runner holds
the already selected executable path; its product constructor still requires
the verified Git artifact. Tests construct synthetic private runner and signer
fields. This proves production capture wiring, not artifact admission, sandbox
admission, exact approval composition or an executable native workflow.

The component fixtures and the existing native signer fixture serialize their
shared production capture owner. This test-only mutex cannot change product
busy-owner refusal. A native fixture assertion failure may poison the fixture
mutex; later fixtures recover that test mutex only. They never recover, clear or
bypass a poisoned or retained production owner. All native assertions remain.

Two new regression methods cover caller boundaries and receipt diagnostics.
Actual disposable child processes exercise both production callers with complete
output, nonzero exit, overflow of either stream and deadline expiry. A readiness
file establishes child startup before live cancellation of the signing caller.
A subsequent real inspection call proves the owner is reusable after these
reaped failures. The existing retained-child test now asserts the exact cleanup
error and still verifies that the original child stays retained. Diagnostic
mapping tests cover both caller fallbacks and both signing stages, preserving
failed outcome, absent commit object and unverified cleanup.

The focused pre-serialization run passes all seventeen selected methods, with
475 other methods filtered and no failures or ignores (2.97 seconds compile,
3.48 seconds tests). The first complete affected run has 1,358 passes, the exact
same 72 inherited failures and 64 existing ignores across 26 unfiltered targets;
formatting, Clippy and all eleven named affected methods pass. It takes 237.88
seconds including reservations, with unchanged source hashes. That run is
superseded by the final test-only native-fixture serialization correction.
The final complete affected gate remains **FAIL**: 1,358 Rust passes, the exact
same 72 inherited failure identities and 64 existing ignores across 26 unfiltered
targets, with zero measured or filtered tests. Both new methods, nine named
controls, formatting and Clippy pass. Both runtime source hashes remain fixed.
It takes 227.25 seconds including reservations:
formatting 2.26 seconds, Clippy 9.37 seconds and Rust 214.11 seconds. No command
times out. The raw log SHA-256 is
`0e9e4a05aac45743f050b57682e195c7ecd7e5088fb8ef2c3085b17183ff25db`.
Exact per-target comparison returns zero as data only, never a passing full gate.
Final whitespace, documentary lint, public-content, unchanged task-table and
byte-identical retained-review checks pass after the evidence is completed.

External signer confinement and whole-unit descendant cleanup remain required.
The dormant adapter is not activated. Retained-child recovery, candidate and
reviewer/artifact authority, separately approved commit/push, durable recovery,
native workflow admission and model measurements remain open. No evaluation,
frozen task, hidden or held-out test, scorer, prompt, threshold, dependency,
supply-chain output, lifecycle, scoreboard or release change. No GPU, model
download or push. Independent core review is required; neither parent closes.

# T-303/T-306: packet-bound commit result reconciliation

Status: independent core review required. The full affected gate remains FAIL.
Both rows remain open without new rows or sub-rows. Existing ancestry stays
immutable.

Result reconciliation now requires `PreparedLocalCommit` instead of a raw plan
and manual receipt. It repeats complete packet integrity, material-finding,
plan-binding and approval checks using the checked launch time before producing
a terminal receipt. The existing driver passes its retained prepared value.
Both Linux test callers migrate to complete synthetic packet inputs; their
existing failed-effect, cleanup-uncertainty and signed-content assertions stay.
The severity match is exhaustive, so a future variant needs an explicit decision.

Two new kernel methods exercise retained packet/approval refusal and actual
mediated dispatch with 23 synthetic platform variants. The successful control
launches once and binds its receipt to the actual authority transaction and
attempt; failed or cancelled outcomes, missing commit observations, changed
preservation, signature, branch, index, remotes, execution surfaces, network,
cleanup or malformed result code yield no success receipt. The consumed grant
comes from the actual issuer/policy/coordinator fixture, never direct creation
of `EffectAuthorization`. Platform output and approval remain synthetic.

A new Linux source-pair method starts with an existing empty file before its
synthetic Git creation and actual native file edit through an internal held-target
fixture. The ordinary source selector still refuses empty content; this test
explicitly asserts that refusal and establishes no public empty-file edit path.
The installed test checker observes `Some(&[])` and the exact nonempty postimage. Replacing the immutable
base observation with absence refuses before calling the checker. The original
file remains empty and no candidate index is created. Git/process observations
and checker success are synthetic; this is no native workflow admission.

This batch addresses the retained complete-packet review's low L-2 raw receipt
entry, the earlier commit-finding review's exhaustive-severity low, and the
source-pair review's low L-1 explicit empty-base case, subject to independent
review. Other same-row lows remain backlog. Trusted checker/artifact integration,
full original-owner approval composition, native admission, durable recovery,
model measurements and approved push remain open.

## Validation

The focused kernel run passes all nine methods: 18.15 seconds compilation and
0.26 seconds testing, after waiting for the shared reservation. The first focused
Linux empty-base attempt fails at the public source selector's existing empty
content refusal. Production behavior stays unchanged. The corrected component
fixture explicitly asserts that refusal, then selects an internal held target
before the existing native edit/projection. Its retry passes: 2.16 seconds
compilation and 0.36 seconds testing. The migrated uncertainty-caller control
also passes: 0.03 seconds build and 0.01 seconds testing. Both Linux raw logs,
including the initial failure, are retained privately.

The complete affected gate remains **FAIL**: **3,082 passed, 72 failed,
72 ignored**, zero measured or filtered, across **46 unfiltered targets** of the
kernel, Linux adapter, host and local API. Every per-target failure and ignore
identity matches the exact parent baseline. This batch adds two kernel passes
and one Linux pass. No affected target is omitted. Of 92 methods in changed
sources, all three new methods and 87 existing controls pass. The other two are
the inherited native candidate/signing fixtures, which still fail at artifact
admission. The migrated native signing reconciliation path is therefore not
claimed exercised. Formatting and Clippy with warnings denied pass. All five
source hashes remain fixed, with no subsequent source edit.

The full run takes 471.13 seconds including shared reservations: formatting
2.61, Clippy 25.93 and Rust tests 440.73 seconds. No timeout occurs. The raw log
SHA-256 is
`67ab513b961f40d41d818d6ff3a7feddd9ab0cf705339fa605eca65b3e5beca4`.
The exact comparison exits zero; this is data comparison only, never gate PASS.
Reproduction scripts, complete failure identities, raw logs and source freeze
are retained privately in `/lane/state/tmp/t306-prepared-reconciliation`.
The full runner reserves heavy commands internally; never outer-wrap it.

The parent dispatch-clock commit received independent **Verdict: PASS**, with
no high or medium finding. Its exact report is retained unchanged. Its low L-1
isolated rollback-guard coverage and L-2 operation/held-scope dispatch coverage
remain same-row backlog. Neither parent row closes. Only documentation and the
retained report follow the final source freeze.

No product API, MCP or terminal commit route is enabled. No evaluator, hidden or
held-out task, scorer, prompt, threshold, dependency, SBOM, lifecycle or scoreboard
change. No GPU, host key, download, push, release or readiness claim.

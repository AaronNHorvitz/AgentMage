# T-306: Offline public-material commit signature verification

This non-closing T-303/T-306 increment implements
[Decision 0192](../decisions/0192-confine-local-commit-signature-verification.md).
The immutable base is `ee983557491646b712acfc9d624f740b665cbbb0`, whose
independent core review was in flight at session start. That review arrived
with **Verdict: PASS** and no high or medium finding during implementation and
is [retained unchanged](../reviews/ee9835574916-local-coding-signer-callers.txt).
Its low L1 native retained-child injection and L2 poisoned-owner diagnostic
remain same-row backlog with the earlier native-signature, fixture teardown,
usable-key inspection, availability and candidate-directory findings. No new
row or sub-row is created, and neither parent closes.

The dormant local-commit adapter now verifies its already checked raw commit
with a standalone GnuPG worker through the existing offline supervisor. The
direct signing runner refuses verification commands. Three sealed descriptors
carry the inspected binary public-key export, detached signature and exact
signed content. Verification checks the complete plan and pinned signer,
including the retained public-key digest. Both supported signature-header
formats reconstruct the original signed bytes, preserving message endings.

The worker projects no repository, external private keyring, host home, agent
socket or user configuration. Its private temporary home, fixed GnuPG arguments,
disabled option files, default keyrings, agent startup and key retrieval keep
verification independent of signer state. The existing socket-denying seccomp
policy is unchanged. Installed executables and root-owned ancestors are held
and revalidated. A new closed unit prefix admits exactly three public-material
projections. The original commit-session deadline, four-MiB output limits,
resource bounds and whole-unit cleanup remain with the existing supervisor.

Only complete, consistently hashed output with successful supervision and exit,
empty stdout and exactly one valid status for the complete pinned signing
fingerprint permits the next task-branch step. Cancellation, expiry, malformed
or changed inputs, partial output, a failed signature or uncertain cleanup
cannot establish success. Failure returns before the task-branch update and
preserves failed execution and unverified cleanup in the commit result.

Six new component methods cover exact reconstruction, immutable public payloads,
pre-dispatch refusals, supervised-result validation, launch construction and
closed-owner unit/cardinality checks. Synthetic results establish these
contracts only. The disposable installed-Git signature regression still checks
both formats, exact primary/subkey selection, three message cases, unpinned
controls, wrong-key refusal and an unchanged branch. Its verification now uses
the worker's reconstruction and fixed public-only GnuPG arguments directly;
it also tests actual altered-message and invalid-public-key failures and checks
that the verification home has no agent socket or private-key directory. This
is cryptographic component evidence, not native supervisor admission. Signing
still uses production direct capture. Existing native admission assertions and
the affected full test targets remain unfiltered.

The focused component run passes six methods with 492 filtered, no failures
or ignores (4.99 seconds compile, 0.00 seconds tests). The real-signature run
passes its one selected method with 497 filtered, no failures or ignores
(0.08 seconds compile, 0.67 seconds tests). Neither is an unfiltered full gate.

The first complete affected run retains 1,364 Rust passes, the exact same 72
inherited failure identities and 64 existing ignores across 26 unfiltered
targets, but additionally fails Clippy because direct capture no longer has a
production stderr consumer. All five source hashes stay fixed during that run.
Only after it finishes, an explicit consuming method discards direct-capture
diagnostics when signing and inspection take stdout. No lint suppression,
ignore, trust change or capture-bound change is added. That first run is
superseded and retained; it takes 410.52 seconds including reservations, with
raw log SHA-256
`20ec13a7243cbbdad75fc88aef9e9d90c705477220bed6d6435022e358c2a88b`.

The final complete affected gate remains **FAIL**: 1,364 Rust passes, the exact
same 72 inherited failure identities and 64 existing ignores across 26 unfiltered
targets, with zero measured or filtered tests. All six new methods, twelve named
controls, formatting and Clippy pass; all five source hashes remain fixed.
It takes 247.80 seconds including reservations: formatting 1.30 seconds,
Clippy 13.93 seconds and Rust 231.00 seconds. No command times out. The raw
log SHA-256 is
`1888f378b8973622a031ec7926ee118a0de956ad0d0c0a5ca086f0c369127567`.
Exact per-target comparison returns zero as data only, never a passing full gate.
Final whitespace, documentary lint, public-content, unchanged task-table and
byte-identical retained-review checks pass after this evidence is completed.

External signer inspection and signed creation remain direct and dormant. Their
confinement and descendant cleanup, candidate/reviewer/artifact authority,
separately approved commit/push, durable recovery, native workflow admission
and model measurements remain open. No evaluation, frozen task, hidden or
held-out data, scorer, prompt, threshold, dependency, supply-chain output,
lifecycle, scoreboard or release change. No GPU, model download or push.
Independent core review is required; T-303/T-306 remain open.

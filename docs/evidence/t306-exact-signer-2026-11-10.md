# T-306: Exact signing key and object-format verification

This same-row repair addresses low L1 and L2 in the retained
[non-signing commit I/O review](../reviews/6033f03afb9e-local-coding-commit-io.txt).
Work begins on immutable base `69ead845774654de33f76e74a56824e5b3a3d467`,
while its independent signer-capture review remains in flight. No reviewed
ancestry changes. Its review arrives during focused-test reservation waiting
with **Verdict: PASS** and no high or medium finding, and is retained
[byte-identically](../reviews/69ead8457746-local-coding-signer-capture.txt).
Low L1 production capture wiring coverage, L2 cleanup diagnostics and L3 retained
child availability remain same-row backlog, along with the earlier candidate
scratch/descriptor lows. Retaining the report does not change runtime sources.
T-303/T-306 remain open; no row or sub-row is added or closed.

Signed commit arguments now request the exact pinned fingerprint, with an
explicit exact-key selector. A primary-key pin cannot silently select a newer
signing subkey. Verification still requires the actual signing fingerprint in
the signature status record; accepting a primary fingerprint reported alongside
a different signing key is not allowed. An unusable selected key fails without
fallback. Raw content verification requires matching parent and tree object
lengths and the corresponding singleton signature header: `gpgsig` for SHA-1,
`gpgsig-sha256` for SHA-256. Wrong, duplicate and mixed-format headers refuse.

Two new regression methods cover object-format rejection and actual disposable
Git/GnuPG signing. Each real fixture has a primary signing key and a newer signing
subkey. An unpinned control demonstrates that default key selection chooses the
subkey and fails the primary fingerprint check. Production argument construction
then signs with each exact selection in each object format, with three message
newline cases. Actual raw content and cryptographic status must verify; the other
key must refuse. The fixture branch must stay at its original parent. Temporary
keys remain outside the repository and are removed with their fixture; fixture
cleanup stops its own keyring agent. These are installed-tool component tests
using a synthetic plan, not product trust admission, approvals, confinement or
a native end-to-end workflow. Existing native tests and trust checks stay intact.

The focused run passes all fifteen selected methods with no failure or ignore
(475 other methods filtered). The complete fixed-source affected gate remains
**FAIL**: 1,356 Rust passes, the exact same 72 inherited failure identities and
64 existing ignores across 26 unfiltered targets, with zero measured or filtered
tests. Both new methods and six affected controls pass, as do formatting and
Clippy. The single source hash remains unchanged. The run takes
276.76 seconds including reservations: formatting 1.46 seconds, Clippy 11.04
seconds and Rust 262.42 seconds. No command times out. The raw log SHA-256 is
`f63ef16a3f2be48fd545f0007120c498b946210fa92443b2a511e387a08f0381`.
Exact per-target comparison returns zero as data only, never a passing full gate.
Final documentary lint, whitespace, public-content checks, unchanged task-table
comparison and byte-identical retained-review checks pass after verification.
The source-level repair has twelve actual exact-key signature checks across two
formats, two selected keys and three message cases, plus two unpinned controls;
these do not establish native admission or a measured agent score.

The adapter remains dormant. External signer confinement and whole-unit cleanup,
candidate authority, independent reviewer/artifact provenance, exact approvals,
separately approved commit/push, durable recovery, native workflow admission and
model measurements remain open. No model/GPU work, evaluation task, frozen or
hidden/held-out test, scorer, prompt, threshold, dependency, supply-chain output,
lifecycle, scoreboard or release changes. Independent core review is required;
neither parent closes.

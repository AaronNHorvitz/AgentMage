# T-303/T-306: batch public-inspection findings repair

Status: complete affected gate remains **FAIL**; independent core review
required. Neither parent closes and no row or sub-row is added.

The retained [independent review](../reviews/e00e172d7170-local-coding-confined-inspection.txt)
has **Verdict: PASS** for exact commit
`e00e172d717066000767cb697194aad1766ba368`, with three low findings and no high
or medium finding. This batch addresses all three within Decision 0193.
The base [control review](../reviews/4bcf4fb34053-local-coding-inspection-control.txt)
arrived during validation with **Verdict: PASS** for exact commit
`4bcf4fb34053afe00accd65cfce99b374d0c9c44`, with no high or medium finding.
Its low L1 checkpoint coverage and L2 synchronous admission-hash latency remain
same-row backlog. No published ancestry is rewritten.

Capture now refuses any `public-keys.d` or `common.conf` directory entry through
the held keyring descriptor. Even an empty configuration, dangling symlink,
special file or configuration unrelated to keybox-daemon use is refused. This
conservative compatibility limitation avoids parsing common options and prevents
using a stale legacy database left after daemon-store migration. The same
check runs before database selection and before and after sealing. Capture is
still a bounded snapshot, not an atomic transaction against concurrent changes
by the same user. No configuration content is read or projected.

A designated revoker record may precede its primary key's fingerprint without
consuming the pending primary identity. Only the primary's own attached
fingerprint can qualify. A revoker cannot stand in for a missing primary
fingerprint or be interposed before a subkey fingerprint. The installed-GnuPG
public-only fixture now adds a real designated revoker and checks both primary
and subkey inspection over sealed keybox and exported public database snapshots.
The revoker fingerprint alone never qualifies.

A test-only sandbox-error constructor permits direct exercise of the production
dispatch error mapper and the result validator. Both preserve CleanupUncertain;
an ordinary manifest failure keeps the ArtifactDenied fallback. The production
constructor and sandbox authority are unchanged. Synthetic error injection is
component coverage, not evidence of native worker cleanup.

Three new regression methods cover daemon indicators and their appearance
during capture, designated-revoker identity separation, and both sandbox-error
paths. The existing real-GnuPG component test is strengthened. These tests do
not establish native executable admission, sandbox execution or an approved
commit workflow. Confined usable signing, candidate/reviewer/artifact authority,
approval composition, approved commit/push, durable recovery and model scores
remain open. Existing same-row backlog remains.

No evaluation, frozen or hidden input, held-out set, scorer, prompt, threshold,
dependency, SBOM, lifecycle, scoreboard or release state changes. No private-key
capture, host agent access, socket-filter change, GPU use, download or push.

## Validation

Focused inspection validation passes: 13 tests, zero failures or ignores,
505 filtered tests. Compilation took 4.08 seconds and tests 0.08 seconds after
the shared reservation. The real designated-revoker fixture passes. This is
filtered component validation; the complete affected gate is recorded below.

The complete affected fast gate remains **FAIL**: **1,384 passed, 72 failed,
64 ignored**, zero measured or filtered, over 26 unfiltered targets. Every
per-target failure and ignore identity matches the frozen inspection-control
baseline; only three passing methods are added. All three new methods and
21 named controls pass, including the strengthened real-GnuPG designated-revoker
fixture. Formatting and Clippy pass. All three changed source hashes remain
fixed throughout the run; there is no source edit after it.

Elapsed time including shared reservations is 302.61 seconds: formatting 25.41,
Clippy 9.62 and the full Rust suite 266.06 seconds. No timeout occurred. The raw
log SHA-256 is
`12783a025d61f26b97cc9da9c51a0e4bbcf26cb8e8f72f2ea9f2690145acfa7b`.
The private bundle is `/lane/state/tmp/t306-inspection-findings`; `run.py`
reserves heavy commands internally and must not be outer-wrapped. Its
`check.py` exit zero means an exact per-target **data comparison only**, never
a passing gate. Final documentation, public-content, unchanged-row and staged
blob checks accompany the commit. Both retained review reports are copied
byte for byte. An initial final-document check rejected an extra blank line;
it was removed without changing source or rules. T-303 and T-306 remain open.

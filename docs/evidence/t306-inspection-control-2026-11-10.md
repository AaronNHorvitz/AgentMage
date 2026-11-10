# T-303/T-306: caller-controlled public identity inspection

Status: complete affected gate remains **FAIL**; independent core review required.
Neither parent row closes. No new row or sub-row is added.

Immutable base `e00e172d717066000767cb697194aad1766ba368` is under independent
review. No reviewed or published ancestry is rewritten. This repair remains in
the existing public-inspection scope under Decision 0193.

`LinuxOpenPgpSigner::inspect_cancellable` accepts the caller's cancellation token.
The legacy constructor delegates with an internal token. The same thirty-second
budget covers executable hashing, public snapshot capture and both offline
workers. Checks precede path consultation, run between 64-KiB executable and
public-database reads, surround public snapshot sealing and precede returning a
pin. Cancellation or expiry returns no signer. Individual filesystem calls and
bounded hashing/sealing operations remain synchronous; the checks do not make
these individual operations interruptible.

Public key records now require unsigned decimal creation timestamps; expiry
may be empty or decimal, with zero meaning no expiry. The selected identity and
its primary must have been created by the current time and have a coherent
expiry. An unusable primary cannot qualify a subkey. Exact fingerprints, signing capability,
revocation/expiry rules, public-only sealed projections, root-owned executable
admission, the socket-denying filter and whole-unit cleanup remain required.
There is no new private-key access or direct inspection fallback.

Three new regression methods cover cancellation before even consulting caller
paths, cancellation at each public-capture control checkpoint including after
sealing, and malformed/future primary/subkey dates. Capture tests also compare
all bytes and the digest after multiple bounded reads. Existing real-GnuPG
public-only component tests and the disposable signature matrix remain controls;
these are not native executable/sandbox admission or an approved commit workflow.

Usable private signing and confined signed creation remain required before
activation. Candidate/reviewer/artifact authority, complete approval composition,
approved commit/push, durable recovery, native/model measurements and both parent
closures remain open. Evaluation tasks, frozen/hidden/held-out inputs, scorer,
prompts, thresholds, dependencies, SBOM, lifecycle, scoreboard and release state
are unchanged. No GPU use, download or push is performed.

## Validation

Focused inspection validation passes: ten tests, zero failures or ignores,
505 filtered tests. Compilation took 5.50 seconds and test execution 0.06 seconds
after the shared reservation. The real-GnuPG public-only method also passes.
This is filtered component validation; the new public-API cancellation method
is included in the complete affected gate below.

The complete affected fast gate remains **FAIL**: **1,381 passed, 72 failed,
64 ignored**, zero measured or filtered, over 26 unfiltered targets. Per-target
counts, every failure identity and every ignore identity match the immutable
inspection baseline, with three additional passes. All three new methods and
18 named controls pass, including the public-API cancellation method and the
existing disposable signature matrix. Formatting and Clippy pass. All three
changed source hashes remain fixed throughout the run; no source edit follows.

Elapsed time including shared reservations is 290.72 seconds: formatting 41.61,
Clippy 11.37 and the full Rust suite 235.95 seconds. No timeout occurred. The raw
log SHA-256 is
`a18e18b4b42b528445f1f9a07de83d7944f9dab9702f78cdff4e8180f472a7a9`.
The private bundle is `/lane/state/tmp/t306-inspection-control`; `run.py` reserves
heavy commands internally and must not be outer-wrapped. Its `check.py` exit
zero confirms an exact per-target **data comparison only**, never a passing gate.
Final documentation, public-content, unchanged-row and staged-blob checks
accompany the commit. Native admission and the complete workflow remain open.

# T-303/T-306: strict signing escapes and a valid selection fixture

Status: independent core review required. Neither parent closes and no row or
sub-row is added. The preceding storage commit remains immutable and under
independent review.

The retained [private-signing review](../reviews/907724989c81-local-coding-private-signing.txt)
has **Verdict: PASS** for exact commit
`907724989c8183aa96ed154f0812be3f78e18f84`, with no high or medium finding.
This same-row batch addresses its low L-1 and L-2 together under the existing
Decision 0194 protocol and selection boundaries. The review is unchanged.

L-1: the integer parser accepted a leading plus sign in percent escapes. The
protocol decoder now requires both encoded bytes to be ASCII hex digits before
converting the value. The existing phase-order test now embeds `%+1` inside
otherwise valid signature data, so refusal cannot come from the outer prefix
check. A new method checks all 256 byte values in both hex cases, all 65,052
non-hex pairs, incomplete escapes, escapes split across data records and valid
data split across records. These tests exercise the untrusted transport bytes;
they do not treat a signature expression as cryptographically verified.
The earlier evidence's malformed-escape claim is explicitly corrected without
changing any historical outcome or run count.

L-2: missing and malformed keygrip tests now use a real private directory and
selected-key file. A successful selected-file capture first proves the fixture
can be read and sealed. The test separately verifies the directory, pinned
report and public-key digest, then requires the artifact-denial class for each
invalid selection. This strengthens selection refusal coverage; it does not
establish native executable admission or a successful `sign_hash` call.

Native execution remains unavailable in this lane: installed executable owners
appear unmapped to the sandbox, so the required root-owned artifact admission
refuses them. No alternate owner, digest-only exception, direct fallback or
permission change is introduced. Native usable signing, reviewer/artifact
provenance, complete approved commit/push, durable recovery and measured coding
acceptance remain open, as do the earlier same-row backlog findings. No model,
evaluation, hidden/held-out input, scorer, prompt, threshold, dependency, SBOM,
lifecycle, scoreboard, release or owner-readiness change is made.

## Validation

The complete affected fast gate remains **FAIL**: **1,406 passed, 72 failed,
64 ignored**, zero measured or filtered, across 26 unfiltered targets. Every
per-target failure and ignore identity matches the previous storage baseline;
only the new Linux parser method adds a pass. That method and all 51 named
controls pass. Formatting and Clippy pass for all three affected crates. Both
source hashes remain fixed throughout validation; no source edit follows it.

The run takes 277.83 seconds including reservations: formatting 33.49, Clippy
9.82 and Rust tests 233.04 seconds. No timeout occurs. The raw log SHA-256 is
`620ec0cf131df94774b7e75d0ffc5459c6c0c9645b73e061ff2aad5b27cc7dbc`.
The private reproduction bundle is `/lane/state/tmp/t306-signing-escapes`.
Its `run.py` reserves heavy commands internally and must not be outer-wrapped.
Its `check.py` exit zero compares exact per-target data; it never means the
full gate passed. Documentation, public-content, unchanged-row and staged-blob
checks accompany this commit.

The earlier focused signer run passes nine methods, with zero failures or
ignores and 531 filtered library tests (plus 18 filtered integration methods).
Compilation takes 7.02 seconds and library tests take 0.03 seconds after the
reservation. This filtered run does not replace the full unfiltered result.
The private artifact observation records UID 65534 for all six installed tools
where production admission requires UID 0; it is a filesystem observation,
not native execution or a model score.

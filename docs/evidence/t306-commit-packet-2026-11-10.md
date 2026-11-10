# T-303/T-306: dormant RSA commit signature assembly

Status: complete affected gate remains **FAIL**; independent core review required. Neither parent
closes, and no row or sub-row is added. The private-signing ancestor remains
immutable. Its independent review arrived after the implementation commit:
the retained [report](../reviews/907724989c81-local-coding-private-signing.txt)
has **Verdict: PASS** for exact commit
`907724989c8183aa96ed154f0812be3f78e18f84`, with no high or medium finding.
Low L-1 (signed percent escape) and L-2 (invalid-keygrip fixture strength) are
recorded as same-row backlog for the next increment. The report is retained
byte for byte; this increment does not claim those lows are fixed.

[Decision 0195](../decisions/0195-dormant-rsa-commit-signature-assembly.md)
adds exact unsigned content construction, bounded RSA/v4 signature packet and
armor assembly, and a dormant composition requiring independent verification
before returning raw commit bytes. There is no product caller or object write.
Approved message bytes are preserved without normalization. Both Git object
formats use their exact signature header. Invalid plans, mixed object formats,
out-of-range timestamps, changed signer/public material and unsupported or
malformed signature expressions refuse the operation. The full OpenPGP digest
includes fixed signature metadata and the v4 trailer. An advisory issuer ID
never replaces full-fingerprint verification.

Production composition calls the existing socket-free signer and public-only
offline verifier under one deadline. No host-agent projection, global filter
change, direct fallback, approval or publication authority is added. Cancellation
is checked before work, after signing and after verification. A verification
failure or uncertain cleanup returns no raw commit, with uncertainty preserved.
The original wire-format implementation uses the protocol fields documented in
[RFC 4880](https://www.rfc-editor.org/rfc/rfc4880.html), published by the IETF
Trust in 2007; no reference code or specification prose is copied.

Disposable installed-tool tests exercise RSA primary/subkey selection, the
production foreground protocol/capture and packet assembly, and real independent
verification with the existing verifier options and sealed public material.
Changed content and corrupted signature values must fail cryptographic checks;
a different full fingerprint cannot satisfy identity. These fixtures use
synthetic artifact/plan authority and direct component processes. They do not
establish native sandbox admission or a complete approved workflow.

The existing signed-creation adapter remains direct and dormant. Native
admission, confined object write, complete approval/preservation composition,
approved commit/push, durable recovery and model measurements remain open. No
evaluation, frozen or hidden input, held-out set, scorer, prompt, threshold,
dependency, SBOM, lifecycle, scoreboard, release or owner-readiness change.
No host private keys, GPU, model download or push is used.

## Validation

The complete affected fast gate remains **FAIL**: **1,399 passed, 72 failed,
64 ignored**, zero measured or filtered, over 26 unfiltered targets. Every
per-target failed and ignored identity matches the frozen private-signing
baseline; the only count change is six additional passes in the Linux library.
All six new methods and 33 named controls pass. Formatting and Clippy pass for
all three affected crates. All four changed source hashes remain fixed during
the run; no source edit follows it.

Elapsed time including reservations is 232.94 seconds: formatting 1.30,
Clippy 9.52 and the complete Rust suite 220.58 seconds. No timeout occurs.
The raw log SHA-256 is
`b7806681bc9fbc4f42aab94cef520fcf4ec43cc463ffc72d6b13c10c6d6f954b`.
The private bundle is `/lane/state/tmp/t306-commit-packet`. Its `run.py`
reserves heavy commands internally and must not be outer-wrapped. Its
`check.py` exit zero is an exact per-target **data comparison only**, never
a passing full gate. Final documentation, public-content, unchanged-row and
staged-blob checks accompany the commit.

An earlier focused run passes all six methods, with zero failures or ignores
and 527 filtered tests (2.53 seconds compilation; 0.33 seconds tests). This does
not replace the complete unfiltered run above. Superseded focused failures are
retained: the fixture needed an explicit byte slice for the zeroizing buffer,
and the installed foreground agent emits unsigned RSA scalars without sign
padding. The parser now requires that observed bounded canonical form, including
high first bits, and refuses redundant leading zeros. No hidden tests or
evaluation threshold is changed. Real signatures verify independently for both
selected primary/subkey identities and both Git object formats. Changed message
bytes and corrupted signatures fail verification; a different fingerprint
fails the identity check. Separate installed-Git comparisons preserve missing
final newlines, trailing blank lines, Unicode and carriage returns exactly.
These remain disposable component processes, not native admitted workers.

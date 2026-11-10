# T-303/T-306: dormant socket-free private signing primitive

Status: complete affected gate remains **FAIL**; independent core review required.
Neither parent closes, and no row or sub-row is added.

The retained [inspection-findings review](../reviews/ac9621ddcf35-local-coding-inspection-findings.txt)
has **Verdict: PASS** for exact commit
`ac9621ddcf352487554b02c7262cc14eff8b91c3`, without any finding. The report is
retained unchanged. Earlier same-row backlog remains open.

[Decision 0194](../decisions/0194-socket-free-private-signing-primitive.md)
adds a dormant private signing primitive without changing any product route or
permission. Public inspection retains the exact fingerprint's keygrip from its
hash-bound listing. Only its selected private file is captured, through held
private directories, with no-follow/nonblocking opens, bounded cancellable
reads, strict types, ownership, modes and link counts, metadata/name checks and
revalidation before and after sealing. Temporary heap bytes are wiped. No
secret digest, private pathname or secret content is recorded in a report.
Capture is not atomic against same-user writes; memory locking and kernel
storage erasure are not promised.

The new closed supervisor owner requires two payloads: the sealed selected
file and sealed fixed commands. A foreground GnuPG agent uses a private home,
no options, no interactive pinentry, no smartcard daemon or external cache.
There is no host-agent socket, live keyring, repository or user configuration
projection. The existing global socket-denying policy is unchanged. Commands
reach stdin separately from the withheld filter. Installed artifacts are held
and revalidated; existing resource limits and whole-unit cleanup remain.

Success requires complete consistent bounded streams, successful supervision
and exit, and an exact phase-ordered response with data only in the signing
phase. Inquiries, errors, cancellation, expiry, malformed escaping, partial
output and cleanup uncertainty refuse the result. Stderr is diagnostic only.
Returned signature expressions are unverified and provide no object, approval
or publication authority. Packet/content construction, independent verification,
native artifact/sandbox admission and complete approval composition remain
open. The current signed-creation adapter stays direct and dormant.

Tests cover exact keygrip attachment, selected sealed capture and hostile paths,
control checkpoints and replacement, protocol ordering and binary escaping,
worker projections and fixed stdin, result integrity, cleanup uncertainty and
control refusal before capture and after dispatch. A disposable installed-agent
fixture signs using the production arguments, command construction and capture
caller without creating sockets. It is a direct component fixture, not the
native sandbox worker, an assembled OpenPGP signature or a complete workflow.

No evaluation, frozen or hidden input, held-out set, scorer, prompt, threshold,
dependency, SBOM, lifecycle, scoreboard, release or owner-readiness change.
No host private keys, GPU, model download or push is used.

## Validation

The complete affected fast gate remains **FAIL**: **1,393 passed, 72 failed,
64 ignored**, zero measured or filtered, over 26 unfiltered targets. Every
per-target failed and ignored identity matches the frozen inspection-findings
baseline; the only count change is nine additional passes in the Linux library.
All nine new methods and 24 named controls pass, including the disposable real
foreground-agent fixture, existing primary/subkey public-inspection fixture,
public-only verification and the unchanged socket-denying policy tests.
Formatting and Clippy pass for all three affected crates. All seven changed
source hashes remain fixed throughout the run; no source edit follows it.

Elapsed time including reservations is 252.10 seconds: formatting 1.40,
Clippy 9.72 and the complete Rust suite 239.40 seconds. No timeout occurs.
The raw log SHA-256 is
`23153a51bebec2418ab6d443908985d79084007c5ff603251f6300c1336582d7`.
The private bundle is `/lane/state/tmp/t306-private-signing`. Its `run.py`
reserves heavy commands internally and must not be outer-wrapped. Its
`check.py` exit zero is an exact per-target **data comparison only**, never
a passing full gate. Documentation, public-content, unchanged-row and staged
blob checks accompany the commit. The retained independent report is copied
byte for byte. Neither parent row closes.

An earlier focused pass covered seven signing/owner methods, zero failures or
ignores and 519 filtered tests; it preceded the final control test. Compilation
took 5.03 seconds and those tests 0.02 seconds after the reservation. It does
not replace the final unfiltered run above. The initial protocol probe showed
binary data may be unescaped; the parser accepts that data while rejecting
malformed percent escapes and carriage-return records. Only disposable keys
were used. The result remains an unverified signature expression.

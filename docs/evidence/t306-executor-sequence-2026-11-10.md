# T-303/T-306: executor sequence coverage

Status: complete affected gate remains **FAIL**; independent core review required. Neither parent closes and no row or
sub-row is added. The preceding escape repair remains immutable; its review arrived during validation.

The retained [storage review](../reviews/bf27deb3a2db-local-coding-confined-storage.txt)
has **Verdict: PASS** for exact commit
`bf27deb3a2dbfb97e6a374132041990caf022aec`, with no high or medium finding.
This same-row batch addresses low L-4: the post-preflight executor ordering
previously had no runnable test in this lane. The review remains unchanged. The retained
[escape repair review](../reviews/5d94bca89017-local-coding-signing-escapes.txt)
also has **Verdict: PASS**, for exact commit
`5d94bca8901722d69a6f61c0fc4a6153ced38815`, with no high or medium finding.
Its low L-1 cancellation/deadline fixture strength stays same-row backlog.

The existing sequence now lives in one module-private generic body. The only
production entry takes a concrete native session; its constructor is internal
to the local-commit module. The public adapter still requires the kernel launch
permit, exact preflight manifest, revalidated pinned signer and closed Git I/O
owner before entry. Native methods delegate to those same owners with the same
cancellation token and session deadline. No native trust rule, fallback, effect
authority, approval rule, result mapping or product route changes.

Four deterministic methods cover both Git object formats. The successful path
checks the complete call order and exact verified-value/object/readback flow,
then checks every success observation against its source. Every stage's failure
ends the sequence without retry and keeps failed outcome, unverified cleanup,
last-observed manifests and absent commit/signature/ref claims. Corrupt or
truncated readback stops before signature verification and compare-and-swap.
All six signer error classes, including uncertain cleanup, are injected at
initial signing and at verification after storage; none reaches a ref update.
Postflight failure remains uncertain even after a successful synthetic update.

Synthetic signer, verifier and storage results prove control flow only. They
do not prove cryptography, native execution, kernel approval or independent
review. Existing real-signature and installed-Git component controls remain.
Native executable admission is still unavailable in this lane. Authenticated
reviewer/artifact provenance, complete product approval composition, durable
recovery, model measurements and approved push stay open. Other retained low
findings stay same-row backlog. No model, evaluation, hidden/held-out input,
scorer, prompt, threshold, dependency, SBOM, lifecycle, scoreboard, release or
owner-readiness change is made. No host keys, GPU, download or push is used.

## Validation

The complete affected fast gate remains **FAIL**: **1,410 passed, 72 failed,
64 ignored**, zero measured or filtered, across 26 unfiltered targets. Every
per-target failure and ignore identity matches the preceding escape-repair
baseline; only four Linux library passes are added. All four new methods and
52 named controls pass. Formatting and Clippy pass for all three affected
crates. All three source hashes stay fixed; no source edit follows this run.

The run takes 231.06 seconds including reservations: formatting 1.50, Clippy
9.78 and Rust tests 218.19 seconds. No timeout occurs. The raw log SHA-256 is
`af10c9d96ebdac6565f98e48a0c885ba107a47eb3a43fa281cf2e62fba1133ca`.
The private reproduction bundle is `/lane/state/tmp/t306-executor-sequence`.
Its `run.py` reserves heavy commands internally and must not be outer-wrapped.
Its `check.py` exit zero compares exact per-target data; it never means the
full gate passed. The newly arrived escape-repair report was retained during
this run; final light checks cover both unchanged review reports and final
prose without further source edits. Public-content, unchanged-row and staged
blob checks accompany the commit.

The earlier focused run passes all four executor methods, with zero failures
or ignores and 540 filtered tests. Compilation takes 4.22 seconds and tests
0.01 seconds after the reservation. Formatting and this focused command each
waited for a shared reservation before the final full run. These waits do not
establish execution or a coding score. The focused run does not replace the
complete unfiltered validation above.

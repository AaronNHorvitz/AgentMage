# T-303/T-306: cancellation at commit stage boundaries

Status: independent core review required. T-303/T-306 remain open, with no new
row or sub-row. The preceding review-read-scope commit remains immutable.
Its [independent report](../reviews/0c859b28ff89-local-coding-review-read-scope.txt)
arrived during full validation and is retained unchanged: **Verdict: PASS** for
`0c859b28ff899a6608c070768f7e5d013a087884`, with no high or medium finding.
Its low L-1 worktree read-selection consistency remains same-row backlog
alongside the earlier lows. Neither parent closes.

The dormant native commit executor now checks cancellation before preservation
preflight and again after that collection returns. A cancelled preflight cannot
start collection or release its result to the signing and storage sequence.
The private sequencing interface also checks the original cancellation token
and commit I/O session deadline before every stage and after final preservation
collection. Even if a worker reports success just as control stops, the next
stage cannot start. A stop after compare-and-swap or postflight cannot become a
successful commit receipt. Failed results retain last-observed manifests with
unverified cleanup; they do not imply rollback, an unchanged branch or no effects.

Three new test methods exercise both object formats. Cancellation and expiry
are injected before the sequence and after every successful stage, including
the final observation. They check the exact completed prefix, retained stored
object and whether branch update already occurred, alongside every uncertain
result field. A separate preflight test refuses collection before cancellation,
rejects a successful collection cancelled through a propagated token, and
retains an active control. No sleep or native admission is needed by these
synthetic controls. Existing sequence failure and success tests remain intact.

These are checkpoints, not interruptible filesystem reads. Preservation
collection still uses its existing bounded workers and does not receive the
commit I/O deadline or cancellation token during an individual collection.
Cancellation can arrive between a checkpoint and the following action; existing
worker cancellation and deadline checks remain required. The lane still cannot
admit the installed executable artifacts. Native forwarding coverage, original
reviewer/artifact provenance, full product approval composition, durable
recovery, measured model workflow and approved push remain open. Earlier
same-row low backlog remains. No product route, public API, permission, trust
exception, fallback or new execution authority is added.

No evaluation, hidden or held-out input, scorer, prompt, threshold, dependency,
SBOM, lifecycle, scoreboard, release or readiness change is made. No host key,
GPU, download or push is used.

## Validation

The complete affected fast gate remains **FAIL**: **1,416 passed, 72 failed,
64 ignored**, zero measured or filtered, across 26 unfiltered targets. Every
per-target failure and ignore identity matches the review-read-scope baseline;
only three Linux library passes are added. All three new methods and 79 named
controls pass. Formatting and Clippy pass for all three affected crates. The
three changed source hashes remain fixed, with no later source edit.

The run takes 294.99 seconds including reservations: formatting 1.40, Clippy
10.98 and Rust tests 281.03 seconds. No timeout occurs. The raw log SHA-256 is
`e6e02884c35b0d0fa3750f7a73158e7a0f51cec70bf0555d306433fcc107ba50`.
Raw logs and reproduction scripts are kept in the private bundle
`/lane/state/tmp/t306-commit-control`. Its `run.py` reserves heavy commands
internally and must not be outer-wrapped. Its `check.py` exit zero compares
per-target data only; it never means the full gate passed.

The earlier focused run passes all seven sequencing methods: zero failures or
ignores, 543 filtered tests, compilation 4.53 seconds and tests 0.03 seconds
after the shared reservation. The full unfiltered run supersedes that focused
result. Every heavy command used the shared build reservation. The preceding
independent PASS arrived during full validation; final light checks cover its
unchanged report and final documentation. Public-content, unchanged-row and
staged-blob checks accompany the commit. No native workflow or model evidence
is inferred from these results.

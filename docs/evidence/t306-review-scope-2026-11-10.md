# T-303/T-306: original review read scope

Status: complete affected gate remains **FAIL**; independent core review required. Neither
parent closes and no row or sub-row is added. The executor-sequence base remains
immutable. Its independent review arrived during full validation and is retained
unchanged at [the exact-commit report](../reviews/9e77209e0d21-local-coding-executor-sequence.txt).
It has **Verdict: PASS** for `9e77209e0d2199a15c6191516db9744ce3844afb`,
with no high or medium finding. Its low L-1 native forwarding coverage remains
same-row backlog with the earlier lows.

The original candidate-review interface required read consent in its comments
but did not correlate that selection with both retained roots. A public borrowed
`LinuxCodingCommitReview` now exposes original write projection and inert
candidate planning only after `consented_commit_review` compares both identities
shown in `inspect_apply`. Mismatches refuse before held-root inspection, source
reads or Git observation. Matching identifiers still require the original held
roots; they cannot adopt an unavailable root. The unscoped operations are now
internal to the repository owner.

The view borrows the original owner, preventing replacement or effects through
that owner while it is live. Every use retains the original root, source,
validation, write history, immutable-base and Git-drift checks. External filesystem
changes remain possible; this is not an atomic lease. The trusted caller must
still obtain consent for complete source/metadata reads and offline Git reads.
Root digests correlate selection and do not authenticate consent themselves.

Three new methods cover mismatched, empty, foreign and swapped selections before
unavailable-root inspection; successful original projection after restoring the
held root; replacement of each held root after view construction; and refusal
of another owner's roots, a later substituted worktree and missing ownership.
The public candidate-planning control now uses the scoped view and retains the
exact native artifact refusal. Replacement data, historical receipts and
original source remain intact. Test-only owner substitution is not an adoption
route. Existing full planning and native-write controls remain unchanged.

These tests use actual held directories and native write receipts with synthetic
Git/process observations. They do not demonstrate native candidate execution,
consent authentication or independent review. Caller findings and artifact
scanner assertions remain unauthenticated. Native executable admission remains
unavailable in this lane. Candidate execution, complete product approval
composition, durable recovery, measured model workflow and approved push stay
open. Earlier same-row low backlog remains. No HTTP, CLI or MCP route or new
permission is added. No evaluation, hidden/held-out input, scorer, prompt,
threshold, dependency, SBOM, lifecycle, scoreboard, release or readiness change
is made. No host keys, GPU, download or push is used.

## Validation

The complete affected fast gate remains **FAIL**: **1,413 passed, 72 failed,
64 ignored**, zero measured or filtered, across 26 unfiltered targets. Every
per-target failure and ignore identity matches the executor-sequence baseline;
only three Linux library passes are added. All three new methods and 76 named
controls pass. Formatting and Clippy pass for all three affected crates. All
four source hashes stay fixed; no source edit follows this full run.

The run takes 355.50 seconds including reservations: formatting 81.62, Clippy
30.82 and Rust tests 241.54 seconds. No timeout occurs. Formatting and Clippy
include shared-slot waiting; Clippy reports 12.65 seconds of build execution.
The raw log SHA-256 is
`adcb9bfdba48795f3adec52265f95c989dfca357d84b94117f66e4fb41e77478`.
The private reproduction bundle is `/lane/state/tmp/t306-review-scope`.
Its `run.py` reserves heavy commands internally and must not be outer-wrapped.
Its `check.py` exit zero compares exact per-target data; it never means the
full gate passed. The executor-sequence PASS arrived during the full run;
final light checks cover the retained report and final documentation without
further source edits. Public-content, unchanged-row and staged-blob checks
accompany the commit.

The final focused run passes all three new methods: zero failures or ignores,
544 filtered tests; compilation 1.76 seconds and tests 0.82 seconds after the
reservation. Two earlier focused runs each pass two and fail one method: the
new fixture incorrectly expected completion validation to survive a worktree
rename, then incorrectly expected an original-only rename to invalidate that
separate worktree seal. The final test checks both existing behaviors explicitly
and preserves original validation history. Product checks and thresholds were
unchanged. Both failed logs remain in the bundle as superseded results. An
initial format-only check was cancelled while still waiting for its reservation,
then replaced by the reserved format-and-focused-test command. All heavy
commands used the shared build reservation. The final unfiltered run supersedes
the focused results; none of these tests is native workflow or model evidence.

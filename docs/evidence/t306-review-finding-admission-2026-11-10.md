# T-303/T-306: unresolved findings at commit admission

Status: independent core review required. T-303/T-306 remain open; no new row or
sub-row. The preceding source-selection commit and its ancestors stay immutable.
Its [independent review](../reviews/18dd68113fc3-local-coding-worktree-read-selection.txt)
arrived during validation: **Verdict: PASS** for exact commit
`18dd68113fc3be02da886bd7e4123cf950c2abaa`, with no high or medium finding.
Its four lows remain same-row backlog alongside earlier lows: terminal field
insertion, strict-parsing test coverage, alternate native read-scope construction
and displaced-read wording. No parent closes.

Both kernel commit planners and the original live-owner candidate planner now
apply the same complete-result check. Medium, high and critical findings refuse,
including low-confidence and duplicate-suppressed findings. Suppression remains
presentation evidence rather than resolution. Informational and low findings
remain admissible under this check. The visible high/critical display count,
normalization, confidence threshold and packet schema are unchanged.

The kernel check verifies the complete packet's integrity before checking all
visible and suppressed findings. Candidate planning and local commit planning
both call it. The live-owner path calls it before held-root inspection, source
reads or offline Git observation. Rejected packets do not spend retained write
or validation records or grant any approval.

This does not establish reviewer identity, independence, completeness of supplied
results or artifact scanner provenance. A caller who omits findings can still
build an explanatory packet. Native admission, full product approval composition,
durable recovery, model measurements and approved push remain open. No route or
permission is added; neither parent may close on this increment.

## Validation

The two new methods first reproduced the pre-fix defects. The kernel candidate
planner accepted a medium finding at zero confidence; the live-owner path
reached held-root inspection instead of refusing the review. The final focused
run passes both methods (kernel 0.21 seconds; Linux 0.32 seconds after compilation
and reservation). These are component observations, not native/model results.

The kernel method covers 40 severity/confidence/duplicate combinations across
both planners. It uses integrity-valid candidate and signer fixtures matching
each exact packet to test local commit refusal independently of candidate
refusal. Informational/low cases exercise successful planning. Tampered outer
packets and altered inner finding evidence refuse. All existing normalization,
suppression and visible-count semantics remain intact.

The Linux method covers twelve suppressed medium/high/critical combinations
through the public borrowed review view and the private Git-observation seam.
A worktree made unavailable after selection cannot hide the review refusal
behind a filesystem error; the observer must never run. Retained receipts,
validation history, original write proof and original source bytes remain
unchanged. The deliberate rename changes the sealed root metadata, so returning
the directory does not restore validation readiness. No candidate scratch is
created. A stable pre-rename packet remains a positive inert-planning control.

Two intermediate focused attempts are retained. The first exposed the old
root-before-review ordering. The second exposed a fixture assumption that
renaming a root back would preserve its sealed observation. The final fixture
requires source-binding refusal after that external change and preserved
historical records; no product boundary is relaxed.

The final complete affected fast gate remains **FAIL**: **3,063 passed, 72 failed,
72 ignored**, zero measured or filtered, over 46 unfiltered targets. All 26
previously selected targets match their retained per-target failure and ignore
identities, with one new Linux pass. The additional kernel selection covers 20
targets, all successful: 1,642 passes and eight unchanged ignored helpers. The
kernel library contributes 1,575 passes and eight ignores, including the new
matrix method; integration targets contribute 67 passes. The new Linux method
also passes. Both new methods and 31 named controls pass. No inherited failure
is treated as a passing gate.

Formatting and Clippy pass for all four affected crates. All four source hashes
stay fixed, with no source edit after the freeze. The run takes 423.66 seconds
including shared reservations: formatting 2.46, Clippy 29.96 and Rust tests
390.72 seconds. There is no timeout. The raw log SHA-256 is
`65bd6db6bb5445e5d19c33abc6ada89b03a1129d42bc6b0297aae924ef95d666`.

Raw attempts and reproduction scripts are retained in the private bundle
`/lane/state/tmp/t306-review-admission`. Its `run.py` reserves heavy commands
internally; never outer-wrap it. Its `check.py` exit zero compares per-target
data and requires the full gate to remain FAIL. The eight ignored kernel helper
identities correspond to unchanged base source; all other newly selected kernel
targets have zero failures or ignores. This is a broader affected-crate test
selection, not a promotion of the previous gate.

The preceding independent PASS is retained unchanged. Final light, public-content,
unchanged-row and staged-blob checks include the report and final documentation.
No evaluation inputs, hidden/held-out tests, scorer, prompts, evaluation thresholds,
dependencies, SBOM, lifecycle, scoreboard, release or readiness claims change.
No GPU, host keys, model download or push is used.

# T-303/T-306: correlate worktree source selection

Status: independent core review required. T-303/T-306 remain open; no row or
sub-row is added. The preceding commit and all its ancestors remain immutable.
Its [independent review](../reviews/c5088d85a5d1-local-coding-commit-control.txt)
arrived during validation: **Verdict: PASS** for exact commit
`c5088d85a5d192211d469c2edfef2b7a84ab71e9`, with no high or medium finding.
Its lows on native forwarding, conservative pre-dispatch uncertainty and slow
postflight recovery stay same-row backlog alongside earlier lows.

This batch addresses low L-1 in the retained
[read-scope review](../reviews/0c859b28ff89-local-coding-review-read-scope.txt)
within the same rows. The public `consented_created_sources` method requires
the exact worktree root identity displayed by `inspect_apply`, alongside the
selected relative paths. It compares the retained identity before any held-root
inspection, path validation or source reads. Missing ownership refuses; root
digests correlate selection and do not authenticate consent. A trusted caller
must still obtain that consent and separate permission for model disclosure.

The old uncorrelated selector is internal to the repository owner. Its existing
source bounds and native path checks remain unchanged. Internal original-apply
and commit-review calls retain their enclosing read-selection checks. Worktree
source selection does not inspect the original checkout or Git metadata.

The two existing host consumers use the root displayed by their retained native
owner. Their trusted callers still own prior consent for that exact owner and
any model disclosure. Those consumers do not add a new root-choice challenge
or authenticate the person's prior selection. This batch makes the Linux
selector's contract explicit, not a new end-to-end consent authentication claim.

The existing HTTP edit-preparation request now requires
`expected_worktree_root_sha256`. The terminal includes the root shown in its
read confirmation and rejects a conflicting input identity. The repair path
passes its existing selected-workspace root after matching the original
failure feedback. No caller path can adopt another worktree. A new API test
requires the root field and read consent, preserves an existing inert preview
on mismatched roots before unavailable-source reads, checks malformed roots
and confirms that preparation writes neither source nor the original checkout.
The existing synthetic CLI protocol test checks the transmitted selected root.

Three new test methods cover mismatches before unavailable roots, exact valid
selection, foreign owners, test-only later-owner substitution, lost process
ownership after reopening, replaced roots, exact path allowlists, rejected
metadata/traversal/duplicate selections, invalid content and symlinks. Reads
of unselected paths refuse, including mixed selected/unselected requests. The
worktree-only positive control succeeds while the original checkout is absent.
The returned scope retains its original descriptor: it can still read a displaced
original directory, but cannot adopt its replacement. Preparing an edit rechecks
the current live owner and refuses a foreign, stale or replaced scope.

The existing real-file edit test now enters through the new public selector.
It retains the full preview, wrong-approval refusals, exact approval, written
bytes, original preservation and repeated-decision refusal. Selection itself
does not authorize a write. Creation and process observations in these fixtures
remain synthetic; they are not native workflow or model measurements.

No HTTP, CLI or MCP route, permission relaxation, candidate execution or commit
authority is added. Original reviewer/artifact provenance, complete product
approval composition, native admission, durable recovery, model measurements and
approved push remain open. Earlier same-row low backlog remains except for the
read-selection consistency addressed here, subject to independent review.
No evaluation, hidden or held-out input, scorer, prompt, threshold, dependency,
SBOM, lifecycle, scoreboard, release or readiness change is made. No host key,
GPU, model download or push is used.

## Validation

The final complete affected fast gate remains **FAIL**: **1,420 passed, 72 failed,
64 ignored**, zero measured or filtered, across 26 unfiltered targets. Every
per-target failure and ignore identity exactly matches the commit-control
baseline. Three Linux-library passes and one API-library pass are added.
All four new methods and 85 named controls pass, including the real-file edit
and transmitted-root CLI controls. Formatting and Clippy pass for all three
affected crates. All ten source hashes remain fixed, with no later source edit.

The run takes 235.14 seconds including shared reservations: formatting 1.30,
Clippy 8.22 and Rust tests 224.09 seconds. No timeout occurs. The final raw log
SHA-256 is `9476a70cf75ee15ec0e523237618d024aeab771278af6d95c116a860fc7074fa`.
Raw logs and reproduction scripts are retained in the private bundle
`/lane/state/tmp/t306-worktree-read-selection`. Its `run.py` reserves heavy
commands internally and must not be outer-wrapped. Its `check.py` exit zero
compares per-target data only; it never means the full gate passed.

The earlier focused Linux run passes the three new methods and the existing
approved-edit method. Compilation is 5.55 seconds, with tests 0.99 and 0.27
seconds after reservation. The final focused API and CLI checks pass one method
each, with compilation 4.58 and 6.96 seconds and tests 0.18 and 2.09 seconds.
The unfiltered final run supersedes these focused checks. The first complete
attempt caught two unmigrated host callers after the selector became internal;
the broader migration includes the API and fixtures too. A focused API build
also caught a test-digest formatting error. Both failed attempts are retained;
the final validation uses the corrected frozen source and the complete gate.

The preceding independent PASS arrived during validation and is retained
unchanged. Final light, public-content, unchanged-row and staged-blob checks
cover the report and final documentation. No passing full gate, native
workflow, model result, row closure or owner readiness is inferred.

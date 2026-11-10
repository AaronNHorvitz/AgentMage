# T-303/T-306: bounded preservation traversals

Status: independent core review required. T-303/T-306 remain open without
new rows or sub-rows.

Local preservation traversals previously bounded each regular file and the
combined worktree content, but not combined reflog content. A reflog tree could
therefore request many individually permitted large reads. The entry count also
left the total retained relative-path bytes unbounded: deeply nested names could
consume excessive memory before reaching that count.

Each traversal now bounds combined regular-file content reads to 64 MiB, for
user and agent-owned reflogs as well as the worktree. It refuses excess before
opening or allocating the next file's content. A separate 64 MiB limit covers
the sum of raw relative-path lengths, including directory and excluded names.
The existing 16,384-entry bound remains. Checked arithmetic refuses overflow and
negative file sizes. Metadata-only records do not read content and consume only
their existing entry allowance and the new path allowance.
The skipped top-level `.git` entry consumes one entry and four path bytes; the
guide now states that explicitly, addressing the parent's low L-2 wording.

Path accounting runs before exclusions and before opening the entry. It bounds
the names retained in records and pending traversal paths. Every production
traversal starts a fresh budget; these limits are per traversal, not a shared
whole-collection deadline or an atomic snapshot. Existing descriptor, identity,
no-follow, fixed-size read, cancellation and content-hash checks remain. No
preservation field, source input or record encoding is removed or narrowed.
Within the limits, identical observations retain identical sorted record bytes.

Four new regression methods cover arithmetic refusal, real traversals at the
exact path allowance and one byte over, accounting for excluded entries, and
actual sparse reflogs at exactly 64 MiB and one byte over. Both user and
agent-owned reflog trees are exercised, with exact expected content hashes.
The path tests precharge a private traversal budget and run the real walker;
they do not construct a huge nested filesystem. Production callers cannot
substitute this budget. These are local filesystem component measurements,
not native Git, approval, model or coding-quality results.

## Validation

The focused preservation selection passes all 11 methods, including all four
new regressions, in 3.01 seconds of test execution. Its 572 filtered methods
are not represented as a complete gate. Formatting and Clippy with warnings
denied pass for the three affected crates.

The full affected gate remains **FAIL**: **1,450 passed, 72 failed, 64 ignored**,
zero measured or filtered, across **26 unfiltered targets**. Every failed and
ignored identity matches the hash-verified parent baseline per target. All four
new methods pass; all 11 methods directly in changed sources pass. Both Rust
source hashes remain fixed throughout the full run and afterward. No timeout
occurs. The comparison exits zero as data verification only.

The full run takes **283.70 seconds**, including reservations: formatting
38.50 seconds, Clippy 12.93 seconds and Rust tests 230.59 seconds. The formatting
time includes a shared build reservation wait. Raw log SHA-256:
`9d954137c4727b767cbe5d0733534c09ba4d6baef6f6518a3439057dc78dc1aa`.
No mutation campaign is claimed. Final documentation and exact parent-report
retention follow the Rust source freeze and receive separate final light checks.

The private reproduction bundle is `/lane/state/tmp/t306-preservation-budget`.
Its runner reserves heavy commands internally; never outer-wrap it. It records
the raw full log, exact source hashes, gate plan and elapsed time. Its comparison
requires the parent's exact failed and ignored identities and the four new
passes. A successful comparison is data verification only, never a passing gate.

## Remaining work

The parent commit `62fe004514d20a346ce877bbbf7b397ff5127a1c` starts under
independent review and receives **Verdict: PASS**, with no high or medium finding.
Its [exact report](../reviews/62fe004514d2-local-coding-preservation-files.txt)
is retained byte-for-byte. Its ancestry is preserved. Low L-1 on privileged
device-node substitution and earlier carried lows remain same-row backlog;
the guide addresses L-2 documentation precision, subject to this batch's review.
This batch does not weaken native executable admission, authenticate a checker or artifact scanner,
compose complete original-owner approval, enable a product commit route or
establish repository/signer recovery. Native execution, model measurements and
approved push remain open. No evaluator, hidden tests, held-out set, prompts,
thresholds, dependency, SBOM, lifecycle, scoreboard, GPU, host keys, download,
publication, release or readiness state changes.

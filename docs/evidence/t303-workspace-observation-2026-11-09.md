# Whole-worktree observations around approved validation

T-303/T-306 remain open. This increment adds process-only source observations
to the existing validation owner. Independent core review is required; no row
or sub-row is added or closed.

## Behavior and limits

The original command driver's kernel permit now encloses bounded observations
immediately around the unchanged native executor. It passes the original permit,
workspace and cancellation through once. An ordinary command does not scan.
A missing observation does not alter the command, result, authority or retained
validation history. Failed, cancelled, corrupt, uncertain or incomplete series
cannot retain a workspace binding. A final linked full pass may retain one,
bound to its exact final receipt and the original worktree creation receipt.

Each observation requires two identical full scans. Every regular file and
directory contributes content or metadata, with no hidden, test, configuration,
generated-file or Git-pointer exclusion. Links, special/unsafe/inaccessible
objects and exhausted bounds make the observation unavailable. All resolution
uses the original descriptor-relative adapter beneath the held root. The scans
bind identity, content, mode, size, modification time and change time; restoring
bytes alone cannot restore a binding. No path or raw content is newly returned.
Inspection repeats the current whole-tree observation and checks owner/receipt
identity. Historical command and validation receipts remain unchanged.

Fixed ceilings per scan: 4,096 entries, 256 directories including root,
directory depth 16, 16 MiB per file and 64 MiB of file bytes; each directory has
at most 256 KiB of UTF-8 names. Each two-scan capture shares a cooperative
two-second deadline. File syscalls can block beyond the deadline or delay
cancellation. Observations add local work outside the command elapsed-time
receipt. Large or unsafe repositories may run their approved tests without an
available source binding; no partial observation establishes a match.

`workspace_binding.matches_current_observation` reports matching sampled data.
`atomic_snapshot` and `independent_task_grade` are false. This is not an atomic
filesystem snapshot or a security boundary against privileged filesystem
mutation. The existing `current_workspace_verified` and `test_suite_verified`
stay false. Neither completion verifier changes, and immutable execution inputs
and reviewed runtime acceptance integration remain open. Native/model workflow,
durable recovery, approved Git and parent measurements remain open too.

## Verification

The complete scoped fast gate over Linux, the host and local API remains
**FAIL**: 1,206 passed, exactly 72 inherited failures and none new, 63 existing
ignored, zero measured/filtered across 26 unfiltered targets. Failure identities
exactly match the unchanged T-301 baseline. Formatting and Clippy pass.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 1.20 |
| Clippy, all affected targets | 0 | 13.72 |
| Tests, all affected targets, no fail-fast | 101 | 234.41 |
| Complete scoped gate | 1 | 250.80 |

All four new methods and the expanded denial/expiry history method pass in the
full run. Tests retain a failed receipt and its rerun edge through a repaired
pass; check final receipt/creation binding; detect changes to source, hidden
configuration, tests, names, modes, identities and restored bytes; refuse a
binding after in-flight mutation; and refuse incomplete observations for links,
non-UTF-8 names, oversized files, excessive depth, directories and entries.
Denial and expiry leave history, rerun edges and binding empty.

The initial focused run passed four methods with 380 filtered; that is not the
full gate. The first full attempt found Clippy's collapsible conditional and was
stopped during test compilation. Its failed/interrupted log is retained. The
conditional and an unnecessary test allocation were simplified without changing
checks, then the complete gate ran from the final source state. No source edit
followed that final freeze. Final documentation/report lint and data comparison
pass; the comparison is not a passing gate.

All four Rust source hashes stayed fixed through the complete final run. Its
raw log SHA-256 is
`db4d8da58f400c50852e1dc7e8a74c5cc695c4e88e96edbcbde4f78cf5a4da8c`.
The private reproduction bundle retains plan, source freeze, raw logs, results
and baseline comparison. Each Cargo operation reserves the shared build slot;
two build jobs and four test threads are used. Exactly eleven inherited-failure
fixture directories remain in the private runtime temporary directory; no
unrelated cleanup was performed.

These component fixtures use the existing synthetic executor and synthetic
creation provenance. They do not demonstrate native project-test execution or
qualify a coding model. The unchanged executable-trust refusal still prevents a
native workflow measurement in this lane. No GPU/model process or download was
started. No hidden/held-out task, scorer, pin, threshold, dependency, SBOM, status
model, scoreboard or release gate changed. Independent core review is required.

The incoming [completion-encoding re-review](../reviews/682c79cf8992-local-coding-completion-encoding.txt)
is retained byte-for-byte. It has **Verdict: PASS** for exact commit
`682c79cf8992c406bfb8b48a70d64816dea8e6b5`, with no high/medium finding.
Its low L1 malformed completion grammar and L2 verifier-test fidelity remain
same-row backlog. The validation-history review's low L1 denial/expiry history
assertions are addressed in this batch. Neither parent closes.

# Sealed source inputs for native project validation

T-303/T-306 remain open. Independent core review is required. No row or sub-row
is added or closed. This increment implements
[Decision 0175](../decisions/0175-sealed-worktree-validation-inputs.md).

## Behavior

Direct native validation and retained endpoint native validation select an
internal sealed-worktree mode on the original executor. The exact original
validation approval displays the mode and all capture bounds. The model cannot
choose another command or projection, and no new approval or execution authority
is created. Ordinary commands and other executor clients keep their existing
mount behavior.

Within the original kernel launch permit, after native trust checks, the
executor captures the complete held source tree. It reuses the unchanged
file-projection primitive to copy each exact held-file preimage into sealed
memory. A second full observation must match identity, content and metadata.
No hidden, generated, configuration, test, Git-pointer or empty-directory entry
is excluded, and pointer targets are never followed. Every unsafe, incomplete,
changed or over-budget capture refuses before process launch; no mutable
fallback exists.

The original supervisor retains the sealed descriptors through cleanup and
passes them in a fixed indexed order. The native sandbox constructs a private
tmpfs directory tree, mounts each copied file read-only, restores ordinary
permission bits and remounts the whole directory tree read-only. No live source
root descriptor is passed to this validation child. Original command bytes,
manifest checks, cancellation, resource bounds, network denial, seccomp and
cleanup ownership remain authoritative.

There are at most 128 sealed regular files, 256 directories including root,
4,096 entries, depth 16, 16 MiB per file and 64 MiB total file bytes. The capture
deadline is the smaller of two seconds and the remaining command deadline.
Special permission bits refuse. Filesystem calls can block past cooperative
deadlines or delay cancellation. The capture counts against native command wall
time; the surrounding original observations still add separate local work.
An admitted attempt stays spent if capture fails.

Sealed file contents remain fixed after capture; capture of the source itself
is not atomic. The sandbox preserves bytes, directory names and ordinary modes,
not source inode identity, timestamps or ownership. There is not yet a projection
digest in the canonical command receipt. Existing workspace bindings remain
sampled comparisons, both completion verifiers remain unchanged, and
`current_workspace_verified` and `test_suite_verified` remain false.
Independent task grading, native/model workflow evidence, completion acceptance,
durable recovery, approved Git and parent measurements remain open.

## Verification

The complete scoped fast gate remains **FAIL**: 1,210 passed, exactly 72
inherited failures and none new, 63 existing ignored, zero measured or filtered,
across 26 unfiltered targets. The failure identities match the unchanged T-301
baseline exactly. Formatting and Clippy pass.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 33.53 |
| Clippy, all affected targets | 0 | 13.93 |
| Tests, all affected targets, no fail-fast | 101 | 255.36 |
| Complete scoped gate | 1 | 304.61 |

All four new component methods and the four existing workspace-binding methods
pass in the full run. The new methods exercise real kernel memory-file seals,
exact indexed mount arguments, unchanged copies after host edits, empty/binary
files, hidden files and Git pointer bytes, ordinary modes, empty directories,
the exact 128-file boundary, overflow, links, FIFO/socket objects, unsupported
names, special modes, excessive depth/directories, oversized files, unavailable
and requested cancellation, expired deadlines and mutation during capture.
They do not execute the native sandbox or project tests and are not a model run.
The focused rerun also passed all four methods, with 384 filtered; it is not the
full gate. No GPU/model process or download was started. The original
executable-trust boundary remains unchanged and still blocks native workflow
measurement in this lane.

Six Rust source hashes stayed fixed through the final full gate. Its raw log
SHA-256 is
`cac0a6730f4ae68e740f72925a565a275235c15f49014875716aeebda7d63ac0`.
The private reproduction bundle retains the plan, freeze, complete raw log,
result and baseline comparison. Every gate Cargo command reserves the shared
build slot; two build jobs and four test threads are used. The private runtime
temporary directory retains exactly eleven inherited-failure fixture directories.
Final documentation/report lint and comparison pass separately after prose is
updated. The comparison does not make the complete gate green.

The first focused run had one pass and three fixture failures: the cancellation
fixture used an unsupported contract version, a filename contained a colon
rejected by the existing path contract, and truncating an already-empty sealed
file to zero is a permitted no-op. The fixtures now use the contract constant,
a supported option-like filename, and actual growth/shrink operations. No
production condition or limit was relaxed. The private reproduction bundle
records these initial diagnostics. The first gate then found one unnecessary
generic borrow in the FIFO fixture under Clippy. Only this builder's queued
test reservation was stopped, before source edits, and the failed/interrupted
attempt is retained. The unnecessary borrow was removed. A duplicate standalone
formatter remained queued for over sixteen minutes and was stopped; the final
full gate includes its original formatting check and passes it. No check, slot,
resource limit or other worker was bypassed. The complete final gate above then
ran from a fresh source freeze.

The incoming [whole-worktree review](../reviews/699e99afe3b5-local-coding-workspace-observation.txt)
is retained byte-for-byte, with **Verdict: PASS** for exact commit
`699e99afe3b524f5bee1908bb566a8e911fc87a3`, no high/medium finding.
Low L1 inspection cost and L3 broader observation coverage remain same-row
backlog. Low L2's timestamp precision claim is qualified in the guide: a
write-and-restore within a coarse filesystem tick can escape sampled metadata.
Earlier low findings remain same-row backlog. Neither parent closes.

No hidden/held-out task, scorer, pin, threshold, dependency, SBOM, status model,
scoreboard or release gate changes. The preceding reviewed commit and all
history remain immutable. No self-approval or model qualification is claimed.

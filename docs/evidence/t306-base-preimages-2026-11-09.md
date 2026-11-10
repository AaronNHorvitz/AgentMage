# T-306: Bind candidate review to immutable base source

This same-row prerequisite for candidate execution implements Decision 0187 on
immutable base `81e600d0df9eb479452048c09cbbaba3aafe915f`, under independent
core review at session start. T-303/T-306 remain open. No row or sub-row is
added or closed, and independent exact-commit core review is required.

The live owner compares every reviewed path's first approved preimage with
the original worktree creation commit. Dirty starting bytes and existing
untracked files cannot be silently treated as base content. Creation requires
absence, distinct from an empty blob. Mismatch refuses with
`code.commit.base-preimage`; changed final observations refuse with
`code.commit.git-drift`. The existing complete original write history, published
creation proof, current source, sealed validation and both Git observations
remain required. No effect or approval follows from a successful plan.

The fixed-object reads use the existing offline linked-repository worker with
unchanged executable trust and resource boundaries. Every read disables
replacement objects. Paths are exact parsed tree keys, never Git arguments or
revision expressions. Symlinks, gitlinks, selected directories and descendants
of non-tree entries refuse. Selected blob bytes must reproduce their Git object
identities, then their SHA-256 preimages. The limits are sixteen selected paths,
128 KiB per blob, the existing 4 MiB worker ceiling for tree output and at most
100,000 tree entries. Existing per-command deadlines apply; these sequential
observations are not an atomic lease or durable proof.

Six new collector/protocol tests cover SHA-1 and SHA-256 object identities,
absence versus empty, literal unusual paths, malformed input/output, unsupported
node kinds, corrupt and oversized blobs, and transcript bounds. Two new owner
tests cover missing, extra, dirty and untracked base observations plus final
drift. The existing mixed-creation test adds occupied-base refusals, including
an empty blob. The owner fixtures still use synthetic Git and validation-process
observations with actual confined native file writes.

A disposable direct-Git protocol fixture reads actual base objects after the
worktree and index change and a replacement ref is installed. The base source
and absence stay correct; index, HEAD and local dirty/untracked bytes stay
unchanged by collection. This test produces Git protocol data directly. It does
not admit the native trusted worker, execute a model task or measure coding
quality. System executable trust still refuses this lane's native path.

The retained [retry-cycle review](../reviews/419331de0c75-local-coding-retry-cycle.txt)
has **Verdict: PASS** for exact commit
`419331de0c7517fcee21200fb7a34dff807b5489`, with no high or medium finding.
Low L4 is addressed by correcting that evidence's Linux test classification:
native change-set digests include random dispatch identities and do not repeat;
the equal-digest repair is exercised by the Rust composition and JSON consumer
regressions. Low L5, approval-receipt reuse in inert history consumers, remains
same-row backlog. The base-preimage work also addresses the remaining immutable
base check identified by the creation review; independent review is still needed.

The complete affected gate remains **FAIL**: **1,309 Rust passes, 72 inherited
failures and 64 existing ignores** across **26 unfiltered targets**, zero
measured or filtered tests. Every target's failure and ignore identities matches
the retained base exactly, with eight additional Linux passes. All eight new
methods and the strengthened existing creation method pass. Formatting and
Clippy pass. All five code files retain their verification-start hashes through
the complete run. The exact comparison is data only; it does not make the gate
pass or accept either parent. Existing native/runtime failures remain open.

The complete gate takes **272.35 seconds**, including reservations. Formatting
takes 2.40 seconds, Clippy 10.67 and Rust 257.70, each including reservation.
These are verification timings, not model performance. No timeout occurs.
The raw log SHA-256 is
`cdddc788d5ae0e3f2d1eb88e1d8895e5caa474af7e0cc3c71ab832582811a643`.
The initial queued formatter is cancelled before verification; no build or test
ran in that waiter. Lightweight formatting then completes, and the full gate's
format check independently passes under reservation. Every compilation and test
suite uses the shared reservation. Final lightweight checks cover completed
prose and the byte-identical review report. All owned checks complete.

No model or GPU process starts. Hidden, frozen and held-out inputs, scorers,
prompts, thresholds, dependencies, supply-chain outputs, lifecycle status and
scoreboard are unchanged. Reviewer/artifact provenance, candidate execution,
separately approved Git, native/model measurements and durable recovery remain
open. No public release or owner readiness is claimed.

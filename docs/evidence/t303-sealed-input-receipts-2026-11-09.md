# T-303/T-306: Sealed source inputs in original validation receipts

Date: 2026-11-09. Base:
`dc5cb4532d97431fef5f58fe7f8a12c266c1ab19`.
Scope: non-closing core increment under
[Decision 0176](../decisions/0176-sealed-validation-input-receipts.md).

The original native executor now records the complete sealed source projection
it hands to the original supervisor. The canonical command receipt includes a
typed optional input record: format version, held-root digest, complete capture
observation digest, ordered projection digest, regular-file count, directory
count including root and total sealed bytes. The projection digest binds the
mode/version, root permissions, directory paths and permissions, and file paths,
permissions and exact preimages checked while copying into sealed descriptors.
Host changes after capture cannot change those descriptors or their identity.

The record enters the original platform result after native supervision returns
complete output and known cleanup. The original kernel receipt sealing checks
shape, bounds and owned-worktree scope. Invalid post-launch evidence remains
canonical uncertainty. The kernel authority result already binds the command
receipt digest, and the existing validation receipt binds it in turn. No side
receipt, caller-selected executor, command substitution or new authority is
introduced. Ordinary commands and kernel cancellation before executor entry
have no sealed record. A record identifies selected source inputs, not proof
that the child entered its command body.
Absent records preserve legacy canonical bytes; older strict receipt readers
need an update to read the additive extension.

The original owner retains failed and successful command receipts in its bounded
history. Inspection exposes `command_history` alongside the original validation
`history` and rerun plans. The immediate validation result exposes the sealed
record. The final linked-pass binding references the exact command receipt and
reports whether root, observation and counts agree with its sampled worktree
observation. Missing or mismatched input records do not assert agreement.
Changing current source can invalidate the current observation match without
changing the retained sealed-input record or failed history.

The record describes source inputs, not every toolchain/runtime file. Its digests
are integrity bindings within the original trusted executor and authority, not
authentication for caller-created records. Sealing does not make source capture
atomic. Both completion verifiers remain unchanged;
`current_workspace_verified` and `test_suite_verified` remain false. No parsed
project output, matching sample or sealed input is an independent task grade.

## Verification

The complete scoped fast gate remains **FAIL**: 2,845 passed, exactly 72
inherited failures and none new, 71 existing ignored, zero measured or filtered,
across 46 unfiltered targets. Failure identities match the unchanged T-301
baseline exactly; all 71 ignore identities match the retained earlier complete
gate. Formatting and Clippy pass.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 2.21 |
| Clippy, all affected targets | 0 | 33.17 |
| Tests, all affected targets, no fail-fast | 101 | 435.27 |
| Complete scoped gate | 1 | 472.07 |

All four new methods, both expanded authority methods and the existing
failed-history/workspace-binding method pass in the full run. The focused
`sealed_` run passes 14 tests with one existing environment ignore and 1,949
filtered tests; it is not the full gate. No failed preliminary run or source
repair was needed. Ten Rust source hashes stayed fixed through the complete
gate. Its raw log SHA-256 is
`ed238f8289ee282fd137bef0b87b7aa483204e578f9b0a20beeeee8370491078`.
The private reproduction bundle retains the plan, source freeze, raw log,
result, exact baseline comparison and inherited ignore identities. Exactly
eleven inherited-failure fixture directories remain in the private runtime
temporary namespace. Final prose lint and integrity checks run separately after
this result is recorded. The comparison does not make the full gate green.

Every Cargo command used its original shared build reservation, two jobs and
four test threads. No reservation, trust check, resource limit or other worker
was bypassed. No GPU/model process, model download or native trust workaround
was introduced. Successful native/model coding remains unobserved under the
existing executable-ownership refusal; no scoreboard line is invented.

The new component methods exercise actual memory-file capture and source
mutation with synthetic command results, not native project-test execution.
They cover projection stability, distinct contents/names/modes/empty directories
and hidden files, invalid record shapes, exact command/validation history links,
failed-input retention, final source agreement, post-capture mutation and
mismatched capture observations. The existing canonical-authority method gains
field tampering, absent-field compatibility and scope checks; the invalid
post-launch method also exercises malformed input evidence. No synthetic result
is a native/model workflow observation or a coding score.

Independent core review is required. The preceding sealed-input commit remains
immutable and in flight; no verdict is invented. T-303/T-306 remain open; no row
or sub-row is added or closed. Completion acceptance, successful native/model
workflow, durable recovery, approved Git and parent measurements remain open.
No frozen task, hidden/held-out test, scorer, pin, threshold, dependency, SBOM,
status model, scoreboard, release or owner-ready claim changes.

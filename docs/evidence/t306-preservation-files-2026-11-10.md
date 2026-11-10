# T-303/T-306: descriptor-relative local preservation scans

Status: independent core review required. The complete affected gate remains
**FAIL** on the exact inherited failures.
T-303/T-306 remain open without new rows or sub-rows.

Local preservation scans previously inspected path metadata and reopened the
path for content, with unbounded reads after the size observation. Directory
replacement or file growth between those operations could escape the observed
object or byte limit. This batch uses held checkout/Git directories, opens
ancestors and descendants without following links, and reads content relative
to held parents. Each opened file must match its observed identity and complete
change metadata; fixed-size reads stop at its observed size and check the
original cancellation scope around each 64 KiB chunk. Post-read descriptor and
name checks refuse detected drift. FIFO replacements open nonblocking and fail
regular-file admission. Worktree symlinks bind their link text without reading
target content. Metadata symlinks and non-regular content refuse.

All existing preservation fields and sorted record bytes remain bound. The
64 MiB file/worktree bounds and 16,384-entry bound are retained. The entry
counter now includes directories and exclusions, closing the directory-only
reflog loophole. Descendant directories on a different device refuse. Before
returning a complete manifest, revalidate the held checkout and Git identities
through their original paths without following links. These samples are not
an atomic snapshot, and individual filesystem calls are not preempted.

Eight new methods cover exact source/metadata/link/reflog encodings; regular,
symlink and FIFO replacement before content open; growth, truncation and
same-size rewriting during reads; oversized-file refusal before content open;
chunk cancellation and every local scan entry; symlinked metadata ancestors;
displaced roots; success at the directory-only entry limit and overflow
refusal; and full collector refusal when either root is replaced during
synthetic Git reads. These are real local
filesystem operations. Git responses, creation and authority are not native
workflow evidence. The existing complete-manifest/cancellation tests still run.

## Validation

The initial focused selection passes 11 methods before the collector integration
method is added. The first frozen full run reports 1,446 passes, the exact
inherited 72 failures and 64 ignores across 26 unfiltered targets; formatting
and Clippy pass. Its five source hashes remain fixed. It takes 306.59 seconds,
including reservations, and is retained as validation-1. Inspection then finds
that retaining one descriptor per pending directory can exhaust this lane's
1,024-descriptor limit. The initial overflow test could therefore refuse on
resource exhaustion before reaching the entry counter. This is an inspection
finding, not a failure observed in that full run.

The final traversal retains pending paths and original object observations,
then reopens beneath the held root without links and checks the same object
when visiting. It does not accumulate descriptors for pending children. The
entry-limit regression now additionally requires a directory-only tree at
exactly 16,384 entries to succeed and a larger tree to refuse. The final
focused selection passes all 12 methods; its raw log is retained.

The final complete affected gate remains **FAIL**: **1,446 passed, 72 failed,
64 ignored**, zero measured or filtered, across **26 unfiltered targets**.
Every failed and ignored identity equals the hash-verified parent baseline.
All **eight new methods** pass. The 29 methods directly in changed sources
report **16 passes and 13 inherited failures**, with no ignores. Formatting,
Clippy with warnings denied and final light checks pass. Five final Rust source
hashes remain fixed throughout the full run and afterward. The initial full
run binds its earlier pending-descriptor snapshot; final source evidence is
validation-2 only.

The final full run takes **277.04 seconds**, including reservations:
formatting 1.40, Clippy 10.47 and Rust tests 263.44 seconds.
No timeout occurs. Final raw log SHA-256:
`b896fa579ddeacf98bdae7dafed791075972a6bb024c3936b71e07980279c1b3`.
The comparison command exits zero as data comparison only, never a passing gate.
Raw logs, source freezes, plans and reproduction scripts are retained privately
under `/lane/state/tmp/t306-preservation-files`. The runner reserves heavy
commands internally; never outer-wrap it. No mutation campaign is claimed.
Final documentation and the retained report follow source freeze and receive
separate final light, hygiene and exact-row checks.

The parent commit `0c44e944d1becfe568866f74cf2733b830825333` starts
under independent review and receives **Verdict: PASS**, with no high or medium
finding. Its [exact report](../reviews/0c44e944d1be-local-coding-checker-cancellation.txt)
is retained unchanged. Low L-1 mutation test-hash provenance, L-2 checker
observer-error coverage and carried same-row lows remain backlog. No row closes.

## Remaining scope

Native Git admission remains refused in this lane: installed executable owners
are unmapped by its user namespace. That boundary is preserved. No actual model
run or coding score follows from these filesystem tests. Trusted checker and
artifact integration, complete original-owner approval composition,
repository/signer recovery, native execution, model measurements and approved
push remain open. The parent review passes for its own non-closing scope;
its ancestry is immutable. No row closes. No evaluator, hidden/held-out test, prompt,
threshold, dependency, SBOM, lifecycle, scoreboard, GPU, key, model download,
push, release or readiness change.

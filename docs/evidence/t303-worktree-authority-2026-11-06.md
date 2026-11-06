# T-303 and T-306: approved worktree authority

This non-closing batch follows exact commit
`571431ba85e77a919136ec9ea15da69a12190a0d`. Its separate
[file-map review](../reviews/571431ba85e7-local-coding-repository-map.txt) is
retained byte-for-byte with **Verdict: PASS** for that batch only. Both parent
rows remain open. File-map selection usability, preview-discard disclosure and
diagnostics remain low-priority backlog; its wording finding is corrected in
the ledger. This record makes no new model, native isolation or row-closure claim.

## Running path

The [worktree guide](../guides/coding-worktrees.md) describes the API and terminal
commands. `serve-coding` opens the existing canonical encrypted authority store
in an explicitly chosen private root, with an HKDF-derived key from the existing
control secret. It does not create another store, schema, runtime coordinator,
credential file or model. Ordinary `serve` keeps its prior behavior.

Only trusted control requests can inspect a repository, prepare creation or
decide an exact preview. Terminal inspection and creation require separate fresh
typed challenges. Creation displays the exact immutable base, task branch, plan,
preservation hashes, roots and expiry. The terminal checks both root identities
around approval; native preparation and dispatch hold and revalidate roots and
metadata and refuse unsafe, overlapping or nonempty destinations and source drift.
Native preflight time counts against approval expiry before grant consumption.
Model output never obtains this control authority.

Approved dispatch uses the existing session-read parent, exact derived
`GitWorktreeCreate` grant, shell-only tool registry, policy, durable authority
transaction and `RepositoryEffectDriver` with `LinuxRepositoryExecutor`. The
existing collector, trusted Git artifact, local-only invocation and preservation
reconciliation remain unchanged. No alternate Git subprocess runner is added.

Canonical authority receipts survive restart. Complete previews and specialized
repository receipts remain process-only; this is not durable coding-run storage.
Consumed or interrupted effects are never automatically retried. Inspection and
creation are non-interruptible after admission, and the API truthfully reports
that status. An earlier acknowledged cancellation prevents dispatch. Crash-safe
native-child ownership and total inspection deadlines are not established.

Successful creation would still require separate source selection before reads,
model disclosure or writes. There is no integrated owned-worktree edit/test/retry
loop, automatic application to the original checkout, commit, push, removal or
MCP. Current write previews retain their conservative isolation/test labels.

## Verification scope

The native-owner tests use real held filesystem objects and the real encrypted
authority owner with explicitly synthetic platform results. They exercise exact
consumption, duplicate-attempt refusal, receipt reopening, preservation failure
and an unwind after durable consumption recovering as uncertain without another
launch. An unwind is not a killed native Git process. Synthetic success never
creates a worktree and is explicitly checked against a missing destination.

The native entry-point test executes its trust-refusal branch in this lane.
`/usr/bin/git` has the overflow owner rather than root; unchanged native checks
return `linux.repository.git_artifact.invalid`. Its positive creation, denial
and expiry assertions are present for a correctly mapped native environment but
were **not executed here**. There is no native creation result to publish.

Actual service/terminal tests cover protected routes, exact object shapes,
inspection consent, non-TTY refusal, separate typed inspection, invalid challenge
text, no-current-preview refusal, state reopen and the native trust refusal.
A separate synthetic API peer tests exact terminal creation approval, one send,
decline and replacement of the displayed management root. Its success response
is protocol evidence only, not native execution or product acceptance.

Final gate, development-context and publication-audit results follow. Raw records
remain in the private lane scratch
directory `tmp/t303-worktree-authority/`; none is a new model score. Frozen tasks,
hidden tests, prompts, graders, dependencies, thresholds and status-model fields
are unchanged. The long acceptance gate remains at T-310.

## Final results

| Check | Observed result |
| --- | --- |
| Rust formatting | PASS; reservation wait dominates the recorded 1,501.10 seconds |
| All-target Clippy, warnings denied | PASS; 19.35 seconds including reservation |
| Full selected source tests: host, Linux, local API | FAIL: 792 passed, 72 failed, 62 existing ignored, zero filtered; 117.45 seconds including compilation/reservation |
| Failure identity comparison | All 72 identities equal the original T-301 baseline: 53 host and 19 Linux; no new or missing failure |
| New native-owner cases | Five pass within the full suite; actual native case is the trust-refusal branch described above |
| New API/CLI/PTY cases | Five pass, no ignored/filtered case; synthetic creation responses remain labelled as such |
| Existing API regression targets | Library two, terminal nine and socket API 35 cases pass |
| Changed Markdown, review retention and publication audit | PASS; exact prior review, frozen inputs, ledger statuses, privacy and links verified |
| Repository documentation policy scan | PASS: 757 Markdown files and all policy invariants; this is not the long acceptance gate |
| Development compatibility | 30/30 contexts match both historical campaigns; 3/3 recorded accepted replies apply exact native bytes; final run 4.478 seconds |

The fast gate remains **FAIL**. No failure is waived, filtered or relabelled as
native success. Its initial plan preceded the documentation additions; separate
Markdown verification covers all five changed Markdown files.

Development reproduction uses the actual API, CLI and controlling terminal with
a synthetic upstream, known source basenames and one-source development
projections. It exercises the existing source/proposal/write workflow through
ordinary `serve`, not the unavailable native worktree creation path. All 240
retained manifest files verify, and the builder recomputed the three native
write receipt chains against the historical replies. This establishes context
compatibility and reply propagation, not retrieval quality, inference, grading,
test execution, a held-out score or a new coding-quality measurement.

Cargo's full test build replaced the mutable API/CLI binaries under a different
feature combination. The first audit therefore refused its current-binary digest
check. After rebuilding the documented API/CLI targets, a second identical
compatibility run establishes final digest agreement. The first run, failed audit
and initial compiler/registry diagnostics remain retained. Binaries are not
archived: independent review must rebuild rather than trust `target/debug`.

The final development summary SHA-256 is
`83a76a286ea5c420e1d50d45bbeb8190cc2cafbdfc942ddc2dddc8950491c1d9`;
its 240-file manifest SHA-256 is
`9c724f706adb5f53c21915a9a691e5a130b085a274ecdcbac0e0e2778391e187`.
The driver records source and runtime binary digests. The final builder audit
records source bindings and the unchanged failure identities; it is not an
independent review. This batch awaits that separate core assessment and closes
neither parent.

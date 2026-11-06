# T-303 and T-306: file discovery and ranked source choice

This is a non-closing core implementation batch under Decisions 0153/0154.
The preceding terminal client has an independent
[exact-commit PASS](../reviews/7c35f361faeb-local-coding-terminal-client.txt)
at `7c35f361faebb878561eec89c1891d26d3f3fd7b`. Its review is retained
unchanged; neither that verdict nor these builder checks close either parent.
This batch needs separate independent read-only core review. No task row is
added, no frozen task or hidden test is edited, and no held-out task runs.

## Delivered behavior

The native Linux file map enumerates an explicitly chosen root through the
existing held-root/path adapter. It inspects names and metadata without reading
file content, following instructions, launching Git or invoking a model. The
control-only `POST /v1/code/map` requires explicit discovery consent and records
the operation in activity. Mapping does not change the current scope or pending
preview and grants no source, model or effect authority.

The map ranks eligible paths by deterministic lexical task-term matches, with
extra weight for basename matches. It excludes hidden and generated/dependency
objects, skips links and special/non-selectable objects, and refuses traversal
or budget failures without returning a partial map. It is not a Git inventory,
content search, secret detector or atomic filesystem snapshot. Limits and
coverage are documented in the [workspace contract](../guides/coding-workspace.md#metadata-only-file-map).
The two-second deadline is cooperative between native calls; a blocked
filesystem operation can delay it and cancellation.

The terminal `map` command displays names only. `map-select` proposes at most
sixteen positive-score paths, shows every path with the root and configured
endpoint, and requires fresh typed `share` confirmation before source selection.
No matches or a declined confirmation preserves the old scope. Selected files
must still pass the existing complete-source native boundary and byte budgets.
Only a subsequent `propose` sends selected context to the chosen endpoint.
Existing parser-backed outlines, excerpt selection, strict edit checking and
exact approved writes remain unchanged.

The mapped root's canonical path/device/inode digest is checked before and
after the terminal confirmation. Optional `expected_root_sha256` binds the
selection request to that digest, checked before reading source and against
the resulting held scope. A mismatch refuses without replacing prior state.
The API's operation lease and existing endpoint/revision display bindings remain
in force; omission preserves the original explicit-selection API.

The preceding review's low DEL finding is fixed by escaping U+007F, including
on error output. The client now displays well-framed JSON HTTP refusals, capped
at 4 KiB and escaped on stderr, then exits nonzero. It distinguishes stale
approval refusal from a dispatched unknown-effect result without retrying.
Malformed, oversized, streaming or truncated error bodies are refused. The API
and terminal guides now document the review's API-listener port trust residual;
server authentication or an owner-only socket with peer checks remains required
before the install kit. Documentation does not resolve that residual authority
exposure. The other historical low findings remain backlog.

## Verification

The final ordinary per-change gate used the preceding terminal commit as base,
the existing shared Node runtime, locked offline Cargo dependencies, two build
jobs and four test threads. Rust checks acquired the shared build reservation;
no test filter, new ignore, failure allowlist or trust exception was used.

| Check | Observed result |
| --- | --- |
| Selected host/API/Linux formatting | PASS |
| Selected all-target Clippy, warnings denied | PASS |
| Native discovery integration target | 5 passed, zero failed/ignored/filtered |
| API library, terminal/PTY, API contracts | 2, 9, 35 passed respectively, zero failed/ignored/filtered |
| Full per-change test command | FAIL: 782 passed, 72 failed, 62 existing ignored, zero filtered; 78.76 seconds |
| Failure identities | Exactly the original 53 host and 19 Linux baseline identities; no new or missing failure |
| Changed Markdown and retained-review checks | PASS |
| Optional whole-repository documentation policy scan | Not run: remained queued for six minutes; only this builder's waiting process group was cancelled |

The new native tests exercise deterministic ranking without reading even binary
or empty file content, hidden/generated exclusions, linked/special objects,
unsafe paths, symlink roots, invalid queries, fixed name/depth ceilings,
cancellation and root replacement. The actual terminal tests traverse real
sockets and controlling pseudo-terminals. They exercise absent/wrong/fresh
disclosure approval, no-match preservation, root replacement while approval is
displayed, the existing sixteen-file read budget, and escaped error diagnostics
without a success result. API contracts enforce control-only explicit discovery,
strict request shape, unchanged scope after mapping, activity visibility, and
root-bound selection refusing without losing the prior scope.

The first gate attempt was interrupted while queued so the source batch could
finish. The second caught a nested conditional rejected by Clippy; its redundant
test command was interrupted through the existing supervisor. Both logs are
retained and neither is a completed or passing test run. The conditional was
collapsed without changing the check or suppressing the lint. The final gate
above completed. No test, threshold or sandbox boundary was weakened.
The optional documentation policy scan never acquired the shared reservation
and made no validation claim. Changed-file Markdown, retained-review and
publication checks pass. The unchanged long acceptance gate remains at T-310.

A reserved external-process check ran the actual API and terminal against a
synthetic upstream for all **30/30 development tasks**, then replayed all
**3/3 previously accepted development replies**. Each disposable projection
contains its one original source file and an unselected sentinel. The query is
the known source basename; all 33 maps rank exactly that source for selection,
followed by fresh typed model-disclosure approval. This checks the map/selection
connection, not retrieval quality on an unfamiliar or complete repository.
All 30 model contexts and serialized upstream messages exactly match both
historical local-model campaigns. The three replayed replies prepare and apply
the exact historical bytes through fresh typed write approval and kernel/native
receipts; sentinels remain unchanged. Total wall time was **4.743 seconds**.
A separate Python audit recomputed all native receipt digests and chains.

This is metadata discovery, synthetic transport verification and recorded-reply
replay: **no model inference, grader, hidden assertion or held-out run** occurs.
No new model pass rate is reported, and the historical scoreboard is unchanged.
There is no prompt, excerpt-selection, edit-format or evaluation-threshold
change. Tests demonstrate no source disclosure from mapping alone.

The retained summary SHA-256 is
`f4074e19c02bcab9fb9bc0f8b34786a6ecb06a638f6601d933f614f0781fdc75`;
the 240-file retained manifest SHA-256 is
`15cfd4df6e59a1d0eb0446f13089e309ee2a581adde8f0abd225fee4680584a6`.
The private review request identifies the driver, raw logs and reproduction
commands. Records bind source and builder-executable digests at run time; the
build directory is mutable and binaries are not archived in this batch.
Independent review should rebuild and reproduce behavior. Raw source and
terminal records stay private because they contain temporary host paths.
The publication audit passes the retained review's exact bytes, source/driver
bindings, all 240 manifest entries, historical-reply bindings, receipt chains,
unchanged frozen inputs, original API regression bytes, ledger/checkbox
preservation, privacy and local links.

## Remaining work

Commands and worktree operations still use the existing native adapters. This
lane's system-executable overflow-ownership blocker is unchanged; no copied or
re-owned executable, relaxed trust check or alternate unconfined runner is
introduced. No model process, GPU reservation, download, new dependency or
supply-chain regeneration is part of this batch.

The current write path still targets selected workspace files, and its previews
and receipts remain process-only. Worktree isolation, confined project tests,
test/fix retries, durable runs and resume/replay, Git apply/commit/push, MCP and
full coding acceptance remain open. Path ranking is only a source-choice aid;
the separate person-facing disclosure and exact-write decisions remain required.
No production platform, parent closure, owner-ready or release claim is made.

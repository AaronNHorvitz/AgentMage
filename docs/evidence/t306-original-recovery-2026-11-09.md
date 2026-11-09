# T-303/T-306: Preserve terminal records and resolve declined transfers

Date: 2026-11-09. Base:
`af9d467237845456a883eb948e91e3fe7bf7e9c0`.
Same-row core repair; both parents remain open. No row or sub-row closes.

The endpoint original-transfer command previously accepted a symlinked root at
its initial shape check, then released the terminal run before the existing
native root check refused the read. It now checks directory metadata and exact
canonical spelling before collecting the record or offering release. Missing
roots, regular files, symlinked roots and ancestors, and redundant path
spellings refuse. It does not normalize an alias into a different selection,
read source contents or grant authority. These observations are non-atomic;
the original held-root checks still run before read consent and after it, and
native source/preimage checks still guard apply. This is an early refusal,
not a substitute for native validation or a filesystem race guarantee.

The ordinary `original`, `original-prepare`, `original-apply` and `original-deny`
commands remain unchanged. The guide now spells out recovery after a declined
post-release read or apply. Denial consumes a pending preview without consuming
eligible changes; a later attempt needs a newly prepared preview and consent.
No command restores a released run, its events, model ports or approvals.

## Verification

The six focused terminal-transfer tests pass, with 25 unrelated methods
filtered in that focused invocation. The complete affected API gate then passes
all **241 tests across eleven unfiltered targets**, with zero failed, ignored,
measured or filtered tests. Formatting and all-target Clippy pass. Both Rust
source hashes remain unchanged across the gate. Eight named regression methods
are checked in the complete raw log, including the base extraction-refusal tests.
No failed compile or test invocation occurred in this batch. Earlier host/Linux
failures are outside this API-only selection and are not cleared.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 11.37 |
| Clippy, all API targets | 0 | 1.85 |
| Tests, all API targets, no fail-fast | 0 | 50.11 |
| Complete affected gate | 0 | 65.12 |

Raw log SHA-256:
`cae12cfcd4b074d1ccbb7613dc987e9cb9ac7cb1b71029eb554fead3f95f1d94`.
The private reproduction bundle binds source hashes, focused results and the
complete gate log. Every Cargo command reserves the shared build slot with two
build jobs and four test threads. Earlier formatting and focused commands also
waited for reservations, outside the complete gate's measured allowance.
Final prose checks run separately after recording these results.

The regression uses actual terminal processes, both authenticated HTTP review
services and native file edits/transfers. Creation provenance is synthetic;
the terminal run is cancelled before any model or tool dispatch. Each service
is tested after both read and apply declines, with separate apply/deny outcomes.
Inspection is compared with the server's exact pending/latest state, read and
apply confirmations are checked, and a repeated consumed decision refuses.
Assertions preserve the worktree, unrelated human file and index sentinel;
apply changes only the eligible original file. The upstream endpoint receives
no request. This is not native worktree creation, model execution, test execution
inside a coding run or a coding evaluation score.

Alias regressions keep the cancelled run's events available and original-apply
routes unavailable after refusing root and ancestor symlinks, redundant path
spellings, missing roots and non-directories. Existing interruption, drift,
lost-reply and exact three-decision tests remain unchanged in their assertions.

## Review and limits

This addresses low L1/L2 in the preceding
[terminal-transfer independent review](../reviews/c86aabe0bb9c-local-coding-terminal-original-transfer.txt).
The extraction-response base remains immutable while its independent review is
in flight. No implementing agent verdict closes these findings or either parent;
independent core review of this batch is required.

Native executable trust remains unchanged. Native/model workflow, approved Git,
durable recovery, inherited host/Linux failures and parent measurements remain
open. No hidden test, frozen task, held-out input, scorer, threshold, prompt,
dependency, supply-chain output, status model, scoreboard, long gate or release
is changed. No model/GPU work, download or resource increase is performed.

# T-303/T-306: Private terminal records before release

Date: 2026-11-09. Base:
`b31925c3655a6ca0d19a324cc1733539e1f6050e`.
Both parents remain open; no row or sub-row is added or closed.

The terminal client can now save a complete terminal event record with explicit
consent and replay it offline after the service and credentials are gone.
`endpoint-record-save` uses the unchanged authenticated request/event routes,
record bounds, canonical verifier and offline replayer. A controlling terminal
displays the request, complete destination and byte count before an exact
`save-record` decision. The existing selected directory must be canonical,
current-user-owned and mode 0700. Its descriptor is held across consent, and
identity is rechecked before creation. Exclusive no-follow creation refuses
every existing name. The mode-0600 plaintext file is named by the sealed request
digest. File and directory synchronization precede the saved-file receipt.

`endpoint-original-apply-saved` saves and flushes that receipt before offering
release. A declined or failed save leaves the original run held. Declining
release preserves both record and run. Subsequent release, two-root read consent
and exact original diff approval remain independent decisions. The ordinary
stdout-only commands remain available. No command automatically retries,
removes a partial file or restores execution authority.

## Verification

The complete affected API gate passes all **243 tests across eleven unfiltered
targets**, with zero failures, ignores, measured tests or filtered tests.
Formatting and all-target Clippy pass. All five Rust hashes remain unchanged
through the complete gate. Ten named new and inherited regression methods are
checked in its raw log. Existing test assertions and every task-row checkbox
are unchanged against the base. Earlier host/Linux failures are outside this
API-only selection and are not cleared.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 0.30 |
| Clippy, all API targets | 0 | 0.50 |
| Tests, all API targets, no fail-fast | 0 | 43.20 |
| Complete affected gate | 0 | 45.58 |

Raw log SHA-256:
`739c81df810511665a0539ee3e887d35bb2a9f51f120cee5c4da7a974efd984f`.
Every Cargo command reserves the shared build slot with two build jobs and four
test threads. Earlier formatting and focused commands also waited for shared
reservations, outside the complete gate's measured interval. Final prose and
privacy checks run separately after recording these results.

The first focused invocation compiled successfully, then failed one new
assertion expecting compact JSON instead of the existing terminal renderer's
pretty JSON. The command itself had returned success with one confirmation.
Only the new receipt assertions were corrected; no existing test or threshold
was weakened. The subsequent standalone focus passes one test, with 242
unrelated tests filtered across its all-target invocation. The transfer focus
passes one test with 32 unrelated methods filtered. Neither focused invocation
is presented as a complete suite. The final gate above filters nothing.

A preliminary complete gate also passed 243 tests in 251.88 seconds including
reservations. Final inspection then strengthened the two new tests to reread
the saved files from disk after service shutdown, rather than replaying bytes
read beforehand. The final source was frozen again and the complete gate rerun;
the table and hash above describe that final run. The private bundle retains
the preliminary run separately. No implementation or existing assertion changed
in this strengthening.

Actual PTYs and authenticated HTTP exercise declined and interrupted save
consent, nonterminal input, directory aliases, unsafe modes, directory identity
replacement and existing file, symlink, hard-link and directory collisions.
The successful mode-0600 record replays in a fresh process after service shutdown
and removal of authentication files. Saving leaves the run held. Saved-transfer
tests exercise both original-review services: declined or colliding saves never
release; a saved record survives declined release; a fresh destination permits
four distinct decisions in order before the native original-file transfer.
Worktree, unrelated human file and index sentinels remain intact. The retained
cancelled record still replays as cancelled after that transfer.

Creation provenance is synthetic. Runs are cancelled before any model or tool
dispatch, and the upstream endpoint receives no request. This demonstrates
actual client retention, HTTP, native file transfer and offline inspection,
not native Git creation, model execution, test execution inside a coding run
or a coding evaluation score.

## Independent review and limits

The retained [release-response review](../reviews/af9d46723784-local-coding-release-response.txt)
has **Verdict: PASS** for exact commit
`af9d467237845456a883eb948e91e3fe7bf7e9c0`, with no high or medium finding.
Its two coverage lows remain same-row backlog. The retained
[original-recovery review](../reviews/b31925c3655a-local-coding-original-recovery.txt)
has **Verdict: PASS** for exact commit
`b31925c3655a6ca0d19a324cc1733539e1f6050e`, with no high or medium finding.
The guide now explicitly calls for canonical root spellings. Its residual
wrong-canonical-root ordering low and earlier native combined-execution lows
remain backlog. Neither report closes a row. This increment requires a new
independent core review; the implementing agent supplies no verdict.

Private file retention is durable inspection, not durable execution recovery.
The record contains canonical envelopes and payload references, not full tool
outputs. It is plaintext and unauthenticated. Replay checks self-consistency,
not origin, referenced payloads, model quality or task success. No owner, grant,
approval, model port, budget or execution cursor is reconstructed. Path checks
are observations, not an atomic filesystem race guarantee. A failure after
creation can leave a partial file, requiring explicit disk inspection.
Synchronous filesystem calls can delay stop; byte bounds do not supply a
filesystem-call timeout.

Native executable trust remains unchanged. Native/model execution, approved
Git, execution recovery, parent measurements and inherited host/Linux failures
remain open. No frozen or hidden test, held-out input, scorer, threshold,
prompt, dependency, supply-chain output, status model, scoreboard, long gate
or release is changed. No GPU/model work, download or resource increase occurs.

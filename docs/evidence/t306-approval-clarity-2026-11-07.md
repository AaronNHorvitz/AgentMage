# T-303/T-306: Creation confirmation and recovery snapshots, 2026-11-07

Builder batch based on `3ee8285e595a208fb1e354d46ab95e56d24c2bae`.
Both mission parents remain open; no row or sub-row is added. This addresses
low findings in the existing creation-transfer and bound-recovery reviews.

The terminal original-checkout confirmation now describes a creation-only
preview as creating absent files with the complete displayed contents and
modes. It states that existing destinations refuse and parents must already
exist. It still displays the entire original preview, verifies the held root
before and after consent, and sends the same exact approval digest once.
Existing-file edit wording is unchanged. The kernel remains the effect owner;
wording changes no plan, digest, permission, native write or verification rule.

Recovery reads service status and then the owner identity. If their
recovery-pending flags disagree, the terminal now exits before displaying any
confirmation or sending a recovery POST. The person must inspect afresh.
Matching reads do not establish an atomic snapshot; the existing server-side
lifecycle checks and exact handoff binding remain mandatory at decision time.
No retry, implicit cancellation, run release or original write is introduced.

The independent [publication/usage re-review](../reviews/6c73f38048d7-local-coding-publication-repair-and-provider-usage.txt)
is retained unchanged. Its **Verdict: PASS** is for exact commit
`6c73f38048d7b3bd362956c330fa67df469e9354`; it resolves creation-publication M1
and accepts optional endpoint usage as a non-closing component. Its low L2
(strict usage validation can discard an opted-in answer) is backlog.
That verdict does not review this batch. The later
[MCP review](../reviews/3ee8285e595a-local-coding-scoped-mcp.txt) arrived after
source verification and is also retained unchanged: **Verdict: PASS** for
`3ee8285e595a208fb1e354d46ab95e56d24c2bae`, no high/medium findings. Its four
low findings (refusal detail, invalid-request replies, staging-origin display,
and warning against copying a control token as the client token) remain backlog.
Neither verdict closes a parent or reviews these approval changes.

## Verification

The unchanged per-change gate passes under one shared build reservation with
two build jobs and four test threads: **119 passed, 0 failed, 0 ignored,
0 filtered**, across all eight selected targets. Formatting and Clippy with
warnings denied pass. Seven commands total **25.92 seconds** excluding the
reservation wait: formatting 0.20, Clippy 1.15 and full Rust tests 23.17 seconds.
All four Rust source hashes remain unchanged after verification. The subsequent
retention of the newly arrived MCP report changes only prose/review checks;
the final plan selects exactly the same Cargo commands and four source hashes.
Final touched-document lint, review-format, whitespace and privacy checks
cover both retained reports and the final ten-path batch. A broader policy
check was queued but cancelled before work; no repository-wide policy pass is
claimed. The long acceptance gate remains for T-310.

The complete runtime-terminal target passes 23 tests, including the new
regression for both inconsistent lifecycle directions: no confirmation and
exactly the two GETs, with no POST. Existing matching attached/pending recovery,
decline, stale-binding and lost-reply cases still pass. The complete worktree
terminal target passes 13 tests. Its existing real-PTY matrix now checks the
creation-specific contents/modes and absence/parent warnings, the unchanged
complete escaped preview and exact decision body, wrong consent and root
replacement refusal, and existing-file diff wording.

The tests use actual terminal executables and controlling terminals against
synthetic HTTP peers. No command or write is dispatched by those peers.
These contracts do not establish native Git
creation, a real-model coding workflow, independent task grades or a score.
The green gate covers only this batch's local API crate and its interfaces;
it does not rerun or clear the host/Linux suites. The existing deferred native
executable-trust blocker remains unchanged.
Configured-endpoint coordinator admission, durable ownership, approved Git
publication and parent development-set measurements remain open. No hidden or
held-out input, threshold, ignore, dependency, SBOM or implementation status is
changed. Exact-commit independent core review is required.

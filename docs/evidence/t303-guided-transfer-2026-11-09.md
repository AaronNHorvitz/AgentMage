# Guided coding, saved records and original transfer — builder evidence

Date: 2026-11-09. Rows: T-303 / T-306, both open. Tier: core.
Base: `ad9a20492b2db26fddd6e30ce6bbf32f9914aeb4`.
No row or sub-row is added or closed. Independent core review is required.

## Behavior

`endpoint-run-apply-saved` connects the existing retained coordinator to private
record retention and separately approved original diff review. It uses one
signal latch and one deadline throughout. The original owner, model transport,
permission checks, budgets, event sequence and tool executor stay authoritative.
Separate exact confirmations still precede every model disclosure and tool,
record saving, terminal release, source reading and original apply.

The complete canonical terminal record is collected, verified, saved and
synchronized before offering release. Both saved-transfer commands reject
archive directories inside either selected checkout, including aliases. They
check the canonical private destination before disclosure, before saving and
before release, including after release confirmation. These are sampled metadata
checks, with no lease against concurrent directory moves. Standalone saving
retains its explicitly chosen destination policy.

The transfer presents the observed terminal envelope. A useful diff may be
transferred after a failed or cancelled run, but its failure remains in the
saved record and its apply confirmation confers no test or task certification.
Any declined confirmation, interruption, timeout, refusal or uncertain reply
stops further requests. There is no automatic retry, cancellation, release,
cleanup or execution recovery. Synchronous record writes, filesystem
synchronization and blocked stdout can delay stop; keep output draining.
Existing standalone commands permit separately
approved inspection and resolution after a post-release decline.

See [Decision 0181](../decisions/0181-guided-coding-record-and-transfer.md) and
[the guide](../guides/runtime-endpoint-runs.md#guided-run-saved-record-and-original-diff-review).

## Verification

The existing fast gate ran against the base above. Each Cargo command reserved
the shared build slot individually, with two build jobs and four test threads.
The only PATH addition was the retained Node installation for Markdown lint.

| Check | Exit | Seconds, including reservation waits |
| --- | --- | --- |
| Whitespace | 0 | 0.10 |
| New-file whitespace | 0 | 0.10 |
| Retained review format | 0 | 0.10 |
| Markdown, four files | 0 | 1.25 |
| Affected-package formatting | 0 | 2.30 |
| Clippy, all targets, warnings denied | 0 | 1.90 |
| Tests, all targets, no fail-fast | 0 | 57.37 |

Total gate wall time was **63.20 seconds**, including its reservation waits.
A preceding formatting command also waited for a shared reservation; that
preparation wait is outside this gate timing. The complete affected API gate
passes **255 tests across eleven unfiltered targets**, with zero failures,
ignores, measured tests or filtered tests. All twelve named regression checks
pass. Eight Rust-file hashes stay fixed throughout verification. No initial
compile or test failure occurred in this batch. Raw gate-log SHA-256:
`828b95739d70611246d5aaef3f9f72caaa2a840e33fc231998da5f4d2d655303`.

The private review bundle retains exact paths, Cargo commands, per-target
counts, source hashes, the raw log and reproduction checks. The sole changes
to existing test assertions strengthen all five standalone collision cases:
each keeps its refusal and sentinel checks and adds an exact one-prompt check.
Other existing test lines and all ledger rows remain unchanged. The retained
review is byte-identical to its source. Added lines pass the privacy scan.
Final prose is checked again after these results are inserted. Earlier
host/Linux failures are outside this API selection and are not cleared.

Seven new methods cover actual controlling terminals and authenticated HTTP.
Four use the retained original repository service. In the driven case, a
synthetic model proposes one real native worktree edit, then emits a malformed
second response. Seven separate confirmations precede disclosure, tool consent,
next disclosure, saving, release, source reading and exact original apply. The
record's canonical sequence retains `FAILED`; the original checkout receives
only the reviewed diff. Human work and index sentinel stay intact. The saved
record remains valid after service shutdown. This uses synthetic creation and
model bytes, not native Git creation, real inference or project test execution.

The other retained-service cases decline every phase on both original-review
services, check one deadline across disclosure and saving, interrupt release,
and reject archive directories inside either checkout and aliases before any
model disclosure or release. Three recording-peer methods test non-TTY invocation,
invalid-budget and noncontrolled selections; lost, refused or stalled replies
after an approved tool request; and an earlier disclosure line queued after
tool approval. The last case presents distinct step tokens and sends no held
tool approval. Failed requests never reach saving, release or diff routes.
No task grade or coding score is inferred from these tests.

## Review and remaining scope

The [guided-endpoint independent review](../reviews/ad9a20492b2d-local-coding-guided-endpoint.txt)
has **Verdict: PASS** for exact commit
`ad9a20492b2db26fddd6e30ce6bbf32f9914aeb4`, with no high or medium finding.
It closes no row. Its low L1 is addressed by step-bound guided disclosure tokens
and a PTY regression that queues an earlier token after tool consent. The held
approval is never sent. Single-shot tokens remain unchanged. No builder verdict
closes a row. The preceding private-record
review's low archive-placement finding is addressed by the new preflight and
rechecks. Its low standalone collision-prompt finding is addressed by requiring
exactly one prompt in every existing collision case, preserving the refusal
and sentinel assertions. These repairs await independent review. Inherited
low findings remain same-row backlog.

Approved signed Git needs original verified branch/base and review provenance;
this increment does not synthesize them from a path or serialized record. Native
executable trust remains an owner-action blocker. Native/model coding
measurement, approved commit/push, durable execution recovery and inherited
host/Linux failures remain open. No frozen task, hidden test, held-out set,
scorer, threshold, prompt, dependency, SBOM, status, scoreboard, long gate or
release is changed. No GPU/model run, download, resource increase, history
rewrite or push occurs.

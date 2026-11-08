# T-303/T-306: bounded guided terminal workflow

Date: 2026-11-08. Tier: core. Immutable predecessor:
`42949c642a5eeeeb0f2c336b17617a0e108df664`.
Independent exact-commit review is required. Both parent rows remain open;
no row is added or closed.

## Behavior

The guided client now carries one elapsed-time deadline across preparation,
model proposals, read/edit/test/failure-disclosure decisions and all repairs.
`code-worktree` defaults to 3600 seconds. `code-worktree-bounded` accepts an
explicit 1 through 3600 second budget. Invalid budgets and piped execution
refuse before requests. Retry attempts cannot renew this deadline.

The existing native terminal driver's signal latch and nonblocking controlling
terminal confirmation are reused. SIGINT, SIGTERM and SIGHUP stop further
requests. The shared HTTP client checks the latch before credential access,
connection and request writes and while polling headers and bodies. A pending
read closes on stop; the original 180-second per-request ceiling also remains.
The native permission owner, exact preview checks, strict preimages, isolated
worktree provenance, validation parser and three-test ceiling remain unchanged.

Stopping is not a server cancellation receipt. An already sent operation may
continue, and lost replies stay uncertain. The client sends no cancellation,
denial, release or retry. Inspect records and disk before another action.
Connect/write calls retain their existing two-second ceiling. Output consumers
must drain presentation; this cooperative budget does not impose a process
watchdog on output backpressure or filesystem stalls.

The proposal-binding predecessor review is retained byte-identically. Its low
literal-index finding is addressed by asserting the selected regression route.
Actual-service guided/repair consumer evidence remains same-row backlog.
The prepared-edit predecessor review was pending at session start and arrived
with **Verdict: PASS**, no high/medium finding, after the source gate. Its raw
report is also retained byte-identically. Low L3 (direct prepare root/creation
coverage) and carried L2 join same-row backlog. Current documentation and review
checks include both reports; no Rust input changed. All predecessor commits
remain immutable. No review verdict is edited.

## Verification

The complete ordinary pre-edit affected-package baseline passed **162 tests
across ten targets**, with zero failed, ignored or filtered. The focused guided
workflow check passed **13 methods**. Four added methods cover 16 stop-at-prompt
cases (four approval kinds and timeout/all three signals), 18 stalled-reply
cases (three requests, three response stages, timeout or SIGTERM), six invalid
budgets, piped refusal, successful repair and default-command interruption.
Prompt and socket stop cases assert the exact request prefix and unchanged fixture source;
stalled peers also observe socket closure. Successful cases retain the original
strict receipt checks and existing three-attempt/consent tests.

The new default-command regression failed original production at the first
read-approval prompt: SIGTERM killed the process with status -15, rather than
returning a handled failure with an inspection warning. This uses the existing
command, not an unknown-command rejection. Original production was restored
only for that reserved probe; fixed source was restored and hash-checked in a
finally block. The same assertion passed the focused fixed-code run.
Raw original failure and restoration hashes remain private.

The final ordinary scoped gate is **PASS: 166 tests across ten targets**, zero
failed, ignored or filtered. All 162 prior passing identities remain, plus
exactly the four named additions. Formatting, strict Clippy, documentation,
review-format and whitespace checks pass. Seven commands took **37.37 seconds**
including their individual reservations. Six changed Rust inputs stayed frozen
through the gate; no source edit followed it. Current documents, added-line
privacy, exact scope and unchanged ledger rows are checked again before commit.
Deferred host/Linux T-301.1 is not rerun or waived; this is not a whole-workspace
or native acceptance result.

Raw baseline, focused results, original failure, final gate, hashes and identity
audit remain private. The independent request includes the already installed
Node runtime and its licence through private hard links beside the shared review
helpers, with version and SHA-256 provenance. No runtime is downloaded or
committed; the reviewer must verify availability and preserve any missing-tool
failure.
All scripted model, write and test observations remain synthetic. These tests
cannot establish native executable admission, a real model or project-test run,
GPU use, an independent task grade or a coding score.

## Open work

T-303/T-306 remain open. Native success is blocked by original fixed executable
ownership admission in this lane; no boundary is relaxed. Autonomous endpoint
edit/test composition, actual-service consumer evidence, durable recovery,
approved Git and parent measurements remain open. No evaluation, hidden or
held-out task, scorer, threshold, dependency, SBOM, status model, scoreboard,
release or push change is made.

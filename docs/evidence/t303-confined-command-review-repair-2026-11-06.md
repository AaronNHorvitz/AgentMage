# T-303 and T-306: confined-command review repair

This is the same non-closing command batch, following exact commit
`8742e65697fd9a76b92b72ed3b9c421b32799ee8`. Its
[independent core report](../reviews/8742e65697fd-local-coding-confined-commands.txt)
is retained byte-for-byte. **Verdict: INCONCLUSIVE**: no open high or medium
finding; the reviewer could not execute the required Markdown linter. Both
parent rows remain open. No new task or review-finding row is created.

## Verification handoff

The prior request incorrectly called a builder-private Node directory shared.
The launcher mounts only the shared scratch and review target directories into
the separate reviewer state namespace. The existing Node runtime, its licence
and recorded provenance are now available in that shared scratch mount. No
launcher, permissions, dependency or network change was needed. The private
review request supplies the actual accessible path and binary checksum. The
[development guide](../guides/local-agent-development.md) now describes this
mount check explicitly.

Before editing any tracked file, the builder reran the exact five-file linter
command missing from the original review: exit zero. It used the shared runtime
and bound the five file digests and original commit in a private JSON record.
That builder run cannot substitute for an independent verdict.

## Review follow-up

Low L1 is corrected in the [command guide](../guides/coding-commands.md): the
kernel refuses a fixed set of interpreter names and inline-evaluation flag
forms. Other interpreters and attached short options can pass; the sandbox
and exact person approval remain the execution boundary. No filter is changed.

Low L2 is addressed by exercising valid sealed specifications at all composition
limits, then separately increasing timeout, stdout or stderr by one. Each
outside-limit specification passes the kernel registry and is refused with
`code.command.bounds` by the narrower coding entry point. A separately sealed
empty-scratch specification checks worktree-only scope. The original malformed
specification cases remain. Memory, tasks and CPU ceilings equal the existing
kernel ceilings; this does not claim an independently looser kernel contract.

Low L3 is addressed by serving a synthetic command-shaped approval preview,
including executable identity, exact non-ASCII arguments, environment and limits.
The executable terminal/PTY test parses the displayed consent and compares the
entire preview, checks ASCII-only escaping including bidi and DEL, and preserves
incorrect-consent, successful-consent and replaced-root cases. This is terminal
protocol evidence against a synthetic peer; no native command is launched.

Low L4 (shared authority errors use the repository prefix) and L5 (same-user
post-creation path-open window) remain backlog for T-309 and before parent
acceptance. Prior positive native creation/drift coverage and the native command
sandbox branch remain unmeasured under the unchanged executable-trust refusal.
No product runtime, authorization, sandbox, command parser or runner changes.

## Results

| Check | Observed result |
| --- | --- |
| Original missing five-file Markdown command | PASS through the shared Node runtime, before tracked edits |
| Native-owner focused cases | 13 passed; includes the new valid-sealed-limit case; native trust-refusal branches retained |
| API/terminal/PTY focused cases | 7 passed; synthetic consent peer does not launch a command |
| Selected Rust formatting | PASS; 17.99 seconds including reservation wait |
| Selected all-target Clippy, warnings denied | PASS; 1.55 seconds including reservation wait |
| Full selected source tests | FAIL: 802 passed, 72 failed, 62 existing ignored, zero filtered; 88.53 seconds including reservation wait |
| Failure identity comparison | All 72 equal the original baseline: 53 host and 19 Linux; no new or missing failure |
| Changed Markdown in the full fast gate | PASS: five files; review-format and whitespace checks also pass |

The full gate remains **FAIL**. No threshold, failure, trust check, skip or
filter is altered. The retained review is unchanged; an independent re-review
is still required. Its verdict alone can accept this batch, and neither parent
row closes here. There is no product-runtime change requiring a new coding
campaign. No model, GPU, development-score, hidden-test or held-out run is
claimed. The [scoreboard](coding-scoreboard.md) remains unchanged. Raw builder
logs and source digests are retained in private shared scratch; the final
review request supplies their paths and hashes. The long acceptance gate,
supply-chain regeneration and release gates are not run in this repair.

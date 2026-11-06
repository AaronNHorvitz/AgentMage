# T-303 and T-306: exact confined commands

This non-closing implementation batch follows exact commit
`8d5c8dd8761b5b391b53e2e45e12c8ba1ed25fe6`. Its separate
[worktree-authority review](../reviews/8d5c8dd8761b-local-coding-worktree-authority.txt)
is retained byte-for-byte with **Verdict: PASS** for that batch only. Both
parents remain open. Low findings about canonical destination spelling and
post-success uncertainty diagnostics are corrected in this batch. Positive
native creation and decide-time drift coverage remain required before parent
acceptance; the lane's native trust refusal is unchanged.

## Implementation

The [command guide](../guides/coding-commands.md) describes the three new
control-only API routes and four terminal commands. Preparation accepts a
complete sealed kernel command specification. Execution requires separate
exact typed consent, with the displayed worktree identity verified around
approval. Models and ordinary client tokens cannot authorize commands.

Successful native creation now retains its worktree and management directory
handles and creation receipt identity in this process. Commands require that
provenance; caller-supplied paths or receipts cannot adopt a directory. Restart
loses ownership and pending commands. This is not durable coding-run continuity.

Both worktree creation and commands share the same exact authority composition:
the existing session parent, policy, single-use derived operation, tool registry
and canonical encrypted authority transaction. The command path always uses
`CommandEffectDriver` and `LinuxBoundedCommandExecutor`. Their native manifest,
namespace, seccomp, cgroup and descendant cleanup enforcement are unchanged.
There is no alternate command runner, new store, schema file, dependency or
runtime coordinator. The command request's inline schema is hash-bound in the
registered tool definition; tool validation requires the exact prepared bytes.

This entry point requires the read-only owned-worktree mode and caps commands
at 120 seconds, 4 GiB memory, four CPU equivalents, 64 tasks and 64 KiB retained
bytes per stream. It denies network, credential and publication scopes.
Previews bind the exact executable, arguments, environment, resource limits,
root, creation receipt, unique request, expiry and approval digest. Monotonic
time accounts for verification before consumption. Denial, expiry and attempted
dispatch consume the preview; mismatch preserves it. Repeated decisions cannot
launch twice. Root replacement refuses, including management-root replacement.

Canonical authority receipts survive restart. Command previews and ownership
are process-only; stdout/stderr are response-only byte arrays. Results verify
the existing command receipt and retained byte counts. Output is untrusted and
terminal display is escaped. A successful exit never becomes test-suite success:
`test_suite_verified: false` is explicit. There is no model feedback, automatic
retry, parsed test coverage, live command-output stream or run resume/replay.
The API reports cancellation unavailable after admission, preserving the
runner's timeout/cleanup rather than claiming a connected cancellation token.
No commit, push, deletion or write to the original checkout is authorized.

## Verification scope

Seven new native-owner component tests use real held roots and canonical
encrypted authority with explicitly synthetic platform results. They cover a
failed command followed by a new separately approved successful command,
exact-byte output, mismatch, denial, expiry, command limits, root replacement,
corrupt output, consumed-decision refusal and authority reopen. A synthetic
unwind after durable consumption recovers as uncertain without restoring
ownership, preview or launch authority. This is not a killed native command.

The native command entry point takes the existing manifest trust-refusal branch
in this lane: `linux.command.manifest.invalid`, without a launch. Its synthetic
creation provenance is disclosed in that component test. The positive native
sandbox branch is present for an admitted host but is not measured here.
Actual API and terminal processes exercise control-token protection, strict
request shapes, inability to invent creation ownership, state reopen and
absence of current previews. A separate synthetic API peer exercises the
terminal's exact command consent, decline, one-send behavior and root
replacement after display. That peer never launches a command.

The first component run exposed a fixture-clock problem: monotonic preflight
advanced durable consumption beyond the fixture's reopening clock. Reopening
with a later clock fixes the fixture without changing production recovery.
The first API test sent a body on GET and correctly received the generic HTTP
400 before route authorization. The corrected fixture uses a bodyless GET.
Both failed logs are retained. An initial formatter invocation preceded the
new test module; the final source checks use the complete tree. The lane's
existing shared Node runtime was added to the linter invocation path after
the default path lacked Node. No boundary, test or threshold was weakened.

Final source and documentation results are recorded below. Raw builder records remain in the private lane scratch
directory `tmp/t303-confined-commands/`. No frozen task, hidden test, grader,
prompt, dependency, status field, supply-chain output or release gate changes.
The long acceptance gate remains at T-310. No new model score or held-out run
is claimed; the [scoreboard](coding-scoreboard.md) retains actual model runs.

## Final results

| Check | Observed result |
| --- | --- |
| Rust formatting | PASS; 0.90 seconds including reservation |
| All-target Clippy, warnings denied | PASS; 11.93 seconds including reservation |
| Full selected source tests: host, Linux, local API | FAIL: 801 passed, 72 failed, 62 existing ignored, zero filtered; 1,054.25 seconds including shared reservation wait and compilation |
| Failure identity comparison | All 72 equal the original T-301 baseline: 53 host and 19 Linux; no new or missing failure |
| Native-owner focused cases | Twelve pass, including the seven new command cases; actual native cases retain their exact trust refusals |
| API/CLI/PTY focused cases | Seven pass; synthetic peer outcomes remain protocol evidence only |
| Documentation policy scan | PASS: 759 Markdown files and all policy invariants; not the long acceptance gate |
| Development model campaign | Not run: this batch adds control-only commands, with no connected model feedback or native admission; the last actual scoreboard remains unchanged |
| Changed Markdown | PASS on five named files; deliberately invalid stdin and named scratch-file probes confirm rejection |

The fast gate remains **FAIL**. No existing failure is waived, filtered or
relabeled as native success. The source gate was planned before the documentation
additions; separate Markdown checks cover all five changed Markdown files.
This batch awaits separate independent core assessment and closes neither parent.

The auxiliary binary rebuild and ordinary-serve development-context replay were
queued but never launched. After confirming that the only owned processes were
reservation waiters, the builder cancelled that auxiliary wait. That replay
would repeat unchanged source/proposal/write routes and would not exercise these
new command routes. No current context, replay, inference, grading, token or GPU
measurement is claimed from it. Existing development and held-out scores remain
historical. The connected coding workflow still needs its actual development
campaign and row-closing held-out measurement.

Raw initial failures, final tests, linter probes and the content-free auxiliary
cancellation record are retained. The builder's final publication audit checks
all failure identities, source digests, frozen inputs, ledger statuses, prior
review bytes, privacy and local links. It is not independent review.

## Independent review follow-up

The [exact-commit independent report](../reviews/8742e65697fd-local-coding-confined-commands.txt)
has **Verdict: INCONCLUSIVE**, solely because the supplied Node path was not
mounted in the separate reviewer namespace. It found no open high or medium
issue. The builder's statement above that this runtime was shared was incorrect.
The [same-batch repair](t303-confined-command-review-repair-2026-11-06.md) records
the corrected shared runtime path and test/wording changes. Neither parent row
closes, and this report is retained unchanged.

# T-303 and T-306: approved test feedback and exact reruns

This non-closing batch connects the local API and terminal to the existing kernel
validation normalizer and rerun planner through the existing native command owner.
It adds no task sub-row. T-303 and T-306 remain open for their connected coding
workflow and independent acceptance.

The preceding [independent repair review](../reviews/3a303aed10c3-local-coding-confined-commands-repair.txt)
has **Verdict: PASS** for exact commit
`3a303aed10c3b03b296e0824b70035bb60ccaa0c`, with no open high or medium finding.
It is retained byte-for-byte. Its verdict accepts only that preceding batch;
it cannot approve these changes. Carried low findings about shared error prefixes,
the same-user post-creation identity window and missing positive native coverage
remain before parent acceptance.

## Behavior and boundaries

The [API and CLI guide](../guides/coding-validation.md) defines the exact request,
result envelope, approval and scope. A person selects one immutable command,
test kind and positive minimum count. The owner fixes the existing parser and its
actual implementation digest. The later exact preview consent authorizes one
execution through `CommandEffectDriver`, `LinuxBoundedCommandExecutor` and the
canonical encrypted authority transaction. Definition confirmation alone is inert.
Normal command and validation decisions cannot approve each other's preview.

The unchanged kernel normalizer checks the verified command receipt, captured
bytes, parser semantics and minimum test count. Exit zero with malformed output,
only skipped tests or too few tests never becomes a validation pass. Failure
names and normalized status reach the API and escaped terminal output. The CLI
returns failure when the command succeeds but validation does not pass.

An assertion failure can select the exact last receipt for an unchanged,
separately approved rerun. The existing kernel planner binds the failure, command
and actual new preview approval. The process retains at most three admitted
attempts of one definition per created worktree. Dispatch spends the attempt and
clears the latest result before authority execution. Corruption, uncertainty or
loss of in-process completion blocks further reruns; denial and expiry do not
spend execution attempts. A pass cannot select another failed-test rerun.

This is per-invocation envelope verification, not independent grading. Ordinary
test-runner prose has no new adapter and is malformed. Scope binds a held root
and creation receipt, not source-content identity. No current-workspace or
full-suite pass is claimed. Output and failed names remain untrusted restricted
local data, without a new secret-scanning claim. Raw output is response-only;
the latest normalized receipt and budget are process-only. Restart loses
ownership and cannot resume commands from canonical receipts.

There is no second executor, coordinator, persistence format, dependency or
privileged bypass. Model feedback/automatic repair, owned-edit binding, durable
coding-run recovery, native positive evidence, commit and push remain open.
The existing coordinator and host harness remain the owners for eventual model
retry and recovery integration.

## Verification scope

Native-owner component tests use real held roots and canonical encrypted
authority with explicitly synthetic command outcomes. They exercise failure
feedback then fresh-approved success, prior-receipt and template mismatch, the
three-attempt budget, denial/expiry, cross-route refusal, malformed/zero/few/skipped
tests, contradictory exit state, compilation failure, corrupted output, synthetic
unwind, root replacement and reopening without ownership/result recovery.
Synthetic unwind is not a killed native child demonstration.

The public native validation entry point preserves the actual launcher trust
refusal in this lane, before a launch. The positive native branch, like the
existing command branch, remains unmeasured. Creation provenance in that test is
explicitly synthetic; it cannot establish native worktree creation.

Actual API and CLI processes exercise control-token and explicit definition
requirements, strict object requests, inability to adopt caller paths, refusal
without creation ownership, unchanged canonical receipts and state reopening.
A synthetic HTTP peer exercises exact terminal consent for both command and
validation routes, escaped full-preview fidelity, post-consent root replacement,
one-send behavior and exit-zero/validation-failed CLI status. No peer outcome is
reported as native execution or test evidence.

Raw builder logs are retained in private shared scratch under
`tmp/t303-test-feedback/`; the independent request supplies exact commands and
digests. Initial focused runs passed 18 native-owner and eight API/terminal tests;
the final source gate also includes the subsequently added native-validation
trust-refusal case. Final measurements follow after that gate completes.

No frozen task, hidden test, held-out input, grader, prompt, model, scoreboard,
baseline failure list, dependency, status-model field or supply-chain output
changes. No new model/development or held-out score is claimed. A row-closing
development campaign and held-out scoreboard are still required for the connected
coding workflow. The long acceptance gate remains at T-310. This batch requires
separate independent core review and closes neither parent.

## Final results

| Check | Observed result |
| --- | --- |
| Final focused native-owner cases | 19 passed, zero failed or ignored; native creation, command and validation cases retain the actual trust-refusal branch |
| API/CLI/terminal cases | Eight passed in both the initial focused run and final complete source suite; synthetic peer results remain protocol evidence only |
| Selected Rust formatting check | PASS; 0.95 seconds including reservation |
| Selected all-target Clippy with warnings denied | PASS; 8.37 seconds including reservation |
| Full selected host, Linux and API source tests | FAIL: 809 passed, 72 failed, 62 existing ignored, zero filtered; 94.46 seconds including reservation |
| Complete failure identity comparison | Exactly the original 53 host and 19 Linux baseline failures; no new or missing failure |
| Changed Markdown and retained-review checks | PASS; five Markdown files and the byte-identical preceding independent report |
| Documentation policy scan | PASS: 762 Markdown files and all policy invariants; not the long acceptance gate |
| Development/model or held-out campaign | Not run; this batch has no connected model feedback/repair and cannot establish a coding score |

The complete fast gate remains **FAIL**. No trust check, test, threshold,
failure or existing ignore is changed, skipped or allowlisted. The initial full
run also retained the same failure set, but additionally found a Clippy Boolean
simplification and one extra Markdown blank line. Both were corrected before
the final complete gate. The initial and final logs are retained. Final reserved
commands waited for the shared slot; no alternate reservation or executor was
used. The builder audit verifies source digests, original failure identities,
unchanged frozen inputs and ledger statuses, prior-review bytes, privacy and
links. It is not an independent review. Final prose checks cover this results
table after the source gate; no source change follows that gate.

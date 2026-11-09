# T-303/T-306 terminal endpoint tool-owner return

Date: 2026-11-08. Parent rows remain open. Independent core review is required;
no row or sub-row is added or closed. This record describes process-local
library continuation, not durable recovery or a complete coding workflow.

## Change and authority

[Decision 0170](../decisions/0170-terminal-endpoint-tool-owner-return.md) adds
consuming return of the original endpoint tool boundary after terminal
publication. The retained registry binds the exact original sealed request and
keeps the released run identity spent. It refuses live approval waits and
faults, retaining the entry. The kernel also refuses uncertainty, suspension,
pending approval, missing or disagreeing outcomes and incomplete terminal
publication, returning the unchanged coordinator without entering dependencies.

For prepared native effects, `into_repository` delegates to the original edit
or command abandonment method. It preserves the actual live creation owner,
successful edit lineage, validation history and attempt ceiling. It abandons
pending previews and grant handles; issued grants remain recorded until normal
expiry. A later operation needs fresh preparation and exact approval. Returning
the owner performs no command, file write, retry, reconciliation or cleanup.

The old coordinator, context, endpoint connection, verifier and budget ledger
are discarded on successful return. Export events first. The tool boundary can
retain its original continuous control and shared clock until abandonment.
Nothing renews either budget or stopped run. This is a trusted in-process
operation with no new HTTP, CLI or MCP route or serialized authority.

The predecessor's [independent review](../reviews/5d4250ae6124-local-coding-prepared-startup.txt)
is retained byte-identically, with **Verdict: PASS** for exact commit
`5d4250ae61241565d3567b391f3dd7b054ed107f`, no high/medium finding. Its low L2
is corrected: native mutable/protected path checks apply to edits; commands
check their original creation/preview and resource ceilings. Low L1, unexecuted
positive prepared-factory binding/admission, remains backlog. The cancellation
predecessor was in flight at session start; no ancestor was rewritten.
Its [independent review](../reviews/a7a532057fdf-local-coding-original-cancellation.txt)
arrived during implementation with **Verdict: PASS** for exact commit
`a7a532057fdf9ddd3b23883793ce136793e0f7d2`, no high/medium finding. Low L1,
unconsumed-preview replacement isolation coverage, remains backlog. Low L2 is
clarified in the command guide: a pre-cancelled direct validation spends an
attempt, unlike endpoint cancellation observed before native preflight.

## Verification

The new kernel cases check unchanged refusal before a run and during approval,
actual boundary identity after terminal return, no additional dispatch, known
denial and model failure, uncertain tool outcome and incomplete or faulted
terminal publication. The host case reuses one actual boundary in two separately
admitted runs over real loopback sockets with synthetic endpoint responses,
requires a fresh challenge and rejects the previous response and spent run IDs.
Existing error/unwind cases also check return refusal and retained events.

The native edit coordinator case now returns its original owner after both
denial and a committed write followed by upstream refusal. Successful lineage
can prepare a separate original-checkout diff; that checkout remains unchanged.
The native command coordinator case retains actual validation attempts and
receipt state after return and abandons the pending preview. Creation provenance
and command observations remain synthetic; original atomic writes, authority
transactions and parsing are real. No complete native Git creation, child-stop
or real-model task result is claimed.

The first focused development run had two passes and one failure: the synthetic
uncertain read incorrectly carried trusted evidence. The existing execution
validator refused it. Correcting only that fixture's evidence produced 14
focused passes across four commands: three kernel, nine host and one each in
the native edit and command coordinator targets. These development runs used
filters; they do not replace the complete gate below. No validator, threshold
or production execution rule changed.

The final ordinary gate ran
`python3 scripts/local_agent_fast_gate.py --base a7a532057fdf9ddd3b23883793ce136793e0f7d2 --timeout 600`.
It selected the host, kernel engine, local API and Linux crates, with their
complete default-feature unit, integration, binary and example test targets.

| Check | Exit | Observation |
| --- | --- | --- |
| Diff/untracked/review checks and Markdown lint | 0 each | Five Markdown files; no lint issues; retained reports unchanged |
| Ordinary scoped formatting | 124 | Timed out after 600.12 seconds waiting for the shared reservation; did not execute |
| Separate identical scoped formatting retry | 0 | Same plan arguments and 600-second limit; 367.53 seconds including reservation wait |
| Scoped Clippy, all targets, warnings denied | 0 | 107.94 seconds including reservation wait |
| Scoped tests, all targets, no fail fast | 101 | 410.24 seconds including reservation wait; 45 targets |
| Ordinary gate | 1 | FAIL; 1120.02 seconds including reservations |

Test totals: **2,770 passed, 72 failed, 71 existing ignored, zero measured,
zero filtered**. The 72 failing identities exactly match the unchanged T-301
baseline: 53 host and 19 Linux, none new or missing. Kernel coverage adds eight
existing ignored tests to the predecessor's scope; no ignore annotation was
added or changed. All four new methods and four strengthened methods pass in
this unfiltered run. All seven Rust source hashes stayed unchanged throughout.

The data audit passes for scope, original source hashes, unchanged ledger rows,
privacy, exact failure identities and the separate identical formatting result.
It reports the ordinary formatting timeout and separate retry separately.
It is **not a passing gate**, failure waiver or acceptance result. The ordinary
timeout remains a failure; tests remain failed. Final documentation and retained
reviews receive the lightweight checks again after this record is completed.
Raw attempts, command plans, results, source hashes and failure identities are
retained privately for independent reproduction. The owned temporary namespace
retains 22 entries, including four inherited host fixture files totaling 36
bytes; it was not empty and was not removed. New fixtures were cleaned up.

## Remaining work

Executable controlled startup, general strict model-driven edit/test/retry,
positive prepared-factory admission, service restart recovery, approved commit
and push, independent task grading and parent measurements remain open.
Existing low-priority findings remain same-row backlog. Native trust and the
deferred T-301.1 owner-action blocker are unchanged. No model download, GPU use,
hidden or held-out test, suite task, scorer, threshold, dependency, supply-chain
output, status model, coding score, release or push is changed. No owner-ready
or external-only declaration is made.

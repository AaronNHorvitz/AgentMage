# Decision 0065: Coding Daily-Use Verification Thresholds

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-22 |
| Authority | Decision 0054 and Task 50.2.4.6 |
| Scope | Pinned-Linux-profile daily-use verification for the standalone coding harness |
| Preserves | Exact activation, model admission, native tools, protected approvals, confinement, canonical stores, verifier ownership, evidence distinctions and independent review |

## Context

Task 50.2.4.6 requires declared thresholds for setup, restart, contention, pressure,
slow clients, model failure and a long-session soak. The architecture requires those
thresholds to exist before results are observed and forbids lowering them to turn a
failure into a pass. Existing component limits remain authoritative; this decision
selects an integration campaign and does not widen any runtime budget.

## Decision

1. Every build or test starts with at least 16 GiB available RAM and runs with at most
   four Cargo jobs in the existing 5 GiB `MemoryHigh`, 6 GiB `MemoryMax` and 512 MiB
   swap scope. Candidate models additionally use one slot, four server threads, a
   200% CPU quota, low scheduling and I/O priority, a 45-minute deadline, at least
   21,000 MiB free VRAM at start and a 22,528 MiB maximum sampled total GPU use.
2. Repeatable setup passes only when at least three fresh private fixtures activate,
   diagnose and run without inherited state. The executable matrix must remain fully
   passing; one favorable run cannot replace a failing case.
3. Restart passes only at a canonical safe checkpoint. A fresh host must resume the
   exact request without replaying a prior tool; source, model, policy, configuration
   or worktree drift must refuse before another effect.
4. Worktree contention passes when a second start against the live owned fixture is
   refused within five seconds, the original owner remains cancellable within thirty
   seconds, and no delayed write or unrelated-process termination occurs.
5. Disk pressure uses a predeclared 1,024-byte run disk budget, equal to the request's
   declared maximum single output and smaller than the canonical request artifact. It
   must fail before a model/tool effect and produce no terminal success claim. Output
   pressure uses a predeclared 1,024-byte cumulative output budget and a matching maximum
   single output. The clean Git observation and completion each fit that single-output
   limit, while their aggregate does not; the run must terminate as exhausted without
   presenting truncated bytes as complete output.
6. The slow-client case must disconnect a deliberately lagging subscriber while the
   independent canonical event pump completes, and cancellation must remain observable
   within thirty seconds. The existing cursor, queue and event ceilings are unchanged.
7. The long-session soak is twenty-five follow-up runs after one initial run in one
   host-owned session. Every run must have a distinct CSPRNG run identity, a verified
   terminal `NO_OP`, complete artifact verification and a clean worktree. The campaign
   must complete within fifteen minutes and retain its raw log and measured elapsed time.
8. An exact candidate-model codec, runtime or resource failure passes the *failure
   handling* case only when raw output and measurements are retained, the worktree is
   unchanged and no fallback occurs. It does not pass model qualification or the
   required failed-test/correction campaign.
9. Executable scripted support in this increment is Python source plus the exact
   registered `fixture.python-validation@1.0.0` command. The structured planner has
   component coverage for Rust, Python, TypeScript, TSX, JavaScript and Swift syntax
   edits and Go, shell, SQL and plain-text exact edits, but those languages are not
   claimed as executable daily-use acceptance. Arbitrary shell, package management,
   network commands and wildcard command templates remain unsupported.
10. Passing this campaign may close only Task 50.2.4.6's local verification work. It
    cannot supply Task 50.2.4.7 independent review, record `M-HARNESS-DAILY`, qualify a
    model, broaden platform support or establish a release disposition.

## Consequences

Pressure failures are intentional, content-minimized results rather than demonstrations
of a successful coding task. The soak measures the connected actual-process path without
inflating authority or inventing an alternate store. Any missed threshold remains visible
and must not be repaired by changing the threshold after the run.

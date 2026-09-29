# Decision 0110: Context Inspection, Job Control and Run Progress

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0081, 0088 and 0107; the implementation amendment; current owner restart |
| Scope | AMR-04 decomposition, CAP-28 inspectable context budgeting, CAP-35 durable background jobs and CAP-39 truthful progress |

## Findings

The coding runtime composes each model context with the deterministic composer. When
a measured packet is too large, it recomposes with a smaller budget. It tells the model
which sources were omitted, but nothing gives a person the same view. Nothing checks
that a recomposition keeps the task's constraints either. A non-essential correction
could be dropped in a reflow and the run would continue.

The existing job scheduler contracts cover leases and schedules. They have no
reconciliation of control requests that arrive from several clients or after a client
reconnects. The development harness stops a run by signalling its process, with no
retry identity, revision check or record of the decision.

Clients can inspect a run's events, but nothing states in one place whether a run is
queued, running, waiting for approval, stopping or ended. Nothing separates the
runtime's own verification from independent review and delivery, which the runtime
cannot observe. Counts are shown without saying whether their total is known.

## Decision

Split AMR-04 further. AMR-04.4, AMR-04.5 and AMR-04.8 are kernel components. AMR-04.6
and AMR-04.7 are the integrations of CAP-35 and CAP-13. Each integration needs its own
actual-process proof. Record all five identities in the work selector's accepted set.

For CAP-28, a context inspection reads one verified composed packet. It lists
retained constraints as pinned, other included sources as selected, and omitted
sources with their reasons. It also gives token and byte use, per-class totals and
the token counter. It carries identities and counts, never excerpts. Rendered text
escapes every identity and is capped at 64 KiB. A retained constraint is any
essential item, or any instruction, newest request, correction, approval, active
objective, blocker or expected output.

A sealed recomposition check compares two compositions of the same session state. It
reports three violations: a retained constraint that is no longer included, a source
missing from the later accounting, and an item identity that now names different
material. It also reports whether the token counter changed and which other sources
were newly omitted. The coding runtime runs this check on every reflow. A dropped
retained constraint ends the build as resource exhaustion rather than continuing
without it, and any other violation is refused as invalid. As amended by
[Decision 0113](0113-review-fixes-for-inspection-and-job-control.md) (finding V3),
the builder also refuses the first composition when it omits a retained constraint
for budget. The check does not judge a
later turn whose request legitimately changed.

For CAP-35, a job control ledger has one owner per job. A client request names its
own request identity, the action and the job revision it observed. The owner applies
each request identity once and answers a retry with the original decision. It
refuses a reused identity with other content, a request built on a stale revision, a
control that is already in effect, and any control of a terminal job.

Suspending or cancelling running work only requests it. The job enters `Suspending` or
`Cancelling`, and only the owner's observation at a safe boundary completes the
change. Work that never started is suspended or cancelled at once. If work finishes
before a pending request takes effect, the finished state stands and the ledger keeps
the request. Every decision and owner event is an entry in a hash chain, and a
restarted owner must replay the retained chain exactly. The ledger grants no authority
and performs no effect. As amended by
[Decision 0113](0113-review-fixes-for-inspection-and-job-control.md) (findings V1 and
V4), the owner keeps a reserve of entries that clients cannot use, refusals have their
own bound, and replay must end at the retained head. The unkeyed chain proves
consistency, not origin or completeness.

For CAP-39, a run progress projection first verifies the complete event chain. It
then reports the run as not started, running, waiting for a decision, stopping after
a cancellation request, or ended in its exact terminal state. It says the run was
verified locally only for a success terminal, which the runtime's verifier must
accept. It always reports independent review and delivery as not established by the
runtime. Once cancellation is requested, no decision is reported as awaited
([Decision 0113](0113-review-fixes-for-inspection-and-job-control.md), finding V6). Turn, model-call and tool-call counts show a denominator and percentage
only when the owner supplies a declared ceiling. Otherwise the text says the total is
unknown.

## Verification boundary

Engine unit tests cover inspection partitions, omission reasons, escaping, render
bounds, reflow and counter change, each violation class and tampered packets. They
cover the ledger's cancellation, suspension, late completion, conflicts, replay
tampering and entry bound. They cover the progress projection's activities, its
verification claim, declared and unknown scopes, and refusal of tampered streams. A
host unit test proves that the coding reflow omits a supporting source but refuses to
drop a correction, and that test failed without the check. No CLI, host view or
durable store uses the inspection, the ledger or the progress projection yet.
Persistence, IPC control, detach and reconnect, and actual-process proof remain
AMR-04.6.

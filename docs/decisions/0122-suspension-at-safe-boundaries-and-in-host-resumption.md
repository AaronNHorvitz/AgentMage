# Decision 0122: Suspension at Safe Boundaries and In-Host Resumption

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088, 0110, 0118, 0120 and 0121; current owner restart |
| Scope | AMR-04.6.4: stop a run at its next safe boundary for an applied suspension and continue it inside a running host through a cursor-bound resumed run |

## Findings

Decision 0120 left suspension and resumption for this row. The ledger already
decides them: a suspension moves a running job to suspending, only the owner's
observation at a safe boundary makes it suspended, and a resumption queues a
suspended job again or withdraws a pending suspension. Four facts shaped this
decision.

- The runtime commits a checkpoint at the end of each completed turn when the
  run has checkpoint ports. A resumed run is a new composition whose request is
  bound to the event cursor of such a checkpoint. It restores the run's state
  from the checkpoint and the journal and verifies both against the cursor.
- The runtime can stop only for a cancellation or a fault. It has no step that
  stops a run and keeps it resumable.
- The development host opens its operational store once per composition, and the
  store admits one connection at a time.
- The composition holds the run's model port, tools and store. Holding it while
  a run waits for a person keeps those resources for nothing.

## Decision

1. Engine. A coordinator consults a suspension probe only right after a
   checkpoint commits, while the commit event is still the run's last event, and
   only when no cancellation is observed there: a cancellation at the same
   boundary wins. When the probe answers yes, the run stops and the coordinator
   answers the boundary: the checkpoint's identity and digest and the cursor of
   its commit event. The coordinator then refuses to advance. A coordinator
   without checkpoint ports commits no boundary, so it never stops for a
   suspension. A pending approval is not a safe boundary. The existing entry
   point is unchanged; a new entry point takes the probe.
2. Owner. When the ledger applies a suspension, the host lets the run's worker
   stop at its next committed boundary. When the worker stops there, the host
   waits until the commit event is visible, records the owner's observation that
   the job is suspended, ends the worker and releases the composition. The job's
   ledger handle stays open, so the suspended job can still be controlled.
3. No stop without a request. The worker stops only for a suspension the ledger
   applied and the host has not withdrawn. While the host decides a resumption or
   cancellation, the worker may not stop for the suspension. If the worker has
   already committed to stopping, the host records that stop first and decides
   the request afterwards. The host acts on the job's phase after each decision,
   never on a decision alone, so a retry that answers an old decision changes
   nothing that has since moved on.
4. Refusals. A run whose coordinator commits no checkpoints refuses suspension
   and resumption before the ledger, so they are not recorded. A suspended run
   cannot be released, declared or advanced, and a direct cancellation of it is
   refused as for every ledger-kept job (Decision 0120).
5. Continuation. When the ledger queues a suspended job again, or cancels it,
   the host continues the run inside the running host before it answers:
   - it closes the held ledger handle;
   - the factory prepares the same run bound to the boundary's cursor. The
     development factory reads the session's current checkpoint, its resume
     binding and the stored base request, checks them against its frozen
     composition, and requires the journal to end at that cursor;
   - the host checks that request against the one it derives itself, composes
     it, and requires the restored history to equal the events it already
     presented;
   - it starts a resumed job again. A cancelled job keeps its terminal decision:
     the continued run observes the cancellation at once and ends with its own
     cancelled outcome, which is not recorded again.

   If any step fails, the run is no longer held and the job keeps the phase the
   ledger decided. A queued job can then be started again by a run resumed after
   a host restart.
6. Resumed request. The continued run's request is the run's base request bound
   to the boundary's cursor. The host and the client derive it separately. The
   client follows the run under it only after the host answers a resumption or
   cancellation of the suspended job with a status that describes exactly that
   request. The continued run declares its recoverability and context views as
   unavailable, as every resumed run does (Decision 0117).
7. Wire. The IPC wire version becomes 8. A step names the boundary a suspended
   run stopped at. The field is absent from every other step, so their encoding
   is unchanged, and its contents are a closed record.
8. Client. The shared CLI driver sends suspension and resumption as job control
   requests. They follow the same status, request and bounded stale-retry rules
   as cancellation. A host that does not take a suspension or resumption leaves
   the run unchanged, and the driver shows that. The driver verifies that a
   suspended step's boundary is the last verified event. While the run is
   suspended, the driver waits for a person's resumption or cancellation without
   asking the host again.
9. Development CLI. SIGUSR1 asks to suspend the run and SIGUSR2 asks to resume
   it; SIGINT and SIGTERM still cancel. The wrapper's `pause` and `resume`
   commands send those signals to the exact running CLI, as `stop` does. Each
   answer, the suspension and the continuation are shown on standard error as
   they happen, in both output formats. The job state after the run lists every
   control answer with its action. A development probe asks for one suspension
   at the first control point and a resumption once the run is suspended. The
   scripted acceptance matrix gains a case that uses it.

A suspended job whose host ends stays suspended in the ledger. A run resumed
after a host restart does not continue it, because only a client resumption may
(Decision 0120). Reconnecting a client to such a job is AMR-04.6.3.

## Verification boundary

Engine tests use durable coordinators with the engine's fixture ports. They
cover:

- a stop at the first committed boundary and the refusal to advance afterwards;
- a resumed composition from the boundary's cursor that completes without
  repeating the effect;
- no consultation without checkpoint ports, before the first turn, at a pending
  approval or after completion;
- a cancellation at the same boundary winning.

Host tests use a real encrypted store and a coordinator that stands in for a
durable one. They cover:

- the stop, the owner's observation and the released composition;
- the refusals while suspended;
- the resumption through the factory while the store is closed;
- the continuation under the resumed request to a completed job;
- a cancellation that arrives after the worker committed to stopping, so it is
  decided after the stop and ends the run as cancelled;
- a resumption or cancellation before the boundary, so the worker never stops.

The wire test covers the step's boundary, the unchanged encoding of other steps
and the closed record. CLI tests cover the driver's suspension, resumption and
cancellation under the derived request, a host that does not take the request,
evidence refusals for a foreign boundary or answer, and every notice and view in
both formats. Single mutations of the rules above each made the expected new test
fail; the results are in the batch record.

The development factory's continuation and the acceptance case run only on a
native Linux host with the confined development tools, so they are not verified
in this lane. No test uses a model or the GPU. Detach, reconnect and cleanup
with actual processes remain AMR-04.6.3. Independent review of this batch is
requested and remains open.

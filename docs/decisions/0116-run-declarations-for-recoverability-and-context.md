# Decision 0116: Run Declarations for Recoverability and Context Views

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0081, 0088, 0108, 0110, 0112 and 0113; current owner restart |
| Scope | AMR-04.7.1 and AMR-04.4.1: show a run's recoverability declaration and context views through the development CLI |

## Findings

Decision 0108 built a recoverability declaration, but nothing produced its effect list.
Review finding R8 (Decision 0109) requires that list to come from canonical records
that the runtime owns, never from client input. Decision 0110 built a content-free
context inspection, but no view showed it. Four facts shaped this decision.

- The CLI cannot build the effect list. Runtime events do not name the tool of each
  call, so the CLI cannot tell a command from a read.
- The Linux coding boundary executes every native call and publishes every change
  record. It is therefore the owner of each run's effects.
- The coding context port composes every context a model receives, but keeps no
  record of the composition once the packet is returned.
- The development CLI reads the ended run's artifacts, then releases the run. The
  only per-run payload on the wire is the transport step.

## Decision

1. Effect record. For each run, the Linux boundary keeps a record of the calls it
   executes and the change records it publishes. A call enters the record once the
   boundary takes its issued grant to execute it. For each call it notes one of four
   kinds: write, create (with its path), command, or no effect. Validation counts as a
   command, and reads, Git inspection and history reads have no effect. It also notes
   how the call
   ended: its outcome and whether state changed, or unknown when the call returned
   an error after that point. A change record is kept only if it decodes and
   verifies. If an execution or a record cannot be kept, the whole record is marked
   incomplete. An incomplete record is never declared, so a declaration cannot
   silently leave out an effect.
2. Effect list. A pure function maps the record to Decision 0108 effects:
   - A write that succeeded and changed state becomes its change record. If no
     record was published for it, it becomes uncertain.
   - A create that succeeded becomes a create of its path.
   - A command becomes a command unless it was denied. A failed, cancelled or
     timed-out command may still have run.
   - Any write, create or command that ended uncertain or unknown becomes uncertain.
   - A published record that belongs to no changed write refuses the list.
3. Run scope. The declaration names its run and says it covers only that run's
   effects. A session-wide, durable declaration is AMR-04.7.2. The boundary assesses
   each write against the file's current bytes, read only through the held worktree.
4. Context views. The context port keeps the content-free inspection of every packet
   it returns, in order, up to 64 per run. A refused composition keeps none. If a view
   cannot be kept, the list is marked incomplete and is never shown as complete.
5. Access. The coordinator exposes its tool boundary and context port read-only, and
   only once the run has its canonical outcome. Nothing can read them beside the loop
   while the run can still advance.
6. Wire. The transport gains a `run_declarations` request for one ended, unreleased
   run. It returns:
   - the run identity and request digest;
   - the recoverability declaration, or nothing if it cannot be declared completely;
   - the context views, or nothing if not every view was kept.

   The Linux IPC wire version becomes 6. An oversized answer is refused with a
   capacity error rather than ending the host's service. A transport without the
   request refuses it.
7. CLI. Before it releases a run, the CLI reads the declarations. It keeps them only
   if they name that exact run and request. It drops a recoverability declaration
   whose seal, run scope, closed reason codes or summary fields do not verify. After
   the run progress on standard error, it prints the declaration and then each
   context view; any part that is missing is shown as unavailable. JSON output
   prints one `run_declarations` object with explicit availability flags. Standard
   output still ends with the outcome.

A declaration grants nothing. Every inverse write it lists still needs its own fresh
approval through the rollback tool. The client can verify a declaration's integrity
and consistency, but not that the host listed every effect; completeness rests on the
effect owner, as Decision 0108 requires.

## Verification boundary

Tests cover:

- the effect list and the recorder;
- declaration verification, including resealed mutations;
- context view retention;
- the wire contract;
- both CLI output formats.

The Linux boundary glue and the actual processes run only on a native host.
AMR-04.7.2 and AMR-04.10 track the native display proofs and the session scope.
Independent review of this batch is requested and remains open.

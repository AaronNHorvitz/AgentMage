# Decision 0120: Host Job Control over Authenticated IPC

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088, 0110, 0113, 0116, 0118 and 0119; current owner restart |
| Scope | AMR-04.6.2: the development host's job status and cancellation through the durable job ledger over authenticated IPC; splits suspension and resumption into AMR-04.6.4 |

## Findings

Decision 0118 made the job control ledger durable in the operational store. Nothing
used it yet. The development host stopped a run when a client sent a direct
cancellation, and it kept no record of who asked or what they had seen. Four facts
shaped this decision.

- The development host opens the operational store once per run, when it composes
  the runtime. It then moves the store into the run's tool boundary on the worker
  thread. A job ledger handle shares the store's lock and must be taken before that
  move.
- The authenticated IPC endpoint records each peer's user, process, process start
  time and executable digest. The IPC service never read them.
- The runtime can stop a run at a safe boundary only through cancellation or a
  fault probe. It has no step that pauses a run and keeps it resumable. A resumed
  run is a new composition from an event cursor, which the host prepares only when
  it was launched to resume.
- A run resumed after a host restart keeps its run identity.

Suspending and resuming therefore need an engine step, a transport state and a
resume path inside a running host. That is a separate package of work.

## Decision

1. Rows. AMR-04.6.2 covers job status and cancellation. AMR-04.6.4 covers
   suspension at a safe boundary and resumption through a cursor-bound resumed run.
   AMR-04.6 closes only when AMR-04.6.2, AMR-04.6.3 and AMR-04.6.4 have closed.
2. Jobs. Each run is one job, and the job identity is the run identity. The owner
   identity is `agentmage-coding-host`. The store authenticates writers by its key,
   so every host over one store is the same owner (Decision 0119).
3. Handover. After it composes a run, the host factory hands the host service the
   job ledgers of the store that the run opened. A factory without them hands over
   nothing, and its runs have no job control. The service holds the handle only
   while it holds the run.
4. Start. Before any work is dispatched, the service creates the job and records
   the owner's start. A run resumed after a host restart continues its job if the
   job is running. The service refuses to start the run in four cases: a new run
   whose job already exists, and a resumed run whose job is suspended, has an
   applied cancellation, or has ended.
5. Outcome. When the run's verified outcome arrives, the service records it as the
   job's terminal observation:
   - completed for a success or no-op, even when a request is pending;
   - cancellation observed for a cancelled run whose cancellation the ledger applied;
   - failed for every other outcome, including a cancelled run without an applied
     request.

   If an owner observation cannot be recorded, the service stops showing and
   controlling that job, because the ledger no longer follows the run.
6. Client scope. The IPC service derives one scope per authenticated connection. The
   scope is `peer-` followed by 32 hexadecimal digits of a SHA-256 digest over the
   peer's user, process, process start time and executable digest. Each control
   request is recorded under that scope. A client never names a scope. The wire has
   no field for one, and the closed contract refuses unknown fields. A restarted
   client process is a new client, so its retries of earlier requests are new
   requests.
7. Wire. The IPC wire version becomes 7. It gains a job status operation and a job
   control operation, and it loses the direct cancellation operation. Over this
   channel a run is cancelled only through the ledger. The host answers a job
   control request only through the owner path, which is given the scope. In-process
   callers have no authenticated scope, so they get no job control.
8. Decisions. The service decides each request through the ledger. It refuses
   suspension and resumption before the ledger until AMR-04.6.4, so they are not
   recorded. Only an applied cancellation stops work. The service sets the run's
   cancellation signal, and the run then ends at its next cancellation check. A
   retry answers the original decision and sets the same signal. The service
   refuses a direct cancellation of a run whose job the ledger keeps.
9. Client. The shared CLI driver cancels by reading the job's status and sending a
   cancellation that names the revision it observed. A stale refusal is observed
   again and sent under a new request identity, at most four times. Only a
   transport that offers no job control is cancelled directly. Before it releases
   an ended run, the driver reads the job's reconciled state. It rejects any status
   or answer that does not describe the exact run.
10. View. After each run, the development CLI shows each control answer and the
    job's reconciled state on standard error, in both output formats. A run whose
    host kept no job ledger shows the state as unavailable.

A run that the service drops without an outcome keeps its job running in the
ledger. This happens after a start failure or a closed connection. Cleanup across UI
closure and a reconnecting client are AMR-04.6.3.

## Verification boundary

Host tests use real encrypted stores and cover:

- the service's start, outcome and control rules;
- per-client retries and conflicts;
- refusals of suspension, resumption, direct cancellation and in-process control;
- durability after the store is reopened;
- the start rules for resumed runs;
- the outcome mapping;
- the wire's closed operations and scope derivation;
- the driver's cancellation, stale retry, evidence refusal and direct fallback;
- the CLI view in both formats.

Eleven single mutations each made exactly the expected new test fail. Each one did
one of the following:

- let suspension reach the ledger;
- allowed a direct cancellation of a ledger-kept job;
- continued a resumed job with a pending cancellation;
- let a new run reuse an existing job;
- reported any cancelled run as an observed cancellation;
- dropped the outcome observation;
- left the process out of the client scope;
- decided a control request without the derived scope;
- stopped retrying after a stale refusal;
- fell back to direct cancellation on any job error;
- accepted a status for another request digest.

No test uses a native host, a model or the GPU. Detach, reconnect and cleanup with
actual processes remain AMR-04.6.3. Independent review of this batch is requested
and remains open.

# Read-Only Jobs and Schedules

Jobs move through pending, running, succeeded, failed, cancelled, waiting-approval, and dead-letter
states. A running job has one unique current lease owner with explicit acquisition, renewal,
expiration, cancellation, and recovery boundaries. Every job binds a workspace, task, idempotency
key, recorded reason, expiry, completed-operation history, and independent model, tool, process,
memory, network, retry, and retained-output ceilings.

Sleep, wake, offline, missed-run, unavailable-workspace, expiry, cancellation, and cleanup are
explicit states. Resume checks durable completed-operation identities and refuses a repeat. Terminal
work clears the lease, terminates descendants, releases resources, and creates a content-free local
notification for completed, failed, blocked, approval-waiting, or resource-constrained work.

Schedules are inspectable and support dry run, manual test, pause, resume, edit, run now, and delete
as separate controls. Initial scheduled operations are read-only local or narrowly granted remote
reads. File writes, generic shell, connector writes, publication, remote state changes, and
interactive approval requests are denied. At most one missed run may be selected, preventing a
catch-up storm.

Post-run receipts bind lease, idempotency, grants, operations, network, resources, outputs,
failures, retries, cleanup, completed operations, descendant termination, and resource release. The
current module is a pure state machine with no clock, persistence, notification, process, network,
or scheduling executor. Native multi-runner crash, sleep/wake, notification, persistence, and
independent-review evidence remains required.

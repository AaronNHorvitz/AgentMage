# Scheduled Authority

Prior user configuration is not live action approval. Every unattended state-changing schedule has
its own threat model and allowlist entry, plus an exact activation approval distinct from the saved
configuration. Only append-to-owned-log, constrained owned-task update, and ordinary fast-forward
dedicated-task-branch update are predeclarable. Messaging, arbitrary shell, destructive or force
operations, variable destination or payload, and work requiring live judgment remain unschedulable.

Each grant binds the task template, workspace, dedicated worktree, file ownership, model, tools,
destinations, payload constraints, budgets, expiry, stop conditions, dry-run evidence, activation
preview, approval, and idempotency key. Schedule, workspace, worktree, policy, destination, payload
constraint, and precondition state are re-read immediately before execution. Any drift stops.

Pause, revoke, expiry, inspect, and emergency stop are model-independent control states. A schedule
cannot create, edit, activate, renew, or broaden itself. Unknown external effects block retry until
fresh reconciliation. The current boundary creates no schedule and performs no effect; it has no
clock, persistence, worktree, filesystem, process, network, or emergency-stop executor. Native
activation, effect, crash, cancellation, reconciliation, and independent-review evidence remains
required.

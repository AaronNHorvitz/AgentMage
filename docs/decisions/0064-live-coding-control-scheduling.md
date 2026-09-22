# Decision 0064: Live Coding Control Scheduling

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-21 |
| Authority | Decision 0054 and Tasks 48.2.5.1-48.2.5.3 |
| Scope | Scheduling of one admitted coding coordinator behind authenticated local IPC |
| Preserves | Coordinator ownership, canonical events, protected approvals, exact grants, native tools, cancellation truth, bounded queues and evidence verification |

## Context

The reusable coordinator already publishes sealed canonical events to bounded nonblocking
subscribers and accepts a cancellation probe throughout model and tool execution. Calling that
coordinator directly inside the authenticated IPC request loop nevertheless blocks control
servicing during inference or native tool work. The terminal cannot receive progress or deliver a
signal while that call owns the transport thread.

Adding a second execution loop, letting the terminal invoke tools, manufacturing presentation-only
events, or treating process termination as successful cancellation would violate the accepted
architecture. The scheduling layer must expose the coordinator's own stream without taking its
authority.

## Decision

1. One host-owned worker thread owns and advances the admitted coordinator. The authenticated IPC
   thread owns only bounded control messages, canonical event delivery, exact cursor validation,
   protected approval responses and cancellation requests.
2. Before execution, the host registers one coordinator event subscription sized by the existing
   count-and-byte hardening ceilings. It never increases run limits. A lagged or disconnected
   nonterminal subscription fails closed; the client reconnect path may use only an exact retained
   cursor once durable journals are integrated.
3. A transport step may be nonterminal with neither approval nor outcome. Such a step contains only
   a verified progress page (or a bounded empty poll after the stream has started), never effect
   authority. The first step must contain the canonical run start. The terminal independently
   verifies every page and polls with a fixed delay.
4. Artifact-created events remain withheld from presentation until the coordinator supplies the
   matching verified artifact references. The scheduling layer cannot create, read or substitute
   artifacts.
5. SIGINT and SIGTERM set a process-local flag through a safe signal-registration dependency. The
   terminal converts an observed flag into one exact shell-originated cancellation identity. The
   host sets the coordinator's shared cancellation probe; it does not kill the worker or report
   completion from the request alone.
6. Cancellation is complete only after the model/tool boundary returns, the coordinator emits
   cancellation-requested and cancellation-observed events, descendants are reconciled where
   applicable, and the canonical outcome is `CANCELLED`. A late terminal result remains truthful
   and cannot be overwritten by the request.
7. The development operator wrapper records an exact CLI PID, executable and root-bound argument
   identity. `stop` refuses an absent, substituted, reused, or still-starting process and signals
   only the validated CLI. It never scans or terminates unrelated processes.
8. This decision does not authorize a model, persistent preauthorization, production activation,
   platform support, release, or independent-review claim.

## Consequences

Progress becomes visible before a long model call ends, and the IPC owner retains reserved control
capacity for cancellation. The same mechanism remains compatible with the existing coordinator,
event verifier and eventual journal-backed replay; it does not introduce another store or engine.
Actual-process acceptance must retain success, denial, race, slow-consumer, cursor and cancellation
logs separately.

# Sprint 41 Local Verification Results

| Field | Result |
|---|---|
| Exact `CommandSpec` and registry | Pass locally |
| One-use `CommandExecute` authority path | Pass locally |
| Direct no-shell Linux execution | Pass on current Fedora host |
| Literal output and exact executable identity | Pass on current Fedora host |
| Timeout and propagated cancellation cleanup | Pass on current Fedora host |
| Fixed command-injection corpus | Pass locally |
| Closed preview and receipt schemas | Pass locally |
| Production command profile registration | Disabled |
| Peak CPU, memory, and task-use measurements | Pass on current Fedora host |
| Hostile multi-process timeout fixture | Partial pass on current Fedora host |
| Native Ubuntu, macOS, and Windows execution | Absent |
| Independent command-boundary review | Absent |
| Sprint result | Blocked |

## Verified Locally

- The kernel freezes executable path and digest, literal argv, empty scratch directory, safe fixed
  environment, risk, mandatory grant use, noninteractive/offline state, and six resource ceilings
  into one exact template digest.
- One exact tool request and held target pass through the durable authority transaction before a
  nonforgeable command launch permit is created. A pre-launch cancellation emits one cancelled
  receipt and calls no executor.
- The Linux adapter verifies root-owned launcher and executable bytes before each attempt, clears
  the host environment, omits `PATH`, mounts only runtime libraries and one descriptor-held target,
  unshares the network, applies seccomp and cgroup limits, and invokes no shell.
- A live `/usr/bin/printf` fixture returns only its expected literal output. Live `/usr/bin/sleep`
  timeout and cancellation fixtures both retain systemd CPU, peak-memory, and greatest-observed-task
  measurements, terminate, and leave no active transient process unit.
- A live OpenSSL multi-process workload creates at least three observed tasks inside an eight-task
  ceiling, reaches its exact deadline, receives control-group termination, and leaves the unit
  inactive. This does not substitute for the remaining grandchild, descriptor, or parent-crash cases.
- The fixed 44-case injection corpus rejects substitution, separators, pipes, redirection,
  background operators, globbing, response files, configuration and alias injection, evaluators,
  pager/editor/plugin and hook arguments, transport helper arguments, shell/interpreter launchers,
  `PATH`, proxy, credential, and loader environment, relative/traversing executables, control bytes,
  and empty arguments. Exactly one literal control case is accepted, and the corpus test asserts the
  exact case count so category loss or silent growth fails closed.
- Live fixtures observe the guest environment as exactly the sealed template variables plus the
  fixed guest working directory, prove empty scratch carries no residue between attempts, and prove
  no second executable is reachable inside the guest mount namespace.
- Preview and receipt schemas reject unknown fields, inherited environment, shell executables,
  false timeout success, nonzero success, and cancellation without descendant cleanup.

## Open Evidence

No production configuration registers a command profile, and no command is available through the
installed VS Code product. Current Linux tests use exact synthetic development templates and do not
authorize repository commands. Sprint 42 owns separately hardened Git templates and repository
preservation.

The current receipt records elapsed time, stream use, cumulative CPU time, peak memory, and greatest
observed task count while the preview records the enforced ceilings.

Inherited-descriptor confinement and empty-scratch residue are now covered by live fixtures. A
multi-level process tree built by exec remains unreachable rather than merely denied, because the
guest mounts no second executable; proving termination of a purpose-built multi-level helper would
require registering a root-owned helper binary, which local development cannot install. Parent-crash
recovery, maximum-limit boundary campaigns, planted host-configuration observation, and complete
canary campaigns remain absent. Ubuntu, macOS, and Windows native
results cannot be inferred from Fedora. Independent review, installed trusted-launcher evidence,
manual fuzzing, the blocked Sprint 40 dependency, and `G-V0.3` remain open.

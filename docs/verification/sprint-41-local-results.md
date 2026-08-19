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
  fixed guest working directory, prove empty scratch carries no residue between attempts, prove no
  second executable is reachable inside the guest mount namespace, and enumerate the guest
  filesystem root as only the declared mounts, with no host home, configuration, credential, or
  repository path present. Planted host configuration, hooks, and rc files are therefore
  unreachable rather than merely rejected.
- Live descriptor fixtures enumerate the guest descriptor table as exactly standard input, standard
  output, standard error, and the enumerator's own descriptor in both working-directory modes.
  Deliberately inherited non-close-on-exec file and directory descriptors, proven inheritable by an
  unsandboxed control child, never cross the boundary. A changed held-worktree identity fails
  before launch.
- Bounded-output and minimum-deadline boundaries are exercised live: output beyond the declared
  stdout ceiling terminates as an output limit, retains exactly the declared byte count, and still
  reports the complete observed length and a SHA-256 over the whole stream; the smallest admissible
  deadline still terminates the unit and verifies descendant cleanup.
- The Linux runner declares a minimum executable task ceiling of three, measured directly:
  Bubblewrap needs one task for itself and one for the guest's pid 1, leaving one for the command.
  Ceilings of one and two are now rejected before spawn with `linux.command.tasks.below_minimum`
  rather than failing opaquely inside namespace creation, and the minimum-boundary test succeeds at
  the declared minimum. The kernel keeps its platform-neutral admitted range because other platforms
  may carry different launcher overhead. The maximum admissible timeout, output, memory, task, and
  CPU ceilings also execute with exact command identity and literal output, so the accepted bound
  range is executable at both ends on Fedora.
- A crashed supervisor leaves no owned unit, descendant, scratch tree, or process behind. Six
  scenarios per run kill a uniquely identified driver at two lifecycle points, once immediately
  after it reports readiness and once after its owned transient unit is confirmed present. The
  abandoned unit is not terminated by the supervisor's death; it stays bounded by `RuntimeMaxSec`,
  measured at roughly two seconds of survival for a three-second ceiling. The interrupted attempt
  produces no terminal receipt and is never replayed. Units are inventoried by exact name before and
  after, descendants are matched by exact control-group path, and cleanup guards terminate only the
  exact owned units even when an assertion fails.
- Preview and receipt schemas reject unknown fields, inherited environment, shell executables,
  false timeout success, nonzero success, and cancellation without descendant cleanup.

## Open Evidence

No production configuration registers a command profile, and no command is available through the
installed VS Code product. Current Linux tests use exact synthetic development templates and do not
authorize repository commands. Sprint 42 owns separately hardened Git templates and repository
preservation.

The current receipt records elapsed time, stream use, cumulative CPU time, peak memory, and greatest
observed task count while the preview records the enforced ceilings.

Inherited-descriptor confinement, host-configuration unreachability, empty-scratch residue, and
parent-crash recovery are now covered by live fixtures. A multi-level process tree built by exec
remains unreachable rather than merely denied, because the guest mounts no second executable;
proving termination of a purpose-built multi-level helper would require registering a root-owned
helper binary, which local development cannot install without weakening executable provenance.
Complete per-template boundary sweeps and canary campaigns remain absent. Ubuntu, macOS, and Windows native
results cannot be inferred from Fedora. Independent review, installed trusted-launcher evidence,
manual fuzzing, the blocked Sprint 40 dependency, and `G-V0.3` remain open.

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
| Peak CPU, memory, and task-use measurements | Absent |
| Hostile child/grandchild process-tree campaign | Absent |
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
  timeout and cancellation fixtures both terminate and leave no active transient process unit.
- The fixed 18-case injection corpus rejects substitution, separators, pipes, globbing, response
  files, configuration and alias injection, evaluators, shell/interpreter launchers, `PATH`, proxy
  and credential environment, relative/traversing executables, and control bytes.
- Preview and receipt schemas reject unknown fields, inherited environment, shell executables,
  false timeout success, nonzero success, and cancellation without descendant cleanup.

## Open Evidence

No production configuration registers a command profile, and no command is available through the
installed VS Code product. Current Linux tests use exact synthetic development templates and do not
authorize repository commands. Sprint 42 owns separately hardened Git templates and repository
preservation.

The current receipt records elapsed time and stream use while the preview records enforced CPU,
memory, task, time, and output ceilings. It does not retain platform-observed peak CPU, memory, or
task use. Native hostile child/grandchild escape, inherited-descriptor, parent-crash, maximum-limit,
scratch-residue, and complete canary campaigns remain absent. Ubuntu, macOS, and Windows native
results cannot be inferred from Fedora. Independent review, installed trusted-launcher evidence,
manual fuzzing, the blocked Sprint 40 dependency, and `G-V0.3` remain open.

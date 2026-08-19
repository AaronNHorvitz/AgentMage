# Bounded Command Runner

## Status And Scope

Sprint 41 implements an inactive bounded-command subsystem for Fedora and Ubuntu development. It
does not activate command execution in the product, register a production command catalog, add a
generic shell, or satisfy `G-V0.3`. The implementation is a locally exercised dependency for later
coding capabilities.

The subsystem admits one complete, immutable direct-process template at a time. A model can propose
a registered template identity. It cannot choose an executable, append an argument, inherit an
environment variable, broaden a working directory, raise a limit, or create authority.

## Authority And Execution Flow

```mermaid
sequenceDiagram
    participant Model as Untrusted model or UI proposal
    participant Registry as Kernel command registry
    participant User as Native approval surface
    participant Grants as Durable grant issuer
    participant Tx as Authority transaction
    participant Linux as Linux command executor
    participant Unit as systemd and Bubblewrap unit

    Model->>Registry: Template ID, version, and exact digest
    Registry-->>User: Canonical argv preview, environment, limits, and digest
    User->>Grants: Exact approval decision
    Grants-->>Tx: Single-use CommandExecute grant
    Tx->>Tx: Revalidate call, target, policy, preimage, and preview
    Tx->>Grants: Atomically consume exact grant
    Tx->>Linux: Nonforgeable one-use launch permit
    Linux->>Unit: Verified executable descriptor and literal argv
    Unit-->>Linux: Exit, output, resource, timeout, cancellation, and cleanup observations
    Linux-->>Tx: Closed platform result
    Tx-->>User: Kernel receipt plus command receipt
```

The only executable route is
[`CommandEffectDriver`](../../kernel/engine/src/command_runner.rs). It receives an opaque
[`EffectAuthorization`](../../kernel/engine/src/authority_transaction.rs) after the durable
transaction consumes one exact `CommandExecute` grant. The platform executor receives a
`CommandLaunchPermit` whose fields are private and whose constructor is unavailable outside the
kernel. Construction of a registry, preview, driver, or model response does not cross the effect
boundary.

## Exact Command Contract

`CommandSpec` binds all of the following into one canonical SHA-256 identity:

- schema, template identity, and immutable semantic version;
- absolute executable path and exact executable-content digest;
- a complete literal argument vector;
- either the initial private `empty_scratch` directory or the exact descriptor-held,
  read-only `owned_worktree` directory;
- a complete replacement environment;
- low or moderate review risk;
- mandatory exact-grant use and explicit noninteractive, offline, no-inheritance state; and
- wall-clock, output, memory, task-count, and CPU ceilings.

`CommandRegistry` is non-empty, bounded to 64 exact templates, rejects duplicate identities, and
resolves only an exact identity/version/digest tuple. A request contains no executable, free-form
argument, environment, path, or limit field. The preview renders argv as an ordered array rather
than a reconstructed shell string.

The current admission contract rejects:

- relative paths, parent components, malformed versions, digest drift, duplicates, and oversize;
- shell and interpreter executables, including environment launchers;
- shell substitution, separators, redirection, globbing, control bytes, and response files;
- configuration, evaluator, plugin, pager, editor, hook-helper, and loader-style argument prefixes;
- every environment variable except `LANG`, `LC_ALL`, `NO_COLOR`, and `TZ`; and
- interactive mode, network mode, inherited environment, or execution without a grant.

Repository commands are not in a production registry. A later coding profile may admit a separately
reviewed template only with the exact `owned_worktree` directory named by its current grant. That
directory remains read-only to the command sandbox; source mutation continues through the distinct
controlled-write transaction. Repository work must also use the preservation controls in Sprint 42.
It must not introduce a generic pull, shell, arbitrary setup command, hook, filter, pager, editor,
or credential-helper route.

## Linux Process Boundary

[`LinuxBoundedCommandExecutor`](../../platforms/linux/src/command_runner.rs) verifies and holds
root-owned, non-writable descriptors for `systemd-run`, `systemctl`, Bubblewrap, and each registered
executable. It reopens and hashes every launcher immediately before each attempt. Changed bytes,
links, writable ownership, missing executable bits, or registry/hash mismatch fail before launch.

The executor starts one random transient user unit with:

- `NoNewPrivileges`, SUID/SGID restriction, locked personality, control-group kill mode, and forced
  final `SIGKILL`;
- exact memory, task-count, CPU, stop-time, and runtime ceilings;
- a fresh Bubblewrap user, PID, IPC, UTS, cgroup, and network namespace;
- dropped capabilities, a sealed fixed seccomp policy, private `/proc`, `/dev`, and `/tmp`, plus
  either private scratch or the exact read-only held worktree at `/work`;
- only read-only runtime-library mounts, with no `/usr/bin`, home, workspace, credential, browser,
  SSH, cloud, or unrelated repository mount;
- the exact executable and sealed seccomp program passed through systemd `OpenFile` descriptors,
  with the executable mounted as `/app/command`;
- an optional held worktree reopened only through AgentMage's own descriptor table, declared as a
  third `OpenFile` descriptor, and mounted read-only at `/work`; and
- an empty inherited environment followed by only the four safe fixed variables in the template.

systemd numbers `OpenFile` descriptors from `SD_LISTEN_FDS_START` in declaration order, so the guest
always receives the executable, the seccomp program, and the optional worktree in that fixed order.
Bubblewrap consumes each of them while building the guest mount namespace, so none remains open when
the target executes. Standard input is always the null device: a directory is never transported
through it, and the worktree descriptor is never reopened through its ordinary filesystem path.

The target is launched directly as `/app/command`. No shell parser, command string, `PATH` lookup,
interactive standard input, repository configuration, or network namespace is available. Model
text never reaches the process boundary as syntax.

Cancellation and timeout both request a control-group kill and stop, wait for the supervisor, and
verify through systemd that the unit is inactive or absent. Cleanup is not inferred from model text
or from a timeout request alone.

## Output, Resources, And Receipts

Standard output and error are drained concurrently to avoid pipe deadlock. The platform retains no
more than each declared byte limit while hashing and counting the complete observed streams. A
command receipt records:

- authority, operation, command, template, request, and preview identities;
- exited, cancelled, timed-out, output-limit, or launch-failed termination;
- deterministic operation outcome, exit code or signal, and stable platform code;
- complete stream digests, total bytes, retained bytes, and truncation state;
- monotonic elapsed time, cumulative CPU time, peak memory, greatest observed task count, and
  verified descendant-cleanup state; and
- a canonical receipt digest.

Exit zero is the only successful process result. Nonzero exit, launch failure, and output-limit
termination are failures. Cancellation and timeout retain their own outcomes. A model narration is
not execution evidence.

The preview records configured memory, task, CPU, time, and output budgets. Linux samples the exact
transient unit's `CPUUsageNSec`, `MemoryPeak`, and `TasksCurrent` properties while it runs, retains
the cumulative or greatest observation, and records the typed values in the terminal receipt.
Unavailable observations remain explicit `null`; they are never replaced by invented zeroes.
Observed memory and task use must remain within the approved template bounds.

The task ceiling counts every task in the transient unit's control group, not only the command's own
processes, because that is exactly what `TasksCurrent` reports and what the receipt must record
literally. Bubblewrap needs one task for itself and one for the guest's pid 1, so the Linux runner
declares a minimum executable ceiling of three and rejects any smaller ceiling before spawn with
`linux.command.tasks.below_minimum`. The kernel keeps its platform-neutral admitted range because
other platforms may carry different launcher overhead; a ceiling this runner cannot execute fails
closed with an exact platform code instead of an opaque namespace-creation error. The enforced
`TasksMax` therefore always equals the declared ceiling, and observed task counts are never adjusted
to hide launcher overhead.

If the supervising process dies mid-attempt, the abandoned transient unit is not terminated by the
supervisor's death: it remains bounded by `RuntimeMaxSec`, which is derived from the command's own
declared timeout. The interrupted attempt yields no terminal receipt, because no result was ever
observed, and it is never replayed to manufacture one.

## Verification And Remaining Work

The retained injection corpus is
[`sprint-41-command-injection-corpus.json`](../verification/sprint-41-command-injection-corpus.json).
Kernel tests cover exact registry selection, shell/interpreter/environment denials, one-use grant
ordering, receipt classification, and cancellation before launch. Fedora live tests cover an exact
literal command, real timeout and cancellation cleanup, retained resource observations, and a
bounded multi-process OpenSSL workload through the native isolation stack.

Sprint 41 remains blocked from closure until all of the following exist:

- an independently admitted production command registry and product configuration;
- complete native Fedora, Ubuntu, macOS, and Windows sandbox runs at minimum and maximum limits;
- complete child/grandchild, descriptor, scratch, and parent-crash fixtures beyond the current
  Fedora multi-process timeout case;
- complete workspace/grant/network/credential canary campaigns;
- clean installed-package execution through the trusted launcher;
- independent command-boundary review; and
- deferred manual fuzzing and the upstream `G-V0.3` dependency.

No local fixture, Linux result, schema validation, or documentation check substitutes for those
missing gates.

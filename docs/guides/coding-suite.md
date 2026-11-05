# Frozen coding suite

T-311.17 extends the original development collection with a separately kept
held-out set. [Decision 0154](../decisions/0154-local-coding-delivery-refocus.md)
keeps the entire suite/admission assignment in that row. Its earlier diagnostic
oracle PASS remains a completed historical milestone, rather than approval of
the expanded assignment. T-311.18 now owns the baseline, preserving its earlier
protocol-lifecycle milestone in the same way.

## Frozen inputs

The development set remains exactly 30 tasks and 89 original test methods,
including its existing Rust task. Its complete task-definition digest is
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
No accepted statement, base, reference, hidden byte, fixture, method selection or
test threshold changes.

The held-out descriptor contains ten distinct historical source changes and
52 complete original test methods. Its byte digest is
`8141e9281c830fe510863403d6c1f137dcb661c2ba6067e68a94a578a13e5c3b`.
The descriptor stays in private operator storage, outside the repository and
editing workspace. Preserve a private backup of those exact bytes. The command
refuses changes, missing inputs, aliases, hard links, special files, shared
permissions and files owned by a different user. There is no environment or
command-line digest override, configurable test selection or generated-score
entry point.

The runner also refuses a descriptor or raw output under this repository, any
registered Git worktree (including an unavailable worktree), or an additional
editing/model workspace supplied with repeatable `--workspace-root` options.
It resolves existing ancestor aliases before opening inputs or creating output.
Worktree discovery failure refuses the operation. Supply every additional model
workspace; unrelated programs' unregistered workspaces cannot be discovered by
the runner. These checks assume operator-owned paths and stable workspace
registration during the command; they do not isolate a hostile host operator.

These held-out tasks concern Python evaluation-protocol repairs in this
repository. All ten modify the same worker module at different historical
commits. This is a narrow tooling set, not evidence of general application
coding quality. Source and regression origins are this repository's own work
under its existing licence; no external dataset or dependency is introduced.
No historical hidden test is extracted or rewritten: each selection names all
complete methods added or changed in its original repair commit. Selection
precedes any control or model run on the new set. The statement/interface audit
also precedes those runs.

Hidden tests remain absent from editing projections. The suite checks every
original Git pin and refuses a held-out reference source already present in a
development editing projection, or a repeated development source change. An
ancestry check also refuses a development base or hidden-test revision that
descends from, or equals, a held-out reference. Missing ancestry information
refuses the check. This does not establish that the history is unknown to the builder or private
on the public internet. The operator can see the underlying history; future
model runs must receive only a fresh task projection and statement, with no
repository history, tests, reference data or cross-task conversation state.
Never tune prompts, models or harness code from held-out feedback.

## Commands

Use the shared build reservation in the lane. Inputs and raw outputs are private;
replace placeholders with operator-selected paths.

```sh
bash /tools/build-slot python3 -B scripts/coding_eval_suite.py check \
  --held-out <private-held-out-descriptor> \
  --workspace-root <additional-model-workspace>
```

`check` validates the commitments and source separation without importing a
held-out test module or executing a held-out assertion. The ordinary fast gate
tests the suite runner on separate synthetic contracts; it does not run the
held-out tasks.

For a row-closing validation only:

```sh
bash /tools/build-slot python3 -B scripts/coding_eval_suite.py verify \
  --held-out <private-held-out-descriptor> \
  --workspace-root <additional-model-workspace> \
  --purpose row-close --output <new-private-output-directory>
```

This one command runs all 30 original development control pairs, the independent
development oracle's supported pairs, and ten held-out control pairs. Every
candidate is reconstructed from pinned base or reference bytes by the host.
The held-out entry accepts an opaque task ID and the base/reference kind, never
candidate bytes, task dictionaries or arbitrary patches. The legacy grader
executes only historical controls: it cannot establish the integrity of a
generated candidate's self-reported test results.

A base must produce observed failures or errors with the exact selected test
count, no skips or expected failures, and exit 1. A missing report, timeout,
input refusal, output truncation or infrastructure failure cannot validate a
base. Its reference must pass every selected method. Cleanup completes before
counts publish. Outputs are exclusive and never overwritten; interruption
propagates. Private reports retain each control's observations, source/test
digests and bounded log digests. The printable summary contains aggregate
results and bindings to private reports, without per-task held-out feedback.
Failed runs stay retained; diagnostic reruns must use new output directories.

The strict base rule applies to both development and held-out pairs in this
suite command, even if the older development verifier's summary passes. The
suite requires each development task exactly once and its complete selected
count on both controls. A duplicate, missing task, skip or abnormal base exit
cannot validate the collection. The standalone legacy verifier is unchanged.

## Adapter validation without held-out tuning

Develop grading adapters against synthetic contracts and development tasks.
Run the frozen held-out controls only with the row-closing command; retain every
attempt, including a failed close, and do not use that feedback to tune the
coding harness, prompts or model selection. Keep raw held-out feedback outside
editing/model workspaces. Its score will measure only the narrow protocol domain
described above.

The ordinary worker unit module overlaps the frozen held-out selections. This
runner's tests do not import that module. If future adapter work changes that
worker, perform its complete gate in the row-closing batch, or keep preliminary
synthetic adapter checks in a separate test module. Do not filter or edit hidden
tests to make a development gate pass. No held-out assertion has executed in
the input-freeze or storage/ancestry checks.

## Admission remains open

These commands do not enable generated scoring or produce a coding baseline.
The independent development oracle now covers 27 of the 30 development
tasks. Remaining development and held-out independent adapters, isolated worker
accounting and remaining clause-coverage admission still need
disposition within T-311.17. Resource, permission and evidence boundaries
remain unchanged.

## Bounded independent-worker cleanup and private scratch

Oracle v28 stages input copies, adapters, shared mode fixtures and trusted
results on one anonymous 128 MiB tmpfs with 32,768 inodes. Each worker's private
temporary filesystem has 64 MiB and 8,192 inodes. Shared mode-fixture data uses
the case quota. Read-only synthetic roots and devices prevent scratch bypass
there. The trusted supervisor has a separate 16 MiB setup filesystem.

The mount supervisor runs only trusted code in a separate networkless user/PID
namespace. Its namespace-local root maps to the same unprivileged host user;
mount setup and nested root-ID mapping retain only the two setup capabilities.
Both task workers drop every capability set and disable further user namespaces.
The supervisor owns a private Unix control connection and bounded host logs;
workers receive neither that connection, the full scratch descriptor nor the
supervisor's mounts. It launches their existing exchange locally, so bind sources
resolve to anonymous mount paths and expose no private host directories in mount
metadata. Grading-only Git setup also runs in that namespace, without source
history. Setup or transport failure refuses the case; no host-disk fallback runs.

After namespace creation, each worker installs a hard eight-process limit before
importing any candidate source. Forked processes and threads share that namespace
limit. The existing hard per-process 512 MiB address-space limit gives a derived
4 GiB address-space ceiling per task namespace. This is a policy bound, not a
resident-memory measurement. Trusted setup/reaper overhead is separate. The
60-second per-process CPU limit and 90-second exchange deadline remain; this does
not add aggregate CPU-time accounting or modify/delegate a host cgroup.

Result publication follows worker termination, scratch-descriptor closure and
supervisor termination/reaping. Releasing anonymous mount references replaces
traversal of candidate-controlled trees, including locked deep trees. The
enclosing host-only case container still uses the original five-second bounded
cleanup child, with five CPU seconds, 256 MiB address space and 64 descriptors;
its recovery and interruption rules remain. Direct traversal diagnostics retain
their original API and controls. No unbounded candidate traversal fallback runs.

Whole-suite `aggregate_worker_quotas` remains false. The historical graders have
the separate limits below; candidate-only CPU and resident-memory accounting
remain unavailable. The inherited lane observations below supplement these limits.
No generated candidate is admitted and no broader resource,
descendant-quiescence or release claim follows. The lane's read-only cgroup
configuration and resource allocation stay unchanged.

## Confined historical Python and Rust controls

The public development grader and frozen held-out control entry use a trusted
single-worker operation in the same anonymous-mount supervisor. The supervisor
constructs its fixed command itself; its control request contains only the
operation, language and deadline. Host preparation verifies hidden and Rust
fixture bytes before starting it. Python grading-only Git initialization runs
inside the supervisor, with no source history. Inputs, drivers and results share
the 128 MiB case filesystem and its 32,768-inode quota.

Python controls have a private 64 MiB temporary filesystem with 8,192 inodes and
a hard eight-process limit, installed before importing task code. The existing
512 MiB address-space and 60-second CPU limits remain per process. The derived
address-space ceiling is 4 GiB per namespace.

Rust controls have 768 MiB and 32,768 inodes for temporary build output and the
writable Cargo home together. The registry is read-only. A hard sixteen-process
limit and a tightened 768 MiB address-space limit apply before Cargo starts,
giving a derived 12 GiB address-space ceiling. The trusted runtime driver narrows
compilation to one job to fit this process/thread budget; the original driver
bytes remain unchanged. Two test threads, four-CPU affinity and the 60-second
per-process CPU limit remain.

Binding a Rust runtime through an alias still exposes its backing filesystem's
root path in mount metadata. Therefore the trusted supervisor copies the already
installed toolchain and registry from read-only inputs to a separate anonymous
1,536 MiB filesystem with 32,768 inodes. Workers receive read-only mounts from
that snapshot, never the host inputs. No download, installation or cache write
on the host occurs. Snapshot overflow or setup failure refuses the control;
there is no disk fallback. Together, maximum Rust scratch capacities total
2,432 MiB, plus the supervisor's 16 MiB setup filesystem. This declared capacity
and the derived address-space ceiling are bounds, not measured resident use.

Workers cannot write their root, device filesystem, inputs, driver, toolchain
or registry, cannot reach the supervisor's connection, and cannot create a new
user namespace. Anonymous argument transport and empty launcher environments
keep host arguments and environment out of namespace PID 1. The supervisor
caps each output stream at 2 MiB and execution at 90 seconds, then signals and
reaps its worker before replying. Socket deadlines also bound preparation plus
execution. Closing scratch descriptors and stopping/reaping the supervisor
release anonymous candidate trees, including inaccessible deep trees. Host
cleanup touches only fixed trusted control entries; it traverses no task tree.
Counts publish after these steps. Original host-only synthetic lifecycle
diagnostics remain separate and cannot pass the public historical-input guard.

Reports bind the control and scratch implementations and retain the declared
worker policy. These graders still execute assertions alongside historical
candidate code. They establish control discrimination only; they do not provide
independent result checking or admit generated code. The held-out input and
row-close restrictions are unchanged.

## Coverage and attempt accounting

Row-closing verification writes `development-coverage.json`, binding every
development definition, hidden digest, complete method selection and expected
count. Supported independent adapters retain their graded observations,
adapter-policy limits and ungraded behavior. Unsupported tasks explicitly have
no independent adapter. Unasserted statement clauses and end-to-end model coding
remain unestablished. This report reads definitions, not held-out assertions.
Its digest is included in the private verification summary.

The report names the previously observed disappearing-process race in dev-003.
Controls receive no automatic retries. Every failed attempt remains retained,
including infrastructure failures; a known race neither passes nor removes an
attempt from the denominator. An operator's separate diagnostic rerun is a new
retained run, never a replacement for a failed one. This is control accounting;
the current-harness model baseline still belongs to T-311.18.

No model or GPU run, held-out score, model-role choice, harness tuning or
parent-row acceptance follows from a passing input check or historical control
validation. Publish actual admitted model measurements in
docs/evidence/coding-scoreboard.md at T-311.18, before changing the coding agent.

## Inherited lane counters

Each development, independent and held-out historical report also binds a trusted
read-only resource reader. On the bounded Linux lane it records the existing
cgroup's finite RAM, swap, CPU and task limits, CPU/event counter deltas and
memory boundary samples. It changes no limit or delegation. The entire inherited
lane is the scope: trusted setup and other lane-local work count too. The memory
peak is the cgroup's lifetime peak, explicitly not a task peak or GPU measurement.

The reader checks membership, cgroup identity, quota stability and monotonic
counters at both boundaries, with a fixed refusal when observations are
unavailable or changed. Historical controls may retain unavailable accounting;
required mode refuses execution or final verified publication. Interruption
still propagates if the final observation fails. Reports contain no cgroup path.
These observations neither enable generated admission nor change the false
whole-suite `aggregate_worker_quotas` flag.

The ordinary worker unit module contains held-out selections. Preliminary
adapter changes run separate synthetic adapter modules and development tests;
the complete ordinary worker module remains deferred intact to row closing.
Record this distinction when reporting fast checks. Do not call a preliminary
batch an uninterrupted ordinary fast-gate run.

# Independent worker grading and candidate admission

The frozen worker-protocol set uses complete original selected methods in a
trusted interpreter and candidate code in a separate confined interpreter.
The assertions, method selections and thresholds are unchanged. This measures
a narrow protocol domain; it does not establish general coding quality.

## Assertion and process boundary

`scripts/coding_eval_worker_oracle.py` supervises one original method at a time.
Each method includes its original module initialization, setup, body, subtests,
assertions and cleanup. A new assertion interpreter and candidate namespace are
created for the next method. Module and class state across methods is ungraded;
retained mocks, aliases and message roots within a method remain shared.

The assertion interpreter compiles the whole original module before removing
its assertion, result and procfs mounts. Result, exception-detail and transcript
files have preopened private descriptors. The remaining filesystem is read-only
except quota-backed temporary fixtures. Symlinks cannot restore removed mounts.
Candidate code receives neither original tests nor a result directory.

A bounded typed graph transports data and explicit operations. It preserves
mutable list, dictionary, namespace and native Mock identities and call histories.
Primitive values, bytes, paths, finite memory streams, selected exception
categories and original fixture classes have explicit encodings. The wire never
transports source, bytecode, pickle or a candidate class to instantiate in the
assertion interpreter. Source-symbol handles resolve only in the candidate.
Mock assertion and reset APIs and test/verdict objects cannot be replaced through
the graph. Closed builtin type literals retain their category. List-derived
invalid inputs retain aliases, contents and their distinction from an exact list;
custom subclass methods and class identity remain ungraded. Unsupported object
configurations refuse grading.

Original fixture callbacks execute under an audit guard. They can use finite
temporary files after sealing; they cannot launch commands, open sockets, use
process inspection, compile code or recover interpreter frames. Callback
assertions remain sticky failures, even if the candidate catches the returned
exception. Their expected-value messages stay private.

Native process fixtures use the existing confined launch/adoption boundary.
A detached procfs descriptor is attached only during synchronous trusted launch
setup and removed before candidate import or a callback. Concurrent launch and
callback windows refuse execution. The candidate has no procfs, capabilities,
network or process-inspection syscalls. Its bridge is PID 1 in its own namespace;
its exit ends its descendants. Temporary fixture Git commands receive the same
confinement. Imported library patches remain subject-local so inner protocol
fixtures cannot replace the bridge's own standard streams.

The host loop is deliberately flat: an additional in-namespace coordinator
exhausted the existing eight-process allowance in synthetic tests. Removing that
coordinator preserves the allowance. No resource ceiling was raised.

## Bounds and records

The existing 512 MiB per-process address-space, 60-second per-process CPU and
eight-process limits remain. A whole task has a 90-second deadline. Shared case
scratch is 128 MiB with 32,768 inodes; private temporary fixtures are 64 MiB with
8,192 inodes. RPC frames have a 1 MiB limit, transcripts 16 MiB, exchanges 4,096,
and object identities 65,536. Overflows fail the attempt.

Each method's trusted report records original counts, startup, clean shutdown,
sealed paths and bounded adapter diagnostics. Full original exceptions and wire
records stay private. Host reports bind whole helper files, frozen task and test
bytes, materialized worker bytes and every promoted log. Counts publish after
namespace termination and anonymous-mount release. There is no single candidate
exit for a task with multiple child processes; that field stays null.

## Generated candidate entry

The historical `coding_eval.py`, `coding_eval_oracle.py` and suite control entry
points keep their frozen-base/reference guards. The separate
`scripts/coding_eval_admission.py` command accepts generated snapshots only through
independent grading. It requires a successful complete-suite control receipt,
matching whole-file source hashes and matching bound control logs. A changed
source, missing/partial receipt or changed frozen input refuses admission.

```sh
bash /tools/build-slot python3 -B scripts/coding_eval_admission.py \
  --candidates <candidate-snapshots> --controls <private-controls-summary> \
  --output <new-private-results>
```

Snapshots are named by frozen task ID. Each must contain exactly its frozen
projection with only the designated source file editable. Tests, extra files,
symlinks, changed dependencies and missing files refuse that attempt. The command
always processes the complete development set. Held-out scoring additionally
requires `--split held-out --held-out <private-descriptor>` and
`--purpose row-close-scoreboard`; raw feedback remains private. Never forward
hidden feedback to the model or use it to tune prompts, models or harness code.

Admission requires verified inherited lane quotas before execution and at final
publication. These kernel-enforced quotas cover the whole lane, including setup
and other lane processes. They bound aggregate resource consumption but do not
provide a separate worker cgroup or candidate-only resource peaks. The existing
`aggregate_worker_quotas: false` disclosure remains. No cgroup limit, delegation
or resource reservation is changed. Unavailable or changed accounting refuses
verified publication.

Every frozen task remains in the denominator. Missing snapshots, infrastructure
failures, protocol errors and ungradable adapter-policy refusals earn no pass and
are retained. There is no automatic control retry or omitted failed attempt.
The evaluator launches no model; inference time, tokens, GPU use, first-attempt
and with-retry model scores belong to the separate campaign record.

Passing means the unchanged original assertions passed over the declared finite
observations. It does not establish unasserted statement clauses, general object
identity, cross-method state or authentic candidate-reported diagnostics. The
existing usage adapter's deliberate self-modification and conservative mutation
refusal limitations remain explicit backlog notes. A policy refusal is ungradable,
counts as an unsuccessful attempt, and does not establish a frozen-task defect.
These limitations must accompany published scores; they are not claims of repaired
behavior or permission to change a hidden test.

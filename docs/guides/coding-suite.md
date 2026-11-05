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
The independent development oracle now covers 20 of the 30 development
tasks. Remaining development and held-out independent adapters, aggregate
worker bounds, shared-fixture scratch quotas, legacy cleanup deadlines,
mount-metadata privacy and remaining clause-coverage admission still need
disposition within T-311.17. Resource, permission and evidence boundaries
remain unchanged.

## Bounded independent-worker cleanup and private scratch

The independent grader repairs/removes candidate scratch in a separate trusted
host child, with a five-second parent-enforced deadline, five CPU seconds,
256 MiB address space, 64 open descriptors and no core files. The child closes
inherited descriptors, starts its own process group and requests termination
if its parent dies. The parent kills and reaps the child before returning,
including interruption and setup failure. No candidate code executes in this
helper. Host-only direct traversal diagnostics retain their original API;
grading uses the bounded helper.

The enclosing case uses the same bounded cleanup, without an unbounded removal
fallback. A mode-fixture case can require two helpers, at most five seconds each;
an ordinary case needs one. Detached deep-tree fragments stay in the private
case container. Failed cleanup records that container for operator recovery and
prevents counts or wire logs from publishing as results. The original
interruption survives cleanup or recovery-record write failure. A failed
recovery write cannot promise a recovery record exists; the private container
remains. This is a cleanup deadline, not an aggregate worker quota or proof
that every namespace descendant has already stopped.

Each namespace-private `/tmp` has a 64 MiB tmpfs byte limit. The synthetic root
and device mount are read-only, preventing ordinary files there from bypassing
that limit. Shared host-backed mode fixtures remain outside this quota, as does
the trusted oracle's result mount. This bounds private temporary file data,
not aggregate process memory, inode usage, processes or CPU. The existing
per-process limits and execution deadline remain. The lane's cgroup filesystem
is read-only; no delegated aggregate limit is claimed or raised.

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

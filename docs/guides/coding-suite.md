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
development editing projection, or a repeated development source change. This
check does not establish that the history is unknown to the builder or private
on the public internet. The operator can see the underlying history; future
model runs must receive only a fresh task projection and statement, with no
repository history, tests, reference data or cross-task conversation state.
Never tune prompts, models or harness code from held-out feedback.

## Commands

Use the shared build reservation in the lane. Inputs and raw outputs are private;
replace placeholders with operator-selected paths.

```sh
bash /tools/build-slot python3 -B scripts/coding_eval_suite.py check \
  --held-out <private-held-out-descriptor>
```

`check` validates the commitments and source separation without importing a
held-out test module or executing a held-out assertion. The ordinary fast gate
tests the suite runner on separate synthetic contracts; it does not run the
held-out tasks.

For a row-closing validation only:

```sh
bash /tools/build-slot python3 -B scripts/coding_eval_suite.py verify \
  --held-out <private-held-out-descriptor> \
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

## Admission remains open

These commands do not enable generated scoring or produce a coding baseline.
The independent development oracle still covers only 19 of the 30 development
tasks. Remaining development and held-out independent adapters, aggregate
worker bounds, scratch limits and cleanup deadlines, mount-metadata privacy,
coverage labels and known-race accounting still need disposition within
T-311.17. Resource, permission and evidence boundaries remain unchanged.

No model or GPU run, held-out score, model-role choice, harness tuning or
parent-row acceptance follows from a passing input check or historical control
validation. Publish actual admitted model measurements in
docs/evidence/coding-scoreboard.md at T-311.18, before changing the coding agent.

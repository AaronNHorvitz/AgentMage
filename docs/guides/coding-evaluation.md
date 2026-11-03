# Coding evaluation development pilot

T-311 requires at least 30 real repository tasks, a fixed development/held-out
split and a current-harness model baseline before tuning. This first executable
portion has **three development tasks only**. It validates scoring machinery;
it is not the complete suite, a model baseline or a scoreboard entry. No held-out
task has been selected or inspected by this pilot. Keep agent tuning deferred
until the complete suite and its baseline exist.

## Frozen tasks

All three tasks come from this repository's own development history. They use
original repository code and tests under the existing repository licence; no
external task dataset, model, dependency or comparison is introduced. This work
does not relicense the historical inputs or certify import readiness.

| Task | Work | Frozen base | Historical fix |
| --- | --- | --- | --- |
| dev-001 | Bind untracked source bytes in implementation identity | `aa3134b224e887fbf5962fbf724b8d3d54cbe8ec` | `14e33a4565e34663fd84fe0939b3ccc5bc2e633d` |
| dev-002 | Separate a model request observation from serving/qualification | `6d1cb28424866cb95c1e5460427b4f0bbfe44384` | `0898b05a6c82bd531704e3f13db412f537ca4f75` |
| dev-003 | Keep the check leader until process cleanup finishes; accept retained review formats | `4fda20f7d9f335cfe859cb3e01206573ea5e3e08` | `e88a9b42cd35821d81f6c8d706b65f151aadcbe2` |

[`coding_eval.py`](../../scripts/coding_eval.py) pins the statements, editable
path, base and reference file digests, hidden test digest, complete projection
digest and exact test counts. Its task-definition fingerprint is checked by
[`test_coding_eval.py`](../../tests/test_coding_eval.py). Expansion adds new tasks;
it must not change these statements, hidden tests, pins or thresholds. The
historical fixes validate the evaluator, never count as model attempts.

The task workspace is a bounded projection of the base commit's tracked Python
files under `scripts/`. It contains no `.git`, test files, reference patch,
evidence or credentials. This is enough for these tooling tasks, not a complete
checkout or a general repository adapter. Only the declared source file may
change. All other files must stay byte-identical; additions, deletions, aliases,
hard links, special files, extra directories and oversize input are refused.
Candidate bytes are copied through held directory/file descriptors before tests
run. The agent never executes inside the grader's process or mounts.

Hidden means withheld from the editing workspace. These development regressions
already exist in public Git history, and their locations/digests are available to
the evaluator operator. They are unsuitable as unseen held-out tasks. The eventual
held-out collection must have separate access and never supply tuning feedback.
Do not call this public development pilot a blind evaluation.

## Run the scorer

Use the existing Python 3, Git and bubblewrap installation on Linux. No download
or installation is performed. Full verification and suites in the shared lane
use its build reservation:

```sh
bash /tools/build-slot python3 scripts/coding_eval.py verify --output <new-private-results>
```

This one command scores each frozen base and the corresponding historical fix.
It succeeds only when every base fails its tests and every fixed candidate
passes all of its pinned tests, with no skips or expected failures. An unavailable
worker, timeout, missing/malformed result, changed test count or hash mismatch
cannot validate a task. Each case keeps bounded raw stdout/stderr, test counts,
exit status, elapsed wall time, source/input digests and a report. The aggregate
report labels itself `evaluator-validation-only` and has `model_score: null`.
No GPU or model runs as part of this command; its token count is unavailable and
its model-attempt count is zero.

For a single candidate:

```sh
python3 scripts/coding_eval.py list
python3 scripts/coding_eval.py prepare --task dev-001 --output <new-candidate-directory>
# Edit only the source file identified by list, using the task statement.
bash /tools/build-slot python3 scripts/coding_eval.py score --task dev-001 --candidate <candidate-directory> --output <new-private-result>
```

`list` prints statements and bases without hidden-test/reference metadata.
`prepare` materializes the base projection without tests or answers. `score`
refuses anything outside that task's one-file scope, snapshots the candidate,
and runs the pinned hidden test module. Exit 0 means all pinned tests passed;
exit 1 means failed tests, timeout or worker failure; exit 2 is an input or
operator error. Existing output/candidate destinations are never overwritten.
Keep output private: test diagnostics may contain task content and temporary
paths. Only sanitized measurements and raw-log digests belong in public evidence.

The scorer launches a separate worker with PID, network and other namespaces,
cleared environment, read-only system runtime, read-only candidate and hidden
tests, a temporary home/scratch and one writable result directory. It mounts no
host home, live repository, model files or credentials. It enforces a 90-second
case deadline, 60-second CPU limit, 512 MiB per-process address-space limit and
32 MiB worker-file limit, allowing the unchanged 16 MiB oversize-input
regression. The parent retains at most 2 MiB each of stdout/stderr and stops a
worker exceeding either output budget. Raw logs are outside the worker mount;
only its temporary result directory is writable. Address-space limits are not a
process-tree memory quota. Timeout/interruption kills the owned process group before reaping its
leader; the worker PID namespace also bounds descendant lifetime. An unavailable
namespace is a failure, never a reason to run unconfined. These are development
evaluation controls; no product sandbox or trust policy is changed.

## What this does not measure

The runner deliberately has no model or agent adapter yet. It cannot produce
first-attempt or within-retry-budget model success, tokens, active GPU time or
coding-quality numbers. A later current-harness attempt must retain the actual
binary/source identities, objective, approvals, raw logs, limits and refusal
cause. A startup refusal is an unavailable baseline, not a solved task or a
zero-quality model score. GPU work always uses the existing queued reservation;
never attach a substitute direct-model edit loop to manufacture a baseline.

The grader imports candidate Python into the test interpreter in its confined
worker, as the original regression tests do. It withholds tests during editing,
checks their frozen bytes and requires the full expected counts. It is not an
independent proof against a candidate deliberately tampering with Python's test
framework or forging its result. Add independent result checking before using
adversarial model-generated candidates for a trusted scoreboard. This pilot's
observed base/reference validation has no generated candidate.

The remaining T-311 work includes 30 or more suitable tasks beyond this narrow
Python projection, varied fixes/features/refactors/test additions, sealed held-out
inputs, a bounded/resumable current-harness adapter, actual model attempts and
failure classification, retry/first-attempt accounting and scoreboard-only
held-out execution. No existing product harness, prompt, hidden test or score
threshold is tuned by this pilot.

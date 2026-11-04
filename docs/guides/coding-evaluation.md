# Coding evaluation development collection

T-311 requires at least 30 real repository tasks, a fixed development/held-out
split and a current-harness model baseline before tuning. This executable
collection has **30 development tasks only**, with 89 reference regressions: 29
Python tooling tasks and one Rust contract bug fix (a regression exercising 640
path cases).
It validates scoring machinery; it is not the complete suite, a model baseline
or a scoreboard entry. No held-out task has been selected or inspected by it.
Keep agent tuning deferred until the complete suite and its baseline exist.

## Frozen tasks

The `coding-development-v4` revision appended 11 original-repository development
tasks. The `coding-development-v5` revision corrects their statements before any
model run after [independent review](../reviews/c86571bb85c8-local-coding-t311-3-development-collection.txt)
found a Git error/call protocol missing from dev-014. It also clarifies dev-004's
ValueError refusals and dev-011's string constant. No hidden test, selection,
threshold or source pin changes. It preserves every original pilot definition byte-for-byte as data,
including statements, base/reference/input digests and the 38-test requirement.
Its Python collection fingerprint remains
`72442a4cf406abd83dc7684a5ef225474b55ee2cbc4cb6e5d34e6a840f0da618`.
The `coding-development-v6` revision appends one Rust task without changing any
Python task dictionary, hidden input, selection or threshold. Its combined
fingerprint is
`3d2b07a1336133a62b49c07522d1786017e241d46f497285d18c8e391c73f88a`.
The `coding-development-v7` appends five original Python fixes, preserving
all 15 accepted definitions (including the Rust task) unchanged. It has fingerprint
`cd5fc51018e53364df2f26eaf91ea29a46805e19a6e7e069370372bd98cb8e46`.
The [v7 record](../evidence/t311-5-development-collection-2026-11-03.md)
records scope and the real control measurements.
The `coding-development-v8` appends ten distinct historical source
changes (12 complete regression methods) without changing any of the 20 accepted
definitions. Its complete fingerprint is
`91f1eecfa91129f2f8392363aaff0e8f49243aa761947e69d5ec1452d7b4b04d`.
The [v8 record](../evidence/t311-6-development-collection-2026-11-03.md)
describes the statements, limited covered clauses and control measurements.
The current `coding-development-v9` corrects dev-021's digest-check scope,
dev-029's notice discriminator and top-level controls shape, and dev-022's
artifact ID field locations before any model measurement. The
[independent findings](../reviews/57ea7ee64ec6-local-coding-t311-6-development-collection.txt)
and [repair record](../evidence/t311-6-statement-contract-repair-2026-11-03.md)
explain these changes. All hidden bytes, source/input pins, method selections,
counts and the 20 previously accepted task definitions stay unchanged. Its
complete fingerprint is
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
The automatic interface-name audit checks names only; it does not establish
statement sufficiency for data shapes or conditional requirements. dev-024's
base projection already contains a similar GPU observation helper in another
script; success there includes finding and adapting existing repository code.
Reaching 30 development tasks meets the count for this collection, not the
complete T-311 requirement: language/work coverage remains narrow, held-out inputs
are absent, generated scoring is disabled and the real-model baseline is open.
The first-three-task fingerprint remains
`d5f5e365924ffea8c6b46c50c28d00e116c01f69de57ac0a9672ffa50e05e3c5`.
No agent attempt informed the selection or statements. The
[expansion record](../evidence/t311-3-development-collection-2026-11-03.md)
records the new contracts, failed prospective tasks and actual verifier results.
The [v5 correction record](../evidence/t311-3-statement-contract-repair-2026-11-03.md)
records the assertion audit and builder-authored alternative controls; these
are diagnostic probes, not blind implementations or model scores.

The prior `coding-development-pilot-v3` revision corrected residual statement defects
from the [second independent review](../reviews/b1734b9957ce-local-coding-t311-1-evaluator-repair.txt).
The earlier v2 revision added function names, return keys, error codes and
encodings, but left a diagnosis-version contradiction and two omitted review
rejections. Before any model measurement, v3 specifies that diagnosis output
changes from version 1 to 2 while persisted records stay version 2, and that
FAIL and BLOCKED review verdicts must be rejected. Bases, references, projections,
hidden tests and exact counts remain unchanged. See the
[v3 correction and assertion audit](../evidence/t311-1-statement-contract-repair-2026-11-03.md).
This is a recorded pre-measurement correction, not feedback from model attempts.

All tasks come from this repository's own development history. They use
original repository code and tests under the existing repository licence; no
external task dataset, model, dependency or comparison is introduced. This work
does not relicense the historical inputs or certify import readiness.

| Task | Work | Frozen base | Historical fix |
| --- | --- | --- | --- |
| dev-001 | Bind untracked source bytes in implementation identity | `aa3134b224e887fbf5962fbf724b8d3d54cbe8ec` | `14e33a4565e34663fd84fe0939b3ccc5bc2e633d` |
| dev-002 | Separate a model request observation from serving/qualification | `6d1cb28424866cb95c1e5460427b4f0bbfe44384` | `0898b05a6c82bd531704e3f13db412f537ca4f75` |
| dev-003 | Keep the check leader until process cleanup finishes; accept retained review formats | `4fda20f7d9f335cfe859cb3e01206573ea5e3e08` | `e88a9b42cd35821d81f6c8d706b65f151aadcbe2` |
| dev-004 | Repair fast-gate coverage and supervised cleanup | `4d7e84d669d7a8ce85926deee90336527cad5b05` | `4fda20f7d9f335cfe859cb3e01206573ea5e3e08` |
| dev-005 | Redact migration compatibility command output | `55bef249c7fe719cb53ced5be0b1f37471fd2dcd` | `d57374f43b66b6e070854e8e23ce71755e89a760` |
| dev-006 | Validate route-grant argument forms | `a84646265bce483cee76aa3a51714389be2d1e05` | `d649b6e4d2320e8d3b2bd291ffe72077a06540fb` |
| dev-007 | Validate memory catalog operations | `935cfdd8f92aafb2b54cc4a25d3317e21679efa1` | `025e3fd5cbb9cac0d4776d16e8cfffda959cfa8d` |
| dev-008 | Route documentation catalogs and seal a synthetic pack | `bc1e2d3924990eab93be5657064b9a8f66b68079` | `0e818e86b037688864bc4d2cdfa4de17057dc0d9` |
| dev-009 | Retain failing child-command diagnostics | `1458cef48d289ff735ae1f5b8364c6e260f54e49` | `3dcaabf3fc744daec71f6870c4be31f190841251` |
| dev-010 | Resolve default mod.rs ownership | `1458cef48d289ff735ae1f5b8364c6e260f54e49` | `3dcaabf3fc744daec71f6870c4be31f190841251` |
| dev-011 | Verify retained-stage sources, types and privacy | `1458cef48d289ff735ae1f5b8364c6e260f54e49` | `3dcaabf3fc744daec71f6870c4be31f190841251` |
| dev-012 | Conservatively resolve raw, explicit and macro module references | `28dd0e4435214e5e6d488912792d5986e4b4b774` | `41e3f432df95b290e3a079193ee04fb3e942a7cf` |
| dev-013 | Diagnose trusted native launch prerequisites | `eef257aadc077a259b676e0b24232013162ac997` | `50f0dbb9728a68657c404f0f00cd53feef5e192a` |
| dev-014 | Bind complete committed worker source trees | `f38f9f85fdbc02c1f7d37b0b89e5fb2bef2ba491` | `c16884163206ebacfa8885d3f7416e684c44eb7c` |
| dev-015 | Require resource/network trial outcomes | `ac6985c0877e8e5921bf592ded56595199e0e146` | `a89cc70e18043e88f1eb8fcc978d87cf70cd0093` |
| dev-016 | Count uncached usage probes, including context errors | `dcb2c824e93ac10e0732653c0369b1e1ca072a42` | `0d132ecd6cdd6ffae37e956d5e5d1cf925b579bb` |
| dev-017 | Restore archive modes independently of umask | `360d814bbb19ff3114cf514cba0a7c326888e0bc` | `f0c5ba524372ca8537640f5fd18aeaa0050cb783` |
| dev-018 | Redact checkout logs and refuse other private paths | `e41056d023c81aff372f5acd17cffd84eb9ed30a` | `cff66354016e5d2bb058a277d1150c2033c73eb3` |
| dev-019 | Preserve package permissions without archive ownership | `c666a1da7fac9250f1abc4389bf741ecaf1fdf9e` | `dc99587fa1940516aaf32de88ac4373df9cb47bc` |
| dev-020 | Require nine tools and mandatory records for multi-file acceptance | `99cc4e1630ea1e658a815e82893f650fe736c884` | `d8557c993838ba54a85dba7a364a9495b6d92496` |
| dev-021 | Verify read-argument correction and terminal denial | `d8557c993838ba54a85dba7a364a9495b6d92496` | `ad28b5ca7862a483bb4b9bf74c199d5f994ae43f` |
| dev-022 | Bind command failure artifacts to their creating turn | `ef4a05d823ea0987229e2ab389867b15bb4f8282` | `7b1953d544c6f37cd11bb2c809db8b0aaa8564b7` |
| dev-023 | Measure stable source and executable identities | `a13fd15e22ad659a6a3e60692caa4b7dea6a7a3e` | `eca3f405ceb198d2e01e2764703a554b44f0b381` |
| dev-024 | Parse GPU observations and retain sampled pressure | `fe898a96bbea44d025b986997bd030136e178d1e` | `0dcd13d04c61d2325504b344a13d493788ea2c26` |
| dev-025 | Create new-file and multi-file synthetic repositories | `ea16322f4cc232ababdaa1c839aceeef6bd0a2d8` | `868234916b99f2491b0d9249a2e9366f1736962e` |
| dev-026 | Keep full-documentation collection deadlines finite | `b7964ea2e8afceb32039c27aa3ce3f0b14d2c59f` | `be002e26b79ef68a2b6cbf1897d9abd9a733fbc3` |
| dev-027 | Validate closed extension argument forms | `025e3fd5cbb9cac0d4776d16e8cfffda959cfa8d` | `709f8d7a13bcd467317c0598fc5ba939c4e78892` |
| dev-028 | Add route exports and support-directory options | `e5f3491047fde65c665329696bc011c34e786e54` | `69c99f66a110496c7599111b03b6c45e0fab00fc` |
| dev-029 | Check suspension and continuation observations | `8a3a341ea45eedcc5d2ef3cd66ee514fab3bb3db` | `3709eabe0ddc4ed04027e982dee9f9e204a53790` |

[`coding_eval.py`](../../scripts/coding_eval.py) and the append-only
[`coding_eval_collection.py`](../../scripts/coding_eval_collection.py) pin the statements, editable
path, base and reference file digests, hidden test digest, complete projection
digest, optional original-method selection and exact test counts. Its complete
and legacy task-definition fingerprints are checked by
[`test_coding_eval.py`](../../tests/test_coding_eval.py),
[`test_coding_eval_rust.py`](../../tests/test_coding_eval_rust.py) and
[`test_coding_eval_collection.py`](../../tests/test_coding_eval_collection.py).
Reports bind the append-only module digest and verification summaries bind the
complete collection fingerprint. Expansion adds new tasks;
it must not change these statements, hidden tests, pins or thresholds. The
historical fixes validate the evaluator, never count as model attempts.

New tasks pin an explicit, nonempty `test_names` list of complete regression
methods from their original historical test module. Those selections were made
from changed historical regressions before any model measurement; each task is a
distinct historical source change, never one task per assertion from the same
change. The full hidden module remains byte-identical to its Git pin, including
helpers, imports, assertions and setup/cleanup. The grader invokes the selected
original methods by name. It neither extracts nor rewrites test code. This makes
a narrow scripts-only task possible without claiming unrelated integration tests
whose source/fixtures are absent from the projection. There is no command-line or
environment override. Duplicate/malformed names or count mismatch are input
refusals; a missing method, selected failure, skip or expected failure cannot pass.
Reports include the selection as well as its frozen task digest. The first three
tasks still run their entire original modules, unchanged.

For Python tasks, the workspace is a bounded projection of the base commit's tracked Python
files under `scripts/`. It contains no `.git`, test files, reference patch,
evidence or credentials. This is enough for these tooling tasks, not a complete
checkout or a general repository adapter. Only the declared source file may
change. All other files must stay byte-identical; additions, deletions, aliases,
hard links, special files, extra directories and oversize input are refused.
Candidate bytes are copied through held directory/file descriptors before tests
run. The grader then initializes a fresh, read-only Git repository with exactly
one candidate-only commit for diagnostics that inspect Git. It contains no
original history, reference objects, tests, remotes or hooks. The editing agent
has no access to these grading mounts.

Every grading Git invocation disables automatic maintenance and garbage
collection, preventing detached host maintenance from rewriting the snapshot
or racing cleanup. Infrastructure exceptions produce a failed case report
with a fixed error code and exception type, preserve available raw logs, and
allow validation of the remaining cases. Partial test counts cannot pass.
Pre-case input/operator refusals exit 2 and print a fixed code and exception type;
they never print exception text containing private paths or commands. Missing or
changed frozen blobs are input-integrity refusals. If encountered inside a case,
the report has `cause: input_integrity`, a fixed `input_error`, no test counts and
no infrastructure error. Both `score` and `verify` exit 2 for that refusal. An
input refusal never satisfies the required base test failure.

Counts are copied to the result only after worker and temporary-tree cleanup
succeeds. Transient tree-removal errors are retried, at most three attempts with
50 milliseconds between them. A persistent error produces a failed infrastructure
report, preserves raw logs, removes partial published counts and retains a private
`cleanup.json` naming the exact leftover workspace for operator recovery. It cannot
report PASS or silently claim that the workspace was removed. Interruption also
runs cleanup before propagating; it cannot publish a successful result.

Hidden means withheld from the editing workspace. These development regressions
already exist in public Git history, and their locations/digests are available to
the evaluator operator. They are unsuitable as unseen held-out tasks. The eventual
held-out collection must have separate access and never supply tuning feedback.
Do not call this public development pilot a blind evaluation.

## Known frozen-test race and scoring policy

The dev-003 historical process-stop helper can raise `ProcessLookupError` while
reading a disappearing `/proc` entry. A reference control exhibited this race in
T-311.5 and passed a separate complete verifier run. Its hidden bytes remain
unchanged. The evaluator does not retry individual cases or discard their results.

Before any generated measurement, fix the following policy: every attempt runs
once; this exception remains a test failure and counts as a failed attempt, with
the raw log retained and a visible known-race note. It is never reclassified as a
pass or removed from the denominator. A diagnostic rerun gets a separate output
and never replaces a score. A failed verifier is an invalid control run; repeat
the entire verifier into a new output if diagnosing it and retain both results.
No generated scoring is currently admitted. Future reports must implement this
label and the per-task covered-clause labels before enabling it; this guide does
not claim the reporting integration already exists.

## Rust control

`dev-rust-001` repairs ambiguous workspace path components in the original
contracts crate. Its frozen base and historical fix predate this evaluation.
The complete original integration regression and its 640-case corpus come from
a later pinned repository commit; neither is extracted, rewritten or exposed by
`prepare`. One integration test means one test, not 640 independent tasks. The
statement specifies the public error variants, codes, validation order, Unicode
classes, encoded syntax and shared construction/deserialization behavior.

The frozen corpus does **not** grade every statement clause: it has no trailing
dot/space, invisible-format, `%2f`/`%5c` or uppercase-encoding cases, and its
serialized-input cases cover invalid UTF-8 rather than revalidation of the new
rules. A passing control proves only those historical regressions. Independent
review T-311.4 L1 requires any future generated report to label this scope before
scoring is enabled. Do not change these hidden inputs to fill the coverage gaps.

The editing projection contains 34 original Cargo manifest, lock and Rust source
files (501,816 bytes), without Git history, scripts, tests or corpus. The original
workspace manifests and lockfile stay byte-identical: retaining all members'
source/manifest context lets locked resolution work without updating dependency
versions. Only `kernel/contracts/src/path.rs` may change. Other crates are not
compiled or executed. The grader mounts the original regression and corpus only
after parent-side control admission; the editing workspace never receives them.

Rust grading uses the already installed pinned 1.95.0 toolchain and dependency
registry through read-only mounts at generic paths. No host home, credentials,
Cargo configuration, original repository or GPU is mounted. It creates a fresh
private target directory for each case; there is no candidate-supplied build cache.
Cargo compiles only the hidden integration target with `--locked --offline
--jobs 2`, one code-generation unit and two linker threads, then that executable runs with two test threads. All tests run, with
no filters. Missing/ambiguous compiler artifacts, compilation failure, malformed
counts, skips or unexpected counts cannot validate a control. The parent keeps
bounded logs, deadline, cleanup and input-refusal behavior shared with Python.
Reports include language, corpus digest and adapter digest.

The worker inherits an affinity of at most four available CPUs. Compiler
children have a 2 GiB per-process address-space limit, 128 MiB
per-file limit and 60-second per-process CPU limit; the parent keeps the
90-second case deadline and 2 MiB stdout/stderr budgets. These are **not**
aggregate process-tree quotas. Rust code can forge a textual test result or exit
early; this parser is not an independent result oracle. Generated Rust remains
disabled by the unchanged parent-side frozen-control requirement. This adds an
executable language adapter, not safe generated scoring, a model baseline or
held-out qualification. The [Rust control record](../evidence/t311-4-rust-control-2026-11-03.md)
retains measurements and setup failures.

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

## Historical controls only

The separate [independent function oracle](coding-evaluation-oracle.md) runs eighteen
existing frozen tasks as base/reference controls with candidate execution and hidden assertions in
different processes. It is an executable prerequisite experiment, not a change
to this scorer's admission. Its narrow JSON adapters, missing aggregate quotas and open host cleanup
finding do not enable generated scoring for either command. T-311.15 remains
open after three reviews; T-311.16 records the required bounded cleanup repair.

Until independent result checking and separate aggregate worker quotas exist,
**generated-candidate scoring is disabled**. Both `score` and the internal `grade`
entry check the complete snapshot against this task's frozen base or historical
reference. The task definition itself must match the registered task exactly.
Other Python or Rust bytes, even a comment-only edit or a plausible solution,
are refused before output creation or worker launch with `evaluation.untrusted-candidate-disabled`
and exit 2. This is a temporary restriction, not a generated-candidate sandbox or
an independent score verifier. An exact historical reference is always a control,
never a model attempt.

The guard is outside the candidate process and mounts. Candidate code cannot
remove it by monkeypatching the test framework, printing a forged Rust test
summary, exiting successfully early or writing forged passing counts.
The command-line regression tests include that forgery attempt and verify that
no result directory is created. Low findings L1 (result forgery) and L2 (aggregate
quotas) still block enabling generated scoring; the guard enforces that blocker.
The current lane exposes read-only cgroup controls, so this work does not claim
a separately delegated worker memory, process or CPU quota.

For a single frozen control:

```sh
python3 scripts/coding_eval.py list
python3 scripts/coding_eval.py prepare --task dev-001 --output <new-candidate-directory>
# This unchanged base is a regression control; score must fail with exit 1.
bash /tools/build-slot python3 scripts/coding_eval.py score --task dev-001 --candidate <candidate-directory> --output <new-private-result>
```

`list` prints statements and bases without hidden-test/reference metadata.
`prepare` materializes the base projection without tests or answers. `score`
first checks file scope and snapshots the candidate, then requires an exact frozen
control before running the pinned hidden test module. Exit 0 means that trusted
control passed all pinned tests. Its report records `candidate_kind` as
`frozen-base` or `frozen-reference`, `score_scope` as
`trusted-evaluator-validation-only` and `model_score: null`.
Exit 1 means failed tests, timeout or worker failure; exit 2 is an input or
operator refusal, including disabled generated scoring and input-integrity errors.
Existing output/candidate destinations are never overwritten.
Keep output private: test diagnostics may contain task content and temporary
paths. Only sanitized measurements and raw-log digests belong in public evidence.

The Python scorer launches a separate worker with PID, network and other namespaces,
an empty launcher environment, read-only system runtime, read-only candidate and hidden
tests, a temporary home/scratch and one writable result directory. It mounts no
host home, live repository, model files or credentials. Both Python and Rust
grading branches pass `env={}` when starting bubblewrap: `--clearenv` alone
clears the worker but leaves the launcher's inherited environment readable at
`/proc/1/environ`. The workers receive only their explicit namespace-local
settings (PATH, HOME, LANG, PWD, plus Rust build settings for Rust). The real
grading regression checks every visible process environment using synthetic
host sentinels, then runs the unchanged frozen Python and Rust regressions.
This environment protection does not hide host bind-mount paths in process
arguments or mount metadata; that exposure remains a pre-admission obligation.
Python grading enforces a 90-second case deadline, 60-second CPU limit, 512 MiB per-process address-space limit and
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

For Python tasks, the grader imports candidate Python into the test interpreter
in its confined worker, as the original regression tests do. It withholds tests during editing,
checks their frozen bytes and requires the full expected counts. It is not an
independent proof against a candidate deliberately tampering with Python's test
framework or forging its result. Add independent result checking before using
adversarial model-generated candidates for a trusted scoreboard. The mandatory
pre-launch guard restricts this interpreter to frozen controls; none of the
observed base/reference validation is a generated candidate.

The remaining T-311 work includes reaching at least 30 suitable tasks beyond this
narrow Python projection and single Rust contract task, varied fixes/features/
refactors/test additions, sealed held-out inputs, a bounded/resumable current-harness adapter, actual model attempts and
failure classification, retry/first-attempt accounting and scoreboard-only
held-out execution. No existing product harness, prompt, hidden test or score
threshold is tuned by this pilot.

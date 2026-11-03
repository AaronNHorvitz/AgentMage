# T-311.2: Enforced historical controls and failure lifecycle

Date: 2026-11-03. Tier: core. Builder evidence; independent review remains open.
Base: `4517275f`, which retains the exact-commit independent PASS for T-311.1.
This row does not close parent T-311 or qualify a coding model.

## Delivered scope

The three-task evaluator previously admitted any one-file candidate into the
same Python interpreter as its hidden regressions. The reviewer identified that
candidate code could forge passing counts (L1), and that per-process limits were
not an aggregate worker quota (L2). Both remained prerequisites to generated
scoring. The scorer now enforces that restriction before worker launch:

- A task must equal its registered frozen definition, and every candidate byte
  must equal either the complete base projection or its historical reference.
- Other snapshots are refused with `evaluation.untrusted-candidate-disabled`
  and exit 2, before creating an output directory or running candidate code.
- Reports label accepted controls `frozen-base` or `frozen-reference`, with
  `score_scope: trusted-evaluator-validation-only` and `model_score: null`.
  An exact historical reference cannot become a model attempt.

This guard is outside the worker and its mounts. It does not authenticate
results from generated code or add separate worker quotas. L1 and L2 continue
to block enabling generated scoring. The lane exposes its cgroup controls
read-only; no delegation or resource-bound workaround was attempted.

Two other carried findings are addressed in this batch:

- **L4:** missing or changed pinned blobs produce an input-integrity refusal.
  Within a case, its report carries `cause: input_integrity`, a fixed
  `input_error`, no test counts and no infrastructure error. Both `score` and
  `verify` exit 2. Such a refusal cannot validate a failing base.
- **L5:** counts are promoted only after worker and temporary-tree cleanup
  succeeds. Transient tree removal errors get at most three attempts, 50
  milliseconds apart. Persistent errors fail the case, retain raw logs, discard
  partial published counts and keep a private `cleanup.json` with the exact
  remaining tree for operator recovery. No successful removal is claimed in
  that case. Interruption runs cleanup and propagates without a PASS report,
  including when cleanup also fails; verification cannot continue by masking
  the stop request.

The task revision remains `coding-development-pilot-v3`. Statements, hidden
tests, base/reference/projection hashes and test-count thresholds are unchanged.
The canonical task fingerprint remains
`d5f5e365924ffea8c6b46c50c28d00e116c01f69de57ac0a9672ffa50e05e3c5`.

## Verification

The reserved run completed the unit suite, full frozen verifier and scoped fast
gate successfully. The row remains open for independent review.

| Check | Observed result |
| --- | --- |
| Unit suite | 32 tests, 0 failures/errors/skips, 3.037 seconds |
| Full verifier | PASS, 6.519 seconds; 3 base failures / 3 reference passes |
| Frozen reference counts | 38 tests; 76 invocations including bases; no skips or expected failures |
| Scoped fast gate | PASS, 4.584 seconds; five selected paths and all five commands exit 0 |
| Fast-gate unit rerun | 32 tests, 0 failures/errors/skips, 2.949 seconds |
| Ordinary documentation validator | PASS; 696 Markdown files and all policy invariants, 153.218 seconds |

| Task | Base outcome | Reference outcome | Base / reference wall seconds |
| --- | --- | --- | --- |
| dev-001 | test_failure; 12 tests, 3 errors | PASS; 12 tests | 0.328481 / 0.319923 |
| dev-002 | test_failure; 3 tests, 19 subtest errors | PASS; 3 tests | 0.255987 / 0.372846 |
| dev-003 | test_failure; 23 tests, 2 failures, 3 errors | PASS; 23 tests | 2.410413 / 2.353609 |

Every case records the current evaluator hash below and the correct frozen
candidate kind. The summary has `passed: true`, `input_refused: false` and
`model_score: null`. All cases report zero model attempts and GPU minutes;
tokens are unavailable. These are evaluator controls, not model scores.

| Artifact | SHA-256 |
| --- | --- |
| Evaluator source | `ed52f907f0e631a2eca54d2f04f6f1ec2d3eaad2c50de4aaf9bef4dff377a1ef` |
| Evaluator test source | `a36958b02955fb513e174eabb5282058bfd64531bcbc2b25d7effcff907cce79` |
| Verifier summary (private) | `74c95e92c97e6ae1309d5c80e05c21b6e0e7b3073d8c52fd7df55d7ed440613f` |
| Unit log (private) | `0247171482bdab986fd22a69c3f88041d247bddade9352e7a9d902d44255d62b` |
| Fast-gate log (private) | `9c27c43176bb5c548f7cf5293634f09980112dec5f8f7ce221ca19939dbf1824` |
| Documentation log (private) | `39d9056b608ec3607494e97c629c061af4e741deba66db19c6c23913fa35567e` |

The raw logs, reports, check driver and exact repeat commands are retained in
private lane state and named in the review request. Shared reservation waits
were respected. An earlier unit-only reservation was intentionally cancelled
before any test ran, to consolidate the checks under one lock; its wrapper exit
75 is not a test failure or a measured timeout. Final prose gets separate
Markdown/whitespace checks after the measurements are recorded. No deferred
long acceptance gate is run.

The new regression controls exercise the actual prepare/score CLI with a forged
success candidate, exact-snapshot admission, hidden-input refusal without launch,
verifier exit classification, transient and persistent cleanup failures after an
actual confined worker, stale-count removal and interruption. Synthetic worker
controls explicitly bypass the historical guard only inside the test process;
the CLI refusal test has no bypass. They do not count as generated coding scores.

## Remaining work

Independent result checking and aggregate worker quotas are still required
before generated scoring. The complete 30-task collection, sealed held-out set,
current-harness adapter and actual model baseline remain open. No harness or
prompt tuning, held-out exposure, model/GPU work, dependency change, hidden-test
edit, threshold change, long acceptance gate or supply-chain regeneration is
claimed. No product permission boundary or implementation-status field changes.

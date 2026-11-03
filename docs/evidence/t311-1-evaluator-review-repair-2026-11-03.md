# T-311.1 evaluator review repair — 2026-11-03

Status: builder repair; independent core re-review pending. T-311.1 and parent
T-311 remain open. No harness/prompt tuning, hidden-test change, model attempt,
held-out task selection or coding-score claim is made by this batch.

## Findings addressed

The unchanged [first review](../reviews/dd7349366d01-local-coding-t311-1-evaluator.txt)
for exact commit `dd7349366d0135e06e7be582756c09117ba87fc0` has
**Verdict: FINDINGS**, with two medium findings. Its retained SHA-256 is
`b22a23acfc720fe3c0c67273e9292afe58ed9af1117faf2c6478510d767ed6a1`.
The builder does not change that verdict or close either row.

**M1:** grading Git could spawn detached automatic maintenance, rewriting loose
objects while the worker ran and racing temporary-directory cleanup. The reviewer
reproduced three crashes in six unchanged runs, traced the previously unresolved
builder failure to this race, and observed four passes with maintenance disabled.
The repair passes `maintenance.auto=false` and `gc.auto=0` to every grading Git
invocation. A regression builds 800 distinct source blobs, then checks that the
object store remains byte-identical for at least 1.2 seconds after initialization,
with no pack artifacts. This exercises the loose-object count that triggered the
observed maintenance; it is not a general proof about arbitrary Git versions.

Infrastructure exceptions now yield failed case reports with fixed error codes
and exception types, preserve available raw logs and allow the remaining cases
to run. Partial passing counts cannot turn such a failure into PASS. Operator
refusals retain exit 2; their diagnostics identify the error without including
private exception messages. Injected-failure regressions exercise both paths.

**M2:** the three original task statements omitted function names, output keys,
error codes or canonical encodings required by the frozen hidden tests. Before
any model measurement, `coding-development-pilot-v2` extends each statement with
these interface contracts. Every original statement remains as an exact prefix;
all other task fields, bases, reference sources, projections, hidden-test hashes
and counts remain unchanged. The new definition fingerprint is
`af789424236397cae91efc51653e0020e584dd625a87232374a8165df7fe2863`.
The v1 fingerprint remains in the historical evidence. A new AST-based regression
checks that each module attribute used by its frozen hidden test and absent from
the base module is named in the statement. Python's standard loader-supplied
attributes are recognized as already present. This checks interface completeness,
not general task solvability or candidate correctness.

Low **L3** is also addressed: the actual worker test checks invisibility of the
invoking repository and home, and checks the process environment's complete key
set, instead of checking an environment-specific absent directory.

## Verification

Reserved unit suite: **PASS**, 22 tests, zero failures/errors/skips, 1.623 seconds.
The documented full pilot validation: **PASS**, all three bases fail by
`test_failure` and all three references pass, 38 reference tests (76 test
invocations across bases/references), zero skips/expected failures.

| Task | Candidate | Tests | Failures / errors | Result | Wall seconds |
| --- | --- | ---: | --- | --- | ---: |
| dev-001 | base | 12 | 0 / 3 | FAIL | 0.335186 |
| dev-001 | reference | 12 | 0 / 0 | PASS | 0.314537 |
| dev-002 | base | 3 | 0 / 19 | FAIL | 0.254612 |
| dev-002 | reference | 3 | 0 / 0 | PASS | 0.349847 |
| dev-003 | base | 23 | 2 / 3 | FAIL | 2.266008 |
| dev-003 | reference | 23 | 0 / 0 | PASS | 2.400726 |

Raw per-case stdout/stderr, counts and reports are retained privately. The
aggregate summary includes all input/log hashes and has
`kind: evaluator-validation-only`, `model_score: null`. Summary SHA-256:
`64c23f670c9784d4f15fa9a189abcfca1f2a9689f4286fe34d4ac965433a683f`.

Observed evaluator and regression-test source SHA-256:

- `scripts/coding_eval.py`: `ea48126d0b137af60b8c92a760b49862822cf1b753e17da529886ca6c76a8c3c`.
- `tests/test_coding_eval.py`: `d359ac0d28f9cfcc58f1606c4b69a209a6174eeeb532ffae5135e33644c43483`.

| Case | Raw stderr SHA-256 |
| --- | --- |
| dev-001 base | `09e9ca1755f3d79d8de798320e336a85f53406508ff7fc388ae385de8d19b3ee` |
| dev-001 reference | `c431767fc62ee4e37ec82f34fa7e574c8c6be879d6f2380cd0b1db685060879b` |
| dev-002 base | `a2061e67964e10549844a8d645f0099df3311ea16a459979b17d0d1192635e9b` |
| dev-002 reference | `2cd8b8f655497ef2b151d05ed745ed887bbafd8048d2b5720e555cdfb183db51` |
| dev-003 base | `f4bb7989f664798f54ff8f6716123cba43eb6fa3ee30c5a3bccb5654a6cad4d4` |
| dev-003 reference | `8162ff91c26b7c4e3d94029f14794cad09982ff4a51eb94f8486639f9c5f3442` |

The scoped fast gate: **PASS**, including retained-review format validation,
Markdown lint, Python lint and all 22 tests (1.630 seconds). The reserved ordinary
documentation validator: **PASS**, 694 Markdown files and all policy invariants.
Final evidence prose is checked with scoped Markdown lint and whitespace checks
after recording these results. No Rust source changed or Rust gate was selected;
inherited native baseline failures remain.

The existing Node runtime was copied, with its licence, into ignored tooling
under the shared repository mount for reviewer access. No runtime download,
dependency addition or lockfile change occurred. Observed runtime v24.15.0,
binary SHA-256 `d1de76d8edf2fededf6f8b30d244e2c0529ac607923a018283b77e9c74bd932c`.

The initial new interface-check regression failed because it did not account
for the loader-supplied `__file__` attribute; the checker now accounts for
Python's standard module metadata. No frozen test or task threshold was
changed to correct that new-check false positive.

## Remaining boundaries

Low **L1**, result forgery inside the candidate's interpreter, remains unresolved.
The guide now qualifies exit 0 as the worker's reported result. Low **L2**, process
limits without aggregate memory/pids/disk quotas, also remains open. Both are
prerequisites before any generated-candidate score or trusted scoreboard entry;
this batch only validates trusted frozen historical base/reference bytes.
Public development tasks cannot become unseen held-out tasks. There are still
only three development tasks, no sealed held-out collection and no real coding
baseline. The existing native CLI readiness refusal remains historical; it was
not retried by this repair. Model attempts and GPU use for this batch are zero.

No product trust check, permission, sandbox authority, threshold or test selection
was relaxed. No dependency, licence, original ledger, status-model or SBOM output
changed. No long acceptance gate, model download or release was run. The separate
reviewer decides whether these repairs satisfy the original row's scope.

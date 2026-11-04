# T-311.19: Legacy evaluator environment isolation

## Scope

The [T-311.18 independent re-review](../reviews/8aac044ec207-local-coding-t311-18-environment-repair.txt)
passed the oracle environment repair and
identified legacy-grader inherited environments as low L2, required before
any generated scoring. T-311.19 applies the same protection to the legacy
Python and Rust grading branches. This is running evaluator infrastructure,
not a model score, agent tuning or completion of parent T-311.

The only production source change passes `env={}` to the shared bubblewrap
`Popen` call. Existing `--clearenv` options clear the worker environment but do
not remove the launcher's original environment exposed through namespace PID 1.
No namespace, mount, permission, resource limit, control guard, task definition,
hidden byte, threshold or test selection changes. Reports continue to bind the
exact evaluator source digest; the frozen collection version remains v9.

## Regression and controls

The new real-namespace regression exercises both grading branches using frozen
historical reference inputs. A test-only preamble checks empty PID 1 environment,
exact worker settings and every visible process environment for synthetic host
credential, home and user sentinels. The parent environment contains only those
synthetic values, preventing a failing regression from printing real host secrets.
Rust input paths are resolved before that substitution. Both original drivers
then run their complete frozen regressions; Rust actually compiles and executes
its original test target. The test also checks the host launcher received an
explicit empty environment. Historical-control admission remains active.

The regression first ran against the unchanged launcher: both language subtests
failed. A separate replay of the base `grade_case` function retained each worker's
stderr and confirmed the failure was exactly `launcher environment not empty`.
After the one-argument repair, the same regression passed both branches.

All measured grading and suite commands use the shared build reservation. Raw logs and command/timing
records are private under the lane's `tmp/t311-19-legacy-environment` directory;
public evidence retains only sanitized facts and digests.

| Check | Observed result | Seconds |
| --- | --- | --- |
| Unchanged-launcher regression | Expected FAIL, both language subtests | 0.506169 |
| Base-function replay and stderr assertions | PASS; both environment leaks detected | 0.488040 |
| Repaired real-grading regression | PASS; both branches and frozen reference tests | 3.909019 |
| Scoped fast gate | PASS; 38 tests, Python checks and changed Markdown | 13.790017 |
| All legacy evaluator modules | PASS; 59 tests | 23.068580 |
| Oracle host and worker callers | PASS; 116 tests | 69.202728 |
| Full legacy verifier | PASS; 30 base failures / 30 references pass; 89 reference tests | 43.907228 |
| Full independent verifier | PASS; 15 pairs, 18 methods, 160 reference calls; controls 30/15/15 | 7.693278 |
| Frozen-input and report audit | PASS; 90 reports, 210 log bindings, 30 frozen tasks | 0.152883 |
| Documentation policy validator | PASS; 720 Markdown files and all policy invariants | 143.218429 |

The audit confirms the production diff is exactly the empty-environment argument,
all existing test methods remain unchanged as syntax trees, the eight related
collection/adapter/oracle files remain byte-identical, and the old ledger suffix
is unchanged. Every retained report matches its summary and raw-log digests.
No cleanup or recovery record remains. Source snapshots are retained to ensure
final measurement-only documentation updates do not change tested code.

The first final-document lint found one extra blank line introduced by the
measurement table update; that failure is retained in `final-docs.log`. The blank
line is removed without changing source or tests.

## Retained digests

Artifact names below are relative to the private evidence directory.

| Artifact | SHA-256 |
| --- | --- |
| `before-fix.log` | `3fce2f3fa71d8ace5150e406a2c951356044adb656b5b638dda6fe8cd2e429a2` |
| `negative-observations.json` | `50ba25cd950afeb64b7f1cb94b72bcbea77000334f1a668804c4b88f5a469bdc` |
| `negative-dev-001.log` | `c5d93f626c699a6527dfe3679916bb0fffe9fc14422d642c1b8a5bbc41766997` |
| `negative-dev-rust-001.log` | `c5d93f626c699a6527dfe3679916bb0fffe9fc14422d642c1b8a5bbc41766997` |
| `targeted.log` | `63a50e336384b0803126880c186169b0d4abfdec16f37d22fa6e55ae1576ee45` |
| `fast.log` | `7f3ae49cbe12b1f280cfe77ecd429d4b87e69f011bd56d928eec906a00b21336` |
| `legacy-unit.log` | `0c9d674aa28a91f0767823b6497e51f363b3222c7112f575e31e601346f8a1b3` |
| `oracle-unit.log` | `4003365be85a5f227730d8be3e6d7bc22eef94fe711c34c6018a762f33b241be` |
| `legacy/summary.json` | `551a4668b172f415a0bcde67931bc571480085df2247e255f71700c752ba81ed` |
| `independent/summary.json` | `2f2a5582551b042b2bc98f0e8af90a7b740a1ef0b6b0ffde36e01cc61c36ae12` |
| `source-digests.json` | `f83735943b831f24a6c435bdc88ad1948af075d10d0fc6fa8b9c1aca96e12248` |
| `audit.json` | `b32296a92f0871a91215c9d5665077bb9f38f1f53a95ee6fc2462703229714e2` |
| `policy.log` | `9550f03eb10fe3b4e2a842074618d73a83240a9edc22c917ea3168a8ff33e339` |

Tested `scripts/coding_eval.py`: `6101e2ac15744858312f27f1f7d3fb9e845397eb3d5aacfc0df7d52bb80be751`.

Tested `tests/test_coding_eval.py`: `5550578b266b73376a0eb8488bd95ba62a06feccbdeccd8cd0eb7dfcde92c5c5`.

## Remaining boundaries

This row remains open pending independent exact-commit core PASS with no open
high/medium finding. The separate reviewer is the only acceptance authority.
T-311.18's existing PASS is retained unchanged; it accepts only that prior row.

Host mount-source metadata and legacy launcher command-line paths remain visible;
this change makes no host-path anonymity claim. Their disposition remains required
before generated scoring. T-311.15 stays open after three reviews, with bounded
cleanup repair at T-311.16. Remaining adapters, aggregate and scratch quotas,
protected held-out tasks, coverage labels, the dev-003 race policy and the real
current-harness baseline remain open. Neither grader admits generated candidates.
No GPU/model run, held-out input, tuning, model download, evidence cascade, long
acceptance gate, release or owner-readiness action belongs to this batch.

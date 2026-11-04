# T-311.15 cleanup review repair

Status: historical builder verification; second independent review has FINDINGS.
Implementation base: `36ba8bd3`. T-311.15 and parent T-311 remain open.

## Review and repair

The [unchanged first review](../reviews/1839b2d5b408-local-coding-t311-15-fixture-oracles.txt)
found medium M1: candidate directory modes prevented removal of the writable
scratch tree. Even ordinary wrong answers could become infrastructure errors
and leave candidate data and hidden-test copies in a retained workspace.

Oracle v12 restores owner read/write/search permissions on real directories in
that scratch mount before the existing case cleanup runs. It visits parents
before children and does not follow symlinks or change file modes or hard-link
targets in static-tree probes. The builder's original claim that both namespaces
had been synchronously reaped and could not race this traversal was false:
[review 2](../reviews/7544b49c12d1-local-coding-t311-15-cleanup-repair.txt)
observed descendant filesystem changes after leader reaping. Its medium M1 also
reproduced depth-related cleanup failure; medium M2 rejects the path-based safety
assumption. The third-review repair is recorded separately. Repair never changes the observations already
recorded by the oracle. Reports and counts publish only after cleanup succeeds.
Interruption and failed result reading also unwind through the repair; genuine
repair/removal failures retain failure semantics and private recovery behavior.

The guide corrects the original cleanup claim and clarifies low L2: dev-019
normalizes its two inputs to Path, faithfully matching the frozen fixture only.
Low L1 remains open: host-backed candidate scratch has no aggregate disk quota;
a bounded mount is required before generated scoring. No generated input is
admitted by this repair.

## Verification

All commands ran through the shared build reservation. Times exclude reservation
waits; the targeted timing is from unittest, and other timings include command
overhead. Source/test bytes stayed fixed after the passing gate. All source
verification passed on its first execution.

| Check | Result | Seconds |
| --- | --- | --- |
| Targeted cleanup regressions | PASS; five new test methods | 2.705 |
| Scoped fast gate | PASS; 58 oracle unit/integration tests | 59.204161 |
| Legacy unit suites | PASS; 58 tests | 19.877403 |
| Independent verifier | PASS; fourteen pairs, 17 methods, 159 reference calls | 7.232355 |
| Full legacy verifier | PASS; thirty pairs, 89 reference tests, no skips | 44.049353 |
| Original reviewer probes | PASS; twenty diagnostics | 5.047625 |
| Original reviewer wrong-mode probes | PASS; two diagnostics | 0.567802 |
| Policy validator | PASS; 715 Markdown files and policy invariants | 134.932821 |

The new sandbox cases cover nested 0500/0000 directories both in the fixture
and elsewhere in candidate scratch, all-directory 0555 and all-entry 0644
answers, inaccessible root/nested/scratch directories, candidate exceptions and
interruption after exchange. They assert no recovery record or leftover case
workspace and preserve oracle counts and failure causes. A host-side test
checks that directory/file symlinks, a cycle, a dangling link and a hard-linked
file cannot make permission repair change targets outside the scratch tree.
The injected cleanup-failure test still refuses to publish counts or success.

The original review scripts ran unchanged with fresh outputs and a dedicated
temporary parent: all 22 cases cleaned up, with no infrastructure cause. Both
correct effects plus extra locked directories now pass their original oracle
assertions; 0555 remains test_failure and 0644/0000 remain adapter_policy. Other
forgery, alias, helper-type and confinement diagnostics retain their outcomes.
All failures stay in the attempt denominator. These are controls and synthetic
probes; model attempts and GPU minutes are zero, model score and tokens null.

## Artifact bindings

The audit passes for all 110 reports/log bindings, oracle-owned counts and sticky
flags, all thirty frozen task/source/test/corpus pins, the unchanged six legacy
evaluator files and the original worker/test bytes. The old ledger suffix and
retained independent review remain unchanged. Artifact names are relative to
the private directory in the review request; raw task data stay private.
The collection remains development-only v9, digest
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
No hidden bytes, task definitions, source/test pins, selected methods, thresholds,
controls-only admission, legacy evaluator implementation or model harness changed.

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `0705716138d5fe046c46def8404e0a2782289d529649d88f8af0769c7fff7fa0` |
| `scripts/coding_eval_oracle_worker.py` | `e291fb8f6e5c68a54517134b4f4f9da8a8a8444fa29b7ac035c73f65391f7ad5` |
| `tests/test_coding_eval_oracle.py` | `0065b407ec20ffc123f4f0790e7f24e7e18c4ad60e636c2b8572b4eb9aeaa1fe` |
| `tests/test_coding_eval_oracle_worker.py` | `5a86deb2e1db6d56178af365e04a75c5c505d1305c9a958bf03c8d985acc3e54` |

| Artifact | SHA-256 |
| --- | --- |
| `initial-targeted.log` | `f4721e074f639e551c02e982c14ecd44aa8d608f7dc2a62529d7b06ce1d8f185` |
| `source-digests.json` | `eff4b3b7798795a8f5b53cb674a5d007437d64385eb50eb7afb8173e8d1cc79e` |
| `verify.py` | `63894111085ba6da014357f8429461f6fcc8cf5d2f30f535c29aad1e4c5a0ff4` |
| `fast-gate.log` | `07ca0f5b2248e0e2f312807661b974fd50c56609a1c0a3e6ee034ddd48d660e5` |
| `legacy-unit.log` | `ef7507a2ba2f3fc1594392c1033e097075405eec72b97a7645dd10c1d6c58f75` |
| `independent/summary.json` | `6bc3414e8f14d920aeafeefccf879a3a3e2cbb64cab7694e98cb875028862885` |
| `legacy/summary.json` | `2860321636bdfbbf9c52ce52897882cde06c4cfde9333c0953dade8295680d48` |
| `verification.json` | `9298df2fff5c984068edffc77a02734fc1e06afcbf4b23297a01432c24934fbb` |
| `replay-probes.py` | `217341ecc7dc408fca1e160b4f74903774eeb3fc7d69adcc47319e5d28afbd88` |
| `review-probe.log` | `31df629897672994f18beafbf48ed5b20ef3dc7024b4ddd320804d8396f8e26e` |
| `review-probe2.log` | `66f5c3d2ea44fbc6bfb383740a832af38204e463c79550556d144da7e19a15c1` |
| `review-probes.json` | `8b0653edb005f2657ed7f81105e30b0473e94541cd243ab1b16fe688288de682` |
| `audit.py` | `76f4cf3289c631801fb54614c4ecdfbb9e54c3d0a0dc4870173bd34d44da4a64` |
| `audit.json` | `5e48b77668e6d95dbc368811380ed7d501b88050fbd7a8aeff24bbba22cf4b0e` |
| `audit.log` | `9775836c39cf78af144d696f193084a6add921a9f76eb0ef4b6a1075dcdb981c` |
| `policy.py` | `24e1eb3afac453c6e27d0898c5494b75862452a0f01122fae5e44826a7f095f4` |
| `policy.log` | `5a34219ce8817f55707827cdf92000dd31e1d53cb79aa2706eec8b7b3072a73f` |
| `policy-result.json` | `2197014a365d9ca714879c29877420f5591add0e0adde17899295f935a5721b1` |

## Remaining scope

T-311.15's second independent exact-commit review returned FINDINGS. The original
M1's natural-mode cases were repaired, but depth and race findings remain for a
third review; low L2 is documented and low L1 remains
open before generated scoring. Parent T-311 remains open: sixteen other adapters,
aggregate worker quotas, protected held-out inputs, covered-clause admission,
the dev-003 flake policy and a real current-harness baseline remain outstanding.
No model/GPU/held-out work, prompt tuning, download, supply-chain cascade, long
acceptance gate, release or owner-readiness claim.

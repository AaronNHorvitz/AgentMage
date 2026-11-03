# T-311.7: Independent function oracle

Date: 2026-11-03. Tier: core. Base: `803f1b0d`. Builder checks PASS;
independent core review pending. T-311.7 and parent T-311 stay open.

## Running change

The [adapter](../guides/coding-evaluation-oracle.md) executes the existing
`dev-015` and `dev-029` candidate functions in a separate networkless PID and
filesystem namespace from their frozen assertions. Only bounded JSON function
calls cross anonymous pipes. The assertion process imports the frozen base,
not candidate bytes. Only it has the hidden method and writable result mount.
Both complete original hidden methods, exact counts and all task data are
unchanged. The collection fingerprint stays
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.

The public command exposes historical-control verification only. Host-side
admission also refuses generated snapshots before output creation or execution.
The legacy grader is unchanged. This addresses the result-forgery prerequisite
for two pure JSON functions; it does not establish independent grading of the
remaining 28 tasks, aggregate worker quotas, held-out inputs or a model baseline.
No harness, prompt, permission boundary or hidden test is tuned or weakened.

## Reserved verification

All execution checks below ran under one shared build reservation. Wall times
exclude reservation waiting; no GPU was reserved or model invoked.

| Check | Observed result | Wall seconds |
| --- | --- | --- |
| Scoped fast gate | PASS, eight paths, five commands, 20 new unit tests | 3.477395 |
| Existing evaluator units | PASS, 58 tests | 19.515202 |
| Independent oracle controls | PASS, two failing bases and two passing references | 0.76507 |
| Full legacy controls | PASS, 30 failing bases and 30 passing references, 89 reference tests | 43.68917 |
| Documentation policy validator | PASS, 704 Markdown files and all policy invariants | 138.723503 |

The new unit suite took 1.906 seconds. All 78 distinct unit tests pass. Real
separate-process probes demonstrate that candidate-side assertion patches,
printed passing summaries, attempted result writes, early exit, forged counts,
oversize replies and duplicate keys cannot pass. A candidate returning every
correct reply but then exiting 7 is still failed, even with an oracle exit 0.
Mount probes demonstrate no hidden test, result mount, lane state or host home
in the candidate. Active interruption, second-launch failure, timeout, output
limits and post-execution cleanup failure exercise owned-process cleanup and
prevent count promotion or success. Protocol unit cases cover invalid UTF-8,
truncation, nonfinite constants, sequence/type/field errors and call/wire budgets.

| Independent task | Frozen base | Historical reference | Reference calls |
| --- | --- | --- | --- |
| dev-015 | One assertion failure, three calls | One original method PASS, both exits 0 | 4 |
| dev-029 | Missing function, protocol failure and one test error, one call | One original method PASS, both exits 0 | 6 |

These are two regression methods and 10 reference function calls, not 10 tasks.
The base protocol failure is expected; infrastructure failure cannot validate a
base. Every reference has zero failures, errors, skips, expected failures and
unexpected successes. These existing public development tests do not cover
every clause of their statements, and are not unseen held-out tests.

The unchanged legacy verifier ran 178 invocations with 39.940676 seconds summed
case timers, no skip, input refusal, infrastructure error or truncation. The
known dev-003 race did not occur. Its 30 historical fixes are controls, not model
attempts. Both verifiers retain raw logs, counts and digest-bound reports. Model
attempts and GPU minutes are zero; tokens and model scores are null.

Initial exploratory verification passed both independent control pairs and 17
initial units before the final three test cases were added. Those logs remain
private. No execution check failed unexpectedly. Builder-authored hostile probes
use test-only host mocks of the historical-control guard; the actual command
and host admission still refuse generated candidates. No generated model
candidate was executed or assigned a score.

## Bindings and preservation

All six existing evaluator source/test files are byte-identical to the base.
A separate audit checks all 30 hidden input hashes, 89 expected tests and the
unchanged fingerprint. Before/after snapshots of the four new sources and six
existing sources match, including after verification. The validator precedes
these final queue/evidence prose updates; final Markdown, whitespace, source
binding and added-text hygiene checks follow. No supply-chain regeneration,
long acceptance gate, native host qualification or release is claimed.

| New tested source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `32ad8e8f71815d12b12c25f5bee036202a7f4e70d141468dc515d3e9209e9b34` |
| `scripts/coding_eval_oracle_worker.py` | `15ff330092eecad169f3b4ccd5540b1b34d910b630fde28200a905f630ab33d1` |
| `tests/test_coding_eval_oracle.py` | `24a3420e3561ac6a9338496f0c8aac2cf73ca53a12e3be3b573a7360d4a62a6e` |
| `tests/test_coding_eval_oracle_worker.py` | `bf79879346bae59a5343b8802dd9d0552e0d8b46230f4a3aef6427a6fc43a9ea` |

| Private artifact | SHA-256 |
| --- | --- |
| `fast-gate.log` | `4360fa00e7fb023444473984858f3968fdfa68e110fde7393e67764031b737c8` |
| `existing-units.log` | `e0e68265260437d6d33e8519d73cade159911c052f5b3f7e656bbbf7290fd97e` |
| `oracle-controls.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `final-oracle-controls/summary.json` | `7decb88cd18625384a121f85eba92fa1de9e8c270c01130e1e243c7a2a054f54` |
| `full-controls.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `final-full-controls/summary.json` | `78892e698a86adc49d0c45e6304d371bcb97ca28f5dfbee24079e4ec966faff5` |
| `documentation.log` | `30e9b3abf3284a1ae72563903110c814bf694b810ccf8c127f9c49e739ac5bdd` |
| `checks.json` | `491fcf1f103e5f1ab01cf3c0097166f2f527c99dc45a9ba03243f1aa624b7e54` |
| `source-before.json` | `9746f0f5d7fee3a765aef59e06b64aa9073864aeccf0b9c6e043fbfd2de2e1b4` |
| `source-after.json` | `9746f0f5d7fee3a765aef59e06b64aa9073864aeccf0b9c6e043fbfd2de2e1b4` |
| `definition-audit.json` | `e1d530b2dec9a7c4c23395c4942ff632319ac1609fa40f5580d8a2a2a574ca19` |
| `initial-controls/summary.json` | `d47516df25712bdf3309ea8de73a0d4a015c0d4d9dce9fb1a3a04d8dcaed81e9` |
| `initial-units.log` | `cf28fdd7ad3c22b7608f8a0c5f45873316dcda3fad2fd0c288fc78aeeaae0501` |

The handoff locates private artifacts. All owned test/worker processes exited,
the reservation was released and no new evaluator tree remains. The four
historical evaluator leftovers are untouched. Per-process limits remain distinct
from aggregate quotas; this lane still has no writable cgroup controls. Parent
T-311 also needs broader independent-oracle coverage, fixed reporting labels,
protected held-out inputs and a real current-harness baseline before tuning.
Only the separate reviewer's exact-commit PASS can close this sub-row.

# T-311.18 environment review repair

Status: builder repair; second exact-commit independent core review required.
Row T-311.18 and parent T-311 remain open. Implementation base:
`d49f3b0eb55e9d0f0615b5f4f3a422f1ce2f5833`.

## Finding and repair

[Independent review 1](../reviews/d49f3b0eb55e-local-coding-t311-18-protocol-lifecycle.txt)
has **Verdict: FINDINGS**. Medium M1 found that `--clearenv` clears the worker's
environment while namespace PID 1 still exposes the launching process's inherited
host environment through `/proc/1/environ`, including credential variables.
The unchanged report is retained byte-for-byte; it contains synthetic examples
and neutral lane paths, without actual host identities or credential values.

Oracle v16 starts both sandbox launchers with `env={}`. The fixed absolute
executable and namespace arguments need no host environment. Workers still get
only the fixed PATH, HOME, LANG and PWD. Anonymous argument transport, descriptor
closure, namespace mounts, no-network boundary, permissions, resource limits,
protocol EOF checks and control admission stay unchanged.

The new real-namespace regression checks candidate and trusted roles with only
synthetic host credential, home and user sentinels. PID 1 must have an empty
environment; workers must have exactly the four configured values. Visible
process environments must contain none of the sentinels. No environment values
are printed. The regression fails for both roles using the original launch
function, then passes with the repair. The mocked exec-success and exec-failure
checks additionally require an explicitly empty launcher environment.

## Verification

All checks use the shared build reservation. Times exclude reservation waits.

| Check | Result | Seconds |
| --- | --- | --- |
| Original-launch negative regression | Expected FAIL in both namespace roles | 0.205999 |
| Repaired namespace, argv and descriptor checks | PASS; three methods | 0.304007 |
| Source fast gate | PASS; 70 unit/integration tests | 69.859995 |
| Final full-scope fast gate | PASS; 70 unit/integration tests | 70.451753 |
| Worker caller suite | PASS; 46 tests | 0.109011 |
| Legacy unit suites | PASS; 58 tests | 19.473690 |
| Independent verifier | PASS; 15 pairs, 18 methods, 160 reference calls | 7.725767 |
| Unchanged legacy verifier | PASS; 30 pairs, 89 reference tests, no skips | 43.918811 |
| Frozen input and report audit | PASS; 30 pins, 90 reports, 210 log bindings | 1.520580 |
| Policy validator | PASS; 719 Markdown files and all policy invariants | 144.628592 |

The initial source fast gate checked the implementation, tests, guide and retained
review. The final full-scope gate also checked the added ledger and evidence.
Final logs and policy validation are retained separately for review. The
launcher source stayed unchanged after the initial gate. Before the final gate,
the new test cleanup was tightened to signal a group only if communicate had
not reaped its leader; assertions and thresholds are unchanged. The negative
regression runs
an exact copy of the base commit's launch function in a private interpreter;
it does not alter repository source or any frozen hidden test.

Independent controls retain 30 attempts: fifteen passing references and fifteen
failing bases. Collection digest:
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
Model attempts and GPU minutes remain zero; tokens and model score remain null.
Raw test logs, command records and summaries are private. The final audit
confirms all thirty frozen pins, ninety reports and 210 log
bindings. The worker, six legacy files, all non-launcher host functions,
retained review bytes and old ledger suffix stay unchanged. No cleanup or
cleanup-recovery record remains.

## Artifact identity

Raw artifacts remain private; neutral labels bind their measured bytes.

| Artifact | SHA-256 |
| --- | --- |
| `oracle host` | `5fd46a6e5257bfd0316c32fc0acc94d9a12db512ce3143bfe30fef483dc0bc37` |
| `oracle tests` | `57659fe6d69fd7e7dbeed0188844d2092a074da436cd698209bd93f6e66f5fb7` |
| `retained review` | `164d34c580242a59afac035b48635085e6407f9a5a37ffa0635f65293720ddf7` |
| `before-launch.py` | `04b76d849b183df95fef98479593e1ec8264293015b6720c99359ecf8e929b81` |
| `before-fix.log` | `85b1f2541bb97d3410b323df122d9d4a52c097c35fdf64add7eb66879068c87f` |
| `targeted.log` | `ddc5a9362a739eaf4a0aa0fd4cb8213eb0644000a24142ce1b37dee10c3c9576` |
| `fast.log` | `6a22029626547c57ec6e1c0d727927345484146c9b36eba41d12d198c7c3cd8a` |
| `worker-unit.log` | `0fa28787fbd945c7f83a78ab8322a022f62c24fd12ad657a68a208f051806c87` |
| `legacy-unit.log` | `d71abe6e2a9d21d214c2c0f5e52689628319c0cc2ca8122f9be2a5074687fa62` |
| `independent/summary.json` | `c396f0e46200c2c63b32a5c4624c2779c992597aae0dff1f3eea7c871acee538` |
| `legacy/summary.json` | `f99da97432bd3ba80c37caf6aa8e472dc793999f9268caeda381ccaa613739f8` |
| `source-digests.json` | `606fc47c52294720685a0d6639cd28a9f0f9cd0621a327af5b1af59255875201` |
| `audit.json` | `123785e3dc38200e22e4340ff0bcb1665c896e197d70b1c074e9945334e0ac70` |
| `final-fast.log` | `28635838b1e0bccae7c53150f78a1faf23c515993a3d08afbc2cd5296472e05a` |
| `policy.log` | `0651f15aca109ce2ba74365748b14e029c6a13326dc9a940218edb0f2d233659` |

## Remaining scope

This repairs the same open row and does not restart its review cycle. Second
independent core review must assess the exact repair commit before closure.
Review low L1 remains: mount metadata can expose host bind-mount roots. Explicit
disposition is required before generated scoring. The frozen legacy grader also
inherits its launch environment; its separate repair remains required before
generated scoring. Neither issue is claimed resolved here.

All frozen tasks, hidden bytes, method selections and thresholds stay unchanged.
Deferred T-311.15/T-311.16 cleanup remains unchanged. Generated scoring, aggregate
and scratch quotas, protected held-out inputs, coverage admission, the dev-003
race policy and a current-harness model baseline remain open. No GPU/model run,
held-out input, tuning, download, supply-chain cascade, long acceptance gate,
release or owner-readiness action is part of this batch.

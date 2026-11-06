# T-311.17 complete suite and independent admission, 2026-11-05

The builder has implemented the expanded suite batch. All 30 frozen development
tasks and ten frozen held-out tasks discriminate their historical bases from
references through both the historical and independent assertion paths.
T-311.17 remains open pending independent exact-commit core review. T-311.18 owns
the unchanged-harness local-model baseline; no model score is recorded here.

## Observed verification

All builds and full test runs used the shared build reservation. The final
functional batch and subsequent audit/entry check have matching source hashes.

| Check | Observed result |
| --- | --- |
| Complete suite command, v6 | PASS: 80 discriminating pairs / 160 attempts, 180.288814 seconds |
| Development controls | 30 historical pairs and 30 independent pairs; 89 original reference methods in each path |
| Held-out controls | Ten historical pairs and ten independent pairs; 52 original reference methods in each path |
| Independent reference API observations | 1,029 development calls; 3,016 outer worker API calls across held-out reference methods |
| Scoped fast gate | PASS: 176 tests, formatting/lint, 278.999028 seconds |
| Remaining complete evaluation modules | PASS: 227 methods in twelve fresh processes, including all 83 ordinary worker methods |
| Distinct unit/regression methods | 403 passing methods; no filtering, hidden-test edits or threshold changes |
| Focused adapter tests | PASS: 28 methods plus three complete original builder-authored state-fixture methods |
| Current-source capacity control | PASS: 128 calls, 58.220819 seconds, unchanged resource ceilings |
| Artifact audit | PASS: 160 reports, 940 log bindings, 104 independent worker control methods |
| Inherited resource records | 160 verified records; zero observed OOM-kill counter increase |
| Admission entry control | PASS: all 30 comment-edited development reference snapshots, 33.327109 seconds |

The entry control uses builder-created snapshots and launches no model. Its
`model_attempts` is zero and `model_score` is null. These are executable grading
controls, not a model baseline or coding-quality scoreboard. Focused methods
also appear in the full regression coverage; the totals are not additive.

The source-bound functional run passed before a private audit stopped on an
incorrect assertion that every base must have the same failure cause. The
corrected audit checks independent development bases against their existing exact
registered profiles, including expected candidate-exception profiles. The failed
audit is retained. The corrected audit and admission entry ran afterward with
identical grading/test source hashes; the full functional checks were not repeated.

## Frozen inputs and retained bindings

| Input | SHA-256 |
| --- | --- |
| Complete development definitions | `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570` |
| Private held-out descriptor | `8141e9281c830fe510863403d6c1f137dcb661c2ba6067e68a94a578a13e5c3b` |
| Complete-suite summary | `556f033fc44f42a4bbdfc99331a4d404b2d14e062464d411beb9209b3a1e97e8` |
| Retained artifact manifest | `34d6eb52dd90b8e944ea0456a17bf34c18360d1aa3a7a445ed6f5ff8213c103d` |
| Worker graph implementation | `83c80538da8c626094b08864712ad5b77d3654b4171a3626ffedc05cd46a0480` |

The private manifest binds 24,348 retained files and
41 source inputs, including failed attempts and the final
source snapshots. The source baseline is commit
`f32fba079a33dde6aa26b0b066327c74c9a84e41`; grading source changes are captured in
whole-file bindings. Original frozen statements, bases, references, hidden bytes,
selections, fixtures and thresholds remain unchanged. The ordinary worker subject
also remains byte-identical to that baseline.

The held-out set is ten historical repairs in one Python worker module. It
measures a narrow protocol/tooling domain. No general application-coding quality,
unknown-to-the-builder history, other-platform support or release acceptance is
established. Raw held-out data stays outside editing/model workspaces.

## Assertion boundary and fidelity fixes

The [worker guide](../guides/coding-evaluation-worker-oracle.md) describes the
sealed assertion process, separate candidate namespace, typed graph and bounded
native process fixtures. Complete original methods execute unchanged. Assertion,
result and procfs mounts disappear before fixture execution. Result descriptors
remain private. Callbacks have an audit guard; native launches temporarily attach
procfs only during synchronous trusted setup, with no callback in that window.
The candidate has no hidden tests, result directory, network, capabilities or
process-inspection syscalls. No candidate-authored result object supplies counts.

A flat host loop creates and tears down one method-owned namespace at a time.
The earlier layered coordinator exhausted the existing eight-process allowance;
removing it preserved that allowance. Frame, transcript, scratch, CPU, address
space and process limits were not raised.

The control runs found and fixed adapter infrastructure problems in this row:
older subjects can lack later imported libraries and exception APIs; native
exception relay must preserve `SystemExit.code` and `StopIteration.value`;
invalid JSON literals and large integers must reach original validation; and
list-derived invalid inputs must retain their distinction from an exact list.
Custom fixture constructors remain refused. Callback assertions keep their
expected-value messages private.

The important state-fidelity defect was Python equality in the node-delta cache:
`1 == True` hid an integer-to-Boolean mutation. Boolean data now has a distinct
wire token, and raw Boolean value tokens refuse decoding. This preserves the fast
whole-record comparison without losing type changes. The identical synthetic
regression failed before the fix and passed afterward; its source digest is
`965d3630357eb2a864fdf8ac502f124b0d71ae9f3239e91ce47904ea1eeaf897`.
Synthetic tests also check current, nested and retained message mutations,
mock-history drift, namespace isolation, assertion forgery, startup and cleanup.

Three unsuccessful full closing attempts and all earlier synthetic failures stay
retained. Historical reference failure outputs and fixture structure were used
to diagnose transport fidelity. No model-candidate feedback, prompts, model
selection or coding-harness implementation changed.

The initial combined 364-method regression run retained one memory-accounting
failure: its child inherited a 305,976 KiB pre-exec peak from unrelated preceding
tests. A separate synthetic probe observed a child's 209,156 KiB inherited peak
while its current RSS was 11,980 KiB. Complete modules now run in fresh processes;
the unchanged cleanup regression passes at 91,772 KiB against its original
128 MiB threshold. No method, assertion or resource threshold was removed.

## Admission and remaining scope

The [admission command](../guides/coding-evaluation-worker-oracle.md#generated-candidate-entry)
requires a successful matching complete-suite receipt and bound logs, exact
frozen projections with one editable source, independent assertions and verified
inherited lane quotas at both boundaries. Legacy in-process entry points retain
their generated-candidate refusal. Every missing, failed or ungradable attempt
remains in the denominator; there is no automatic control retry or excluded
infrastructure failure.

The hard inherited quota covers the entire lane, including trusted setup and
other lane work. It bounds aggregate consumption without a delegated worker
cgroup. `aggregate_worker_quotas` remains false; memory peaks remain lane-lifetime
values, not candidate-only peaks. No cgroup limit or reservation changed.

Finite observations do not establish every statement clause or general object
identity. Cross-method module/class continuity, custom list-subclass behavior
and unsupported fixture configurations remain disclosed. The existing usage
adapter's deliberate self-modification and conservative mutation-refusal limits
remain backlog notes with coverage tags; a policy refusal is ungradable and earns
no pass. This record does not claim those behaviors repaired.

Independent core review must assess this exact implementation and its admission
scope before closing T-311.17. The next work remains T-311.18's actual local-model
baseline, then T-303/T-306. No release, owner-ready declaration or parent T-311
closure follows from these controls.

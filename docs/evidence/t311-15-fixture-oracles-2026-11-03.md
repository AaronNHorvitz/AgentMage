# T-311.15 independent fixture observations

Status: builder source verification passed; independent core review required.
Implementation base: `ae45ccfe`. T-311.15 and parent T-311 stay open.

## Delivered behavior

Oracle v11 adds independent grading for the existing frozen development tasks
`dev-017` and `dev-019`. The complete original methods, source/test pins,
selections and thresholds remain unchanged. No generated input is admitted.

For file modes, the candidate recreates the four-entry text fixture in a
separate disposable filesystem. A fixed acknowledgement carries no mode values.
The trusted worker independently inspects actual permission bits through a
read-only mount, with descriptor-relative, no-follow traversal. It replays only
those bits onto its original fixture for the unchanged assertions. Forged mode
maps, missing effects and symlink aliases fail and count. No candidate-selected
reply path is opened. File bytes and tree-shape changes are ungraded by the
frozen method; regular inspectable observations are explicit adapter policy.
Archive integration and special-file refusal semantics remain ungraded.

For extraction arguments, the candidate uses an equivalent local default mock
of the command helper. A closed bounded codec preserves its call order, values,
list/tuple distinctions and lexical Path values. All records validate before
replay, and replay requires the original trusted helper to remain a mock. No
recorded command runs; real extraction and helper error propagation are ungraded.
Malformed frames fail as protocol; unsupported objects, nesting and counts fail
as adapter policy. All failures remain in the attempt denominator.

Only the mode task gets the new writable disposable mount. Candidate code stays
separate from assertions and result mounts; no network, host home, credentials
or publication authority is added. The existing cleanup removes that mount's
backing tree before counts publish. Aggregate memory/process/CPU/disk quotas
remain unavailable, so generated scoring remains disabled.

The [guide](../guides/coding-evaluation-oracle.md) also clarifies the preceding
review's two low findings: candidate-raised worker protocol exceptions may choose
a failure cause, and extra mock configuration is outside the two frozen capture
fixtures. Neither label establishes the provenance of an adversarial failure.

## Verification

Checks use the shared build reservation; command times exclude reservation
waits. No runtime source or test changed after the passing final fast gate.

| Check | Result | Seconds |
| --- | --- | --- |
| Scoped fast gate | PASS; 94 oracle unit/integration tests | 55.853986 |
| Legacy unit suites | PASS; 58 tests | 19.555601 |
| Independent verifier | PASS; fourteen pairs, 17 methods, 159 reference calls | 7.223406 |
| Full legacy verifier | PASS; thirty pairs, 89 reference tests, no skips | 43.993429 |
| Policy validator | PASS; 714 Markdown files and policy invariants | 144.498839 |

The separate 94-test source run also passed (55.009 seconds). Fourteen independent
bases retain exact expected failure/count profiles, and all fourteen references
pass: 28 counted controls. The full legacy verifier passed on its first run for
this batch; no hidden tests were skipped or changed. Model attempts and GPU
minutes are zero; model score and tokens are null.

Twelve committed integration contrasts exercise both graders: ten agree, while
the two deliberate unittest assertion-patching attacks pass the legacy grader
and fail the independent one. Other new probes reject forged filesystem claims,
a fabricated stat implementation and aliases, and verify candidate mount/network
confinement. The original two new task references still pass.

An initial diagnostic log is retained: 91 tests and two new contrast-expectation
failures. Those expectations wrongly required the deliberately patched legacy
grader to reject assertion forgery. They now require the observed distinction:
legacy passes, independent fails. That evolving diagnostic run is not a pinned
acceptance snapshot. No frozen input, hidden assertion or threshold changed.
The design was also corrected before final verification to inspect actual
filesystem bits independently, rather than accepting candidate-reported modes.

The policy validator passes. Final Markdown, whitespace, digest and public-text
hygiene checks pass after the evidence update. No source/test bytes change.

## Artifact bindings

The audit verifies all 88 final reports and their log bindings, oracle-owned
counts and sticky flags, every frozen source/test/corpus pin for all thirty
tasks, the unchanged six legacy evaluator source/test files and the untouched
old ledger suffix. The collection digest remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
The retained T-311.14 PASS is byte-identical to its independent original.

Artifact names are relative to the private directory in the review request.
Raw task data and diagnostic logs stay private. Source digests bind the final
runtime/test bytes checked by the green gate.

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `03afb3ad8b068170b3d60b3ef434fda7559648f3aecf0880244cf67389eaab50` |
| `scripts/coding_eval_oracle_worker.py` | `e291fb8f6e5c68a54517134b4f4f9da8a8a8444fa29b7ac035c73f65391f7ad5` |
| `tests/test_coding_eval_oracle.py` | `121cee5c0afc0a1356d6053c1a0f7b4b7327edcdb69868131f7c5a4f082f2465` |
| `tests/test_coding_eval_oracle_worker.py` | `5a86deb2e1db6d56178af365e04a75c5c505d1305c9a958bf03c8d985acc3e54` |

| Artifact | SHA-256 |
| --- | --- |
| `initial-tests.log` | `b92ed0d012bf9c241bb7da8371eddd315735bea18bb392d826bc7500a351b2e4` |
| `tests.log` | `9e5234e10b4820e35395bb5cd250841b497e23f0462089564a8f17a8d1340695` |
| `fast-gate.log` | `957f8b62e0bb1799b292444359c10f06f451d16ae5d3e03cabea132c55686eb2` |
| `legacy-unit.log` | `00a89c96046940fc1f9c8fb88e3c88f7e209f7d011a501805a35fee4a4705d2e` |
| `independent/summary.json` | `c14eb3cf5bb3ca92efc9e64030f231f9351dca84cd0b4f857b730e7e7b16a1c8` |
| `legacy/summary.json` | `e91c381d65d93b450937d1a4eb102cc7752413a520c84e3c9f3b1476ac9ca6e5` |
| `verification.json` | `660303b03dc10b90f526a6e5879819a9a5ee21c9dd878d5ef6041bdfba5068d6` |
| `source-digests.json` | `d09fd4975e3b36219324fc6af4cfa3ba92116b9cb8bd74a92cf907ab5b743e42` |
| `verify.py` | `e4fe722a3f0ae475e34461cb0b6ee4b317e81e1ff19a54b0ca6296f89418aa3d` |
| `audit.py` | `9609471f79344ebce16078cc82aae561291bb64895a23f886dc62354dbfb27e4` |
| `audit.json` | `81b91b4714fc34f03bbbedb15ba103622f3597a00dcbb5eaa631bf8f33878885` |
| `audit.log` | `c5850ccb3117b00f2b2594f2e44e0f5207ea3f8b5868d19a1516f64086d725a9` |
| `policy.py` | `9e2b73627955e4d959328e0f45fe05bcff5ec5a958c48b279816f52a8163cb62` |
| `policy.log` | `512eb4af822b4f379e6e837b11f16eb270b072bfa441d2a790ec94d1136e6ce2` |
| `policy-result.json` | `486212f79ad759a267066f9a2dd4e1b051a4635d3822dace10487789129a1531` |

## Integrity and limits

The collection remains development-only, v9. These are measured historical
controls and adversarial diagnostics, not model attempts. Independent grading
covers fourteen of thirty development tasks. Sixteen adapters, aggregate quotas,
protected held-out inputs, covered-clause admission, the dev-003 flake policy and
a real current-harness baseline remain open. No model/GPU work, held-out run,
harness tuning, download, supply-chain regeneration, long acceptance gate,
release or owner-readiness claim.

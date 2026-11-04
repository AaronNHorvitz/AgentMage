# T-311.13 bounded predicate observations

Status: builder verification passed; independent core review required.
T-311.13 and parent T-311 stay open. Implementation base: `be8f70d4`.

## Behavior

This batch addresses lows L1, L2 and L3 from the exact-commit
[T-311.12 independent review](../reviews/3a545e857648-local-coding-t311-12-truthiness-repair.txt).
The independent oracle advances to v8. Predicate observations cross inside a
single `value` envelope. Candidate map keys such as `adapter_policy`,
`candidate_exception`, `value`, `tests` and `passed` remain observation data;
only the adapter's outer fixed tags affect failure classification. The original
assertions still own counts and verdicts in a separate confined process.

The scalar predicate now carries bounded JSON lists and maps as well as finite
scalars. Its original assertions decide truthiness. The map predicate keeps
string-keyed maps with bounded JSON values. Both preserve a 16-container-depth
bound on each observation value; maps bound each value independently. Nonfinite
floats, foreign objects, unsupported types, cyclic and deeper containers receive
an explicit adapter-policy refusal. Exact Boolean types remain ungraded.

Every supported result type declares bounded response encoding as adapter policy.
An ordinary adapter reply exceeding 64 KiB or refusing JSON encoding, including
the interpreter's integer-digit limit, becomes the existing fixed policy tag.
It no longer crashes the worker and masquerades as malformed protocol. Direct
hostile frames still undergo independent schema, sequence, byte, finite-value
and depth validation. A forged tag can only fail its own counted attempt;
unknown result types cannot declare policy. Sticky failures, call/transcript/log
limits and cleanup ordering stay intact. Candidate exception payloads are never
reconstructed in the trusted process.

The [guide](../guides/coding-evaluation-oracle.md) and report coverage distinguish
these bounded transport refusals from frozen assertion failures. No hidden
method, complete test blob, task statement, base/reference source pin, selected
method, threshold or historical control profile changes. Legacy evaluator source
and tests stay byte-identical to the base. Updated adapter tests use the new
wire envelope and preserve explicit raw-frame forgery/refusal checks.

## Verification

All heavy checks used the shared build reservation. Times exclude the outer
reservation wait. The initial 72-test suite passed in 34.868 seconds; the scoped
gate repeated it after the final runtime source edit.

| Check | Result | Seconds |
| --- | --- | --- |
| Scoped fast gate | PASS; 72 oracle tests | 36.555371 |
| Legacy unit suites | PASS; 58 tests | 19.444698 |
| Independent CLI verifier | PASS; ten pairs, 13 methods, 149 reference calls | 5.050151 |
| Legacy CLI verifier | PASS; 30 pairs, 89 reference tests, no skips | 44.041221 |
| Persistent confined probes | PASS; 36 probes, 20 legacy contrasts | 10.462277 |
| Documentation policy validator | PASS; 711 Markdown files and policy invariants | 129.701475 |

The independent verifier retains twenty invocations: ten failing bases and ten
passing references, with unchanged exact historical control profiles. The full
legacy verifier retains all thirty failed bases and passing references. These
are diagnostic controls, not generated attempts or model scores.

| Persistent probe group | Observed result |
| --- | --- |
| List, map, integer and null predicate truthiness | Both graders pass on identical candidate bytes |
| Inverted scalar and map predicates | Both graders fail; independent cause test_failure |
| Reserved map keys, including a false extra check | Both graders agree: five passing variants and one assertion failure |
| Sixteen nested containers | Both graders pass both predicates |
| Seventeen nested containers | Legacy passes; independent adapter_policy failure is declared and counted |
| Oversized, huge-integer, nonfinite, foreign-object and cyclic observations | Independent adapter_policy failure, candidate exit 0, counted attempt |
| Oversized helper and verification replies | Independent adapter_policy failure, candidate exit 0, counted attempt |
| Direct forged policy, exception and extra-field tags | Fixed policy, exception and protocol failures respectively, all counted |

The 36 probes include twenty legacy comparisons on identical snapshots. They
patch only historical admission inside a private diagnostic process; production
admission and every frozen assertion remain unchanged. Unit tests also check the
exact 64 KiB response boundary, sticky failures and refusal of unknown policy
types. Observation-depth refusals remain explicit adapter scope; they are not
misreported as failures of the frozen truthiness assertions.

All 136 retained report/log bindings were audited: twenty independent controls,
sixty legacy controls, 36 independent probes and twenty legacy contrasts. The
independent reports match oracle-owned counts and final worker/source hashes.
All thirty base/reference source pins and complete hidden-test hashes verify,
including the Rust control's separate hidden revision and corpus digest. Six
legacy evaluator source/test files are byte-identical to the base. Collection v9
remains `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
The retained T-311.12 review is byte-identical to the independent original.

The first audit helper incorrectly looked for the Rust hidden test at its source
reference commit. The helper was corrected to use its existing `hidden_revision`
and corpus pin; its failed log is retained. No evaluator, test, pin or threshold
changed for that correction. The passing legacy verifier already used the right
revision. Owned runtime processes exited and the build reservation was released.
Four historical evaluation scratch trees remain; this batch cleaned its own trees.

## Source and artifact bindings

Artifact labels below are relative to the private batch record identified in the
review request. Raw source projections, logs and probes remain private.

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `b196b045447efa4143e51bc4c1366c0f2aff3f896aa4571688d6fdcc7ad31393` |
| `scripts/coding_eval_oracle_worker.py` | `8b60babe115840a0a9dd7ee0dee6b595c4d7bfb592c95716f97beaaee38d43b9` |
| `tests/test_coding_eval_oracle.py` | `d4d9f90c3b182761c1f2257f106981e2ea0bc3d8ee15d74090aa78d2a668a6e1` |
| `tests/test_coding_eval_oracle_worker.py` | `89f054af086c3b1dae9200008156ad4f522635ac8a1eb1d2981bd4cdd0c911aa` |

| Artifact | SHA-256 |
| --- | --- |
| `initial-tests.log` | `cb1665335db7a6cf4b64d1db2fe8143c98bc02603797341feff2752526da140c` |
| `fast-gate.log` | `fd2b23e60f316c360f493b0fad7c5a3f81edbccd8a99bb9cf3fecb8ece7e3e8a` |
| `legacy-unit.log` | `68c1ac3f76c709329d310e855b9c0706cae3d9121fbbb624560981676701a5b2` |
| `independent/summary.json` | `fcb731bafe97cd7bf3ba54baf069f7090a3a6f889b21ae7275d6cbd4f5f4e39d` |
| `legacy/summary.json` | `de7230ef27811dd8348129b221adaf73af9d86fc9b74764585bf3a875298da41` |
| `probes/summary.json` | `e28f1e635df7a2f28dec2d3707dd7868eb8405eedf74b6e4a68aff0aad7983ff` |
| `probes.py` | `7d429f5d4df8b65610e3393c15b7459ad8c15b1df05f49801e5c108a4eea4649` |
| `policy.log` | `0b6816d24b8f5d979bfb0a77be55d7000fbb9b5678ff8d430634d1ffa04788ef` |
| `verification.json` | `a51d8ec7e7e4a8392f28b33ac5d414c77754b47c8960cae4f544739d24bc0c32` |
| `source-digests.json` | `a0f1cba472969869c762631749287a2ce31a67ffe8d92d2c9f8acd1bec7ae228` |
| `verify.py` | `82f396e131c562d9917aeba2f34426e5a5664a4cf71cbb2f0ac355cb9aa62c62` |
| `probes.log` | `5a106c1a94c70a18b09f198474cda56de05f4603e0b70bd03527c78d46308554` |
| `audit.json` | `22cc8a038172a5a217be82a39e84dfabc68fae68399f04a92dd777255ea7feff` |
| `audit.py` | `7dab717975101f6319a74739041375e3c8ce0e3fd1939e60ac0ea26171a02244` |
| `audit-first.log` | `d0db37b4aeee7fb1cc7c02ac749e568abbef2d0c02df46295b8371a41805850d` |

Only queue and evidence text changed after runtime verification. Final Markdown,
whitespace, source-digest and added-text hygiene checks precede the commit.
Builder verification does not close T-311.13 or parent T-311.

## Limits

All tasks remain development tasks. Diagnostic probes patch historical admission
only in their private host process; production admission continues to refuse
generated source before launching workers. This batch admits no generated score,
model, held-out input or tuning. Twenty other tasks still need independent
assertions; aggregate worker quotas, protected held-out inputs, covered-clause
admission and the current-harness real-model baseline remain open. Path return
container/element types remain explicitly ungraded. No GPU/model run, model
download, long acceptance gate, supply-chain regeneration, release or owner
readiness is claimed. The deferred native-host blocker remains T-301.1.

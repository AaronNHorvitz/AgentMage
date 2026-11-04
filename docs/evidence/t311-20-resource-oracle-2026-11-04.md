# T-311.20: Independent synthetic GPU observation oracle

Status: builder verification passed; independent exact-commit core review
required. Implementation base: `ab6508903c66e0b6d820640ac08cc188ec96cf22`.
Row and parent T-311 stay open.

## Scope and limits

Oracle v17 adds frozen development task dev-024. Its two complete original
methods stay in the trusted namespace. Only finite completed-process stubs,
constructor arguments and GPU mock values reach the candidate. The candidate
runs its own helper and guard; tagged observations return to the unchanged
assertions. Guard instances retain their candidate-owned state between calls.
Only the fixed trusted HarnessError category is reconstructed, without candidate
exception text. Other exceptions remain fixed counted candidate failures.
The [guide](../guides/coding-evaluation-oracle.md) describes the closed schemas,
observation policy and ungraded clauses.

The original tuple equality and malformed-output refusals grade helper values;
the pressure assertion grades sample truthiness, error and peak values. A list
in place of the GPU tuple fails. A falsey list returned by sample passes the
same original truthiness assertion in both verifiers; exact Boolean sample
return type is ungraded. Reserved map keys remain inert observation data.
Objects, unsupported tagged values, excessive depth and encoded size are
adapter policy, with failed attempts retained. Unknown fields/operations,
invalid or duplicate instance numbers and live descriptors are protocol refusals.

All GPU observations are synthetic. No GPU device, model, lock acquisition,
process cancellation, actual memory measurement or complete resource admission
is demonstrated. Subprocess invocation arguments, exception messages,
unselected guard state/transitions and unselected statement clauses remain
ungraded. Candidate code controls its own interpreter and observations; finite
assertions establish only the reported behavior. Generated scoring stays disabled.

## Measurements

The four focused protocol checks are lightweight unit tests. Every sandbox and
full-suite command below ran through the shared build reservation. The initial
queue wait is excluded from command timings. Final document lint covers the
completed measurements. Source/test bytes remained fixed throughout the reserved
batch and match the retained tested-source digests.

| Check | Result | Seconds |
| --- | --- | --- |
| Focused protocol checks | PASS; four new unit tests | 0.126084 |
| Targeted sandbox/protocol checks | PASS; 52 tests | 4.618653 |
| Scoped fast gate | PASS; 122 oracle/protocol tests, Python and Markdown checks | 78.454608 |
| Legacy unit suites | PASS; 59 tests, no skips | 23.877904 |
| Independent verifier | PASS; 16 valid pairs, 20 original methods, 166 reference calls | 8.284631 |
| Full unchanged legacy verifier | PASS; 30 valid pairs, 89 reference tests, no skips | 44.226018 |
| Retained diagnostic probes | PASS; ten variants, eight legacy contrasts and one confinement probe | 4.378448 |
| Integrity audit | PASS; 111 reports, 265 log bindings, 30 frozen tasks | 0.277174 |
| Policy validator | PASS; 721 Markdown files and all policy invariants | 137.95632 |

The independent verifier retains 32 control attempts: 16 passing references and
16 failing bases. The new base has two missing helpers, two original errors and
two calls; its reference passes both original methods with six calls. The
control outcome profiles were confirmed by the actual namespace runs. Every
unsuccessful diagnostic variant remains a counted failure. No failure, skip,
expected failure or threshold was removed. The existing first verifier test's
expected collection counts increase for the new task; all other existing test
methods are unchanged.

The audit confirms the six legacy source/test files, all task dictionaries,
source/hidden pins, method selections and thresholds are unchanged. The frozen
collection fingerprint remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
Existing sandbox/launcher/cleanup and grading functions are unchanged. Original
T-311.15 and its T-311.16 cleanup finding stay open after three reviews; no fourth
review or cleanup repair is attempted. The old ledger suffix and the retained
T-311.19 independent PASS review remain byte-identical. No recovery record was
produced by these retained runs. All recorded tests passed on their first run.

No model, GPU, held-out input, prompt/harness tuning, model download,
supply-chain regeneration, long acceptance gate, release or owner-readiness
operation occurred. Model attempts and GPU minutes remain zero; model scores
and token measurements remain null. Parent T-311 still needs remaining adapters,
coverage admission, aggregate/scratch quotas, host-path metadata disposition,
protected held-out inputs, the dev-003 policy and a current-harness real baseline.
The model-lab inherited-environment review low returns before T-312.

## Artifact identity

Raw reports and logs remain private. Neutral labels bind the measured bytes;
no private host path or raw task transcript is published.

| File | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `540ac3e51704d16c143454061c080d3a46a8a7aa497d380361b17e6282bf7cc5` |
| `scripts/coding_eval_oracle_worker.py` | `bde70eda12642ba5565f7f19951f60bc911fc920cb12c8128d2c40a630ea83a4` |
| `tests/test_coding_eval_oracle.py` | `c1ca0b1c65a87d146c04777657ef1467cb26b12cc211e16bdd6327e81e21aaa0` |
| `tests/test_coding_eval_oracle_worker.py` | `4f39f7a34823c765dac0cd80e2c568e803b39a9afa9101f7fa46659bdf8c3264` |
| `source-digests.json` | `c7e13cb24e5df084998d5ef749cad6de13d366d87b7dbf0171044e04f67b9b8b` |
| `protocol-targeted.log` | `5d6c72c54d0973eb12910f9f8e360ebb5ba14edbdb69063db1bee46e760468a2` |
| `targeted.log` | `f4a0131a877ccad1f1a8f3b66eb1af1475f108e7eb7fe1273a69390268270168` |
| `fast.log` | `08ef38a0f850b5d7198bb18fcc53b2d1d18df775e4ad691f92eaf3ca61cd3f5a` |
| `fast-result.json` | `04972d7faf4ba5370929d68ae2ec30f4f220794f1ba269b19ea9e91b3f79087b` |
| `legacy-unit.log` | `d77822a66e3c9cbb911605f174ba0b832f480005e8835ddf47c0927b670b41bd` |
| `legacy-unit-result.json` | `b2ee35975660d7ffe4d0cadd566957d2f232ca33e94d51579b804a1add520f41` |
| `independent/summary.json` | `55ebc279a4db77789ee257c3ce0d7a006febc88efb73ed2c18b59a6a0aa4b4b1` |
| `independent-result.json` | `8dd9621b02f22789f953c9b3ac6f8856bc4810e83e3e165a929a116922b69c46` |
| `legacy/summary.json` | `25c102d2fc0ad6982b6e04c3cc936c3eff2ba894d3047c5a20b244cf0ec5c301` |
| `legacy-result.json` | `192901c435e97927ef48cbe891ad021550188630c886e38efad9df264846f65c` |
| `probes.json` | `57d525b1bc15c7afce9a66df782cf3ba24087f8da05738b2cd66bc4235f9747e` |
| `probes-result.json` | `0e6cd8f9920066fced87596a7d917b2e81d4a317a7bc63f707799e4101f30fa5` |
| `audit.json` | `bbae8c4e132176a57b1a0bc6984df237968f4f9a806e755aa9db334948ecaaf9` |
| `policy.log` | `2c600c8cb8a9b889bba16227549730f5c6465958ff45146774f16778ef4fae3b` |
| `policy-result.json` | `f30f52b286552ce6372cbbd9d8fb07fb46d0fbbd3e0d6dcc24754580b80b39d9` |

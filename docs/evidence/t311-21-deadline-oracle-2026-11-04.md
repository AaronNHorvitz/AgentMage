# T-311.21: Independent collector deadline oracle

Status: builder verification passed; independent exact-commit core review
required. Implementation base: `e7548fb738169012d532762add728a16717b0703`.
Row and parent T-311 stay open.

## Scope and limits

Oracle v18 adds frozen development task dev-026. Its two complete original
methods stay in the trusted assertion namespace. The candidate receives only
finite completed-process stubs, executable resolutions, parsed arguments and
one timeout stub. Candidate command declarations cross as typed data before
assertions, preserving their list/tuple types for the original count/order checks.
Candidate return values and observed calls cross in a separate closed envelope;
reserved keys inside those values remain inert data.

The trusted process validates every observation before replaying into original
MagicMocks. It refuses real functions as replay targets. No candidate object,
callable, exception class or traceback executes in that process. A TimeoutExpired
category reconstructs only the trusted standard-library exception with fixed
local fields. Output write calls are replayed even when a timeout propagates,
so writing before re-raising still fails the original assertion. Swallowing the
timeout also fails. The [guide](../guides/coding-evaluation-oracle.md) describes
the schemas and explicit coverage limits.

Wrong deadline, argument-container, capture and count variants fail the original
assertions. Swallowed timeouts and writes before timeout propagation also fail.
Integer truthy capture and inert record maps pass both verifiers, as the original
assertions allow. Unsupported values are adapter policy; private exception text
does not enter the wire log. A real candidate namespace probe confirms no hidden
test, result, lane or GPU-device mount and no reachable loopback network. Generated
source remains refused before output creation outside diagnostic-only tests.

These fixtures execute no collector commands, compiler or long documentation
gate. Record contents/digests, missing-executable handling, minimum command count,
timeout fields, unselected main paths and other statement clauses remain ungraded.
Unsupported containers/subclasses or undeclared resolution requests can be refused
even when original assertions would accept an implementation. Call observations
are candidate-controlled data; this is not proof of real command effects. The
guide also clarifies the existing sticky-exception count behavior: alternate
candidate method errors can differ from the original single-process suite.

## Measurements

The six pure protocol tests are lightweight unit checks. The eight focused
tests and the complete validation batch ran through the shared build reservation;
queue time is excluded. Focused time is the test runner's reported duration;
other durations come from retained command wrappers. No source/test edit occurred
during the complete batch. Final document lint follows measurement recording.

| Check | Result | Seconds |
| --- | --- | --- |
| Pure protocol checks | PASS; six tests | 0.14013 |
| Focused checks | PASS; eight tests including real namespaces and legacy contrasts | 4.357 |
| Scoped fast gate | PASS; 130 oracle/protocol tests, Python and Markdown checks | 83.331276 |
| Legacy unit suites | PASS; 59 tests, no skips | 23.042287 |
| Independent verifier | PASS; 17 valid pairs, 22 original methods, 169 reference calls | 8.789816 |
| Full unchanged legacy verifier | PASS; 30 valid pairs, 89 reference tests, no skips | 44.237442 |
| Retained probes | PASS; ten variants, eight legacy contrasts and one confinement probe | 4.468137 |
| Integrity audit | PASS; 113 reports, 271 log bindings, 30 frozen tasks | 0.257156 |
| Policy validator | PASS; 722 Markdown files and all policy invariants | 139.088802 |

The independent verifier retains 34 control attempts: 17 passing references and
17 failing bases. The new base has one original deadline assertion failure;
its timeout method passes. Both controls run two original methods and three
protocol calls, including the command-declaration observation. The reference
passes without skips or errors. Every unsuccessful diagnostic remains a counted
failure. No failure, threshold, hidden method or test selection was removed.

The audit confirms six legacy source/test files, all task definitions and pins,
the old ledger suffix and the retained T-311.20 review are unchanged. All existing
test methods remain unchanged except the verifier test's expanded collection
counts. The collection fingerprint remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
Sandbox/launcher/cleanup and host grading functions are unchanged. No cleanup
recovery record was produced. All recorded verification commands passed first run.

T-311.15 and T-311.16 stay open after three reviews; no fourth review or cleanup
repair was attempted. Generated scoring remains blocked on remaining adapters,
coverage admission, aggregate/scratch quotas, host-path metadata disposition,
protected held-out inputs, dev-003 policy and a current-harness real baseline.
T-311.20 equality/subclass and sticky-count lows remain recorded in the ledger;
model-lab inherited environments return before T-312.

No model, GPU, held-out input, prompt/harness tuning, model download,
supply-chain regeneration, long acceptance gate, release or owner-readiness
operation occurred. Model attempts and GPU minutes are zero; model scores and
token measurements remain null. This row needs independent exact-commit core PASS.

## Artifact identity

Raw reports and logs remain private. Neutral labels bind the measured bytes;
no private host path or raw task transcript is published.

| File | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `9d4d5a28221cd2d45ca4fea37c8564143e055e7aca3ef4b35c9845bc47765dc7` |
| `scripts/coding_eval_oracle_worker.py` | `9d2642c3f2da5d88f635b6c94ea783655a9315f04172bb5828e32f8e14993de9` |
| `tests/test_coding_eval_oracle.py` | `80a92447962c123587b21d07d699959331805c0c62182171015bd48ffcdf2352` |
| `tests/test_coding_eval_oracle_worker.py` | `5120cd304c08b1eff247d0f9ac3a070a5273477d133baa84e62a66a66fb44023` |
| `source-digests.json` | `be6415964bcc77ff9619a70897c5700e7998b3cfac60c9aff8d86a9004ab4a96` |
| `initial-focused.log` | `6c32f4cc98363bbc93bb3f708df81aea88ea2811827a19c251d2b4da25e83652` |
| `protocol-targeted.log` | `3efe8c691e1776c702f96c7fe5040cb15b5323ec56035ba4d3ad4a560bb9f8f7` |
| `protocol-targeted-result.json` | `cb41b55a8abefc073aa171dc43f2708a4f5b2dccef164079602c1939a92e6da1` |
| `fast.log` | `ff70147812226164b2fd0ad40b9a58735675e3390acf20438c23bc29f157771d` |
| `fast-result.json` | `9ea608918f9867238e5499424d79400360a2faa9227754b34d387191ffe244c0` |
| `legacy-unit.log` | `dbc220c134c3eab5ca9a9749ea6ec90e33774ef827b997f4652e36f0964cf9a8` |
| `legacy-unit-result.json` | `d4e74bb95aa06f5027f1063eaeda1f689c6b5649359211d552c677261c9eec96` |
| `independent/summary.json` | `8f4fe709ffff5195c7ca4cf0cdd54ce5037e7adce1b1a5d2ee805d7a7119063d` |
| `independent-result.json` | `e3fc9cd60f2713e3b5cb48357348f06b53fcbdc2dfdb9371938bcc05526d10c8` |
| `legacy/summary.json` | `22b1eee5f000b2c65e36973dc16cbe5bc56da2c85923acaef6349e802cd0a412` |
| `legacy-result.json` | `8038a29f048b1b2d6aecde7b2acb82b1fe8eef4992c704b46821303a10db5cbc` |
| `probes.json` | `57d525b1bc15c7afce9a66df782cf3ba24087f8da05738b2cd66bc4235f9747e` |
| `probes-result.json` | `657b8f82fb5a391d86bb7a8523b63e5c7f2a5123dd67006613502d3925bf0a02` |
| `audit.json` | `9b3429dcf10ed08789d265b91e88c48983509e59825e3271991f701b384671fd` |
| `policy.log` | `4483d40cad25cdfd369b6789a29205cc50ebfbc3bfaa218171a28b0f616e53b8` |
| `policy-result.json` | `20ef835880123c4294fc6e7b71f79d4df88b681e01eaefc03cbabd1e109531e9` |

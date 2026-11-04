# T-311.14 independent captured-output checks

Status: builder verification passed; independent core review required.
Implementation base: `4e00c4b3`. T-311.14 and parent T-311 stay open.
The subsequent independent review has FINDINGS; the
[same-row repair record](t311-14-stub-review-repair-2026-11-03.md) records the
stub mismatch and its repair. Measurements below describe the original batch.

## Behavior

Two existing development tasks, dev-005 and dev-018, now use the independent
function oracle v9. Their complete original hidden methods execute only in the
trusted namespace. Proxies replace `validate_raw` and `capture` before loading
the hidden module. Only synthetic text and the current finite subprocess stub
(stdout, return code and closed command list) reach the candidate. The candidate
runs its own validation and capture functions with a local subprocess stub.
No fixture command executes. No mock, callback, hidden source or supplied
exception object crosses the boundary. Both namespaces retain the same lexical
workspace and home paths without rewriting fixture text.

Return observations carry bounded JSON with a fixed top-level tuple tag. The
trusted side reconstructs only that built-in tuple and lets the original
assertions judge the returned data. Wrong supported values remain assertion
inputs. Unsupported objects and oversized observations fail as adapter policy;
malformed envelopes fail as protocol. Reserved map keys remain data. Reports
explicitly exclude real command execution, subprocess invocation arguments,
other module globals and unselected statement clauses from their graded scope.

This batch also addresses low L1 from the exact-commit
[T-311.13 independent review](../reviews/85d96fd27bee-local-coding-t311-13-observation-envelope.txt).
Memory exhaustion during response encoding becomes the fixed adapter-policy
refusal instead of a worker crash. The refusal still counts as a failed attempt;
no allocation error message reaches the assertions. This classifies an encoding
failure and does not add an aggregate memory quota or guarantee worker survival
if the candidate itself exhausts resources before encoding.

The [guide](../guides/coding-evaluation-oracle.md) describes the boundary. Frozen
source/test pins, whole hidden modules, method selections, statements and
thresholds stay unchanged. Legacy evaluator code and tests remain unchanged.
No generated-candidate admission, held-out evaluation, model run or harness tuning.

## Verification

Initial reserved verification passed: 79 unit/integration tests in 40.665 seconds;
all twelve independent base/reference pairs pass their exact historical profiles.
The two new reference controls add two complete methods and eight calls: fifteen
methods and 157 reference calls total. Each invocation remains counted.
Final checks used the shared build reservation; times exclude reservation waits.
No source changed after the final runtime checks.

| Check | Result | Seconds |
| --- | --- | --- |
| fast-gate | PASS; 79 oracle unit/integration tests | 42.155531 |
| legacy-unit | PASS; 58 tests | 19.445316 |
| independent-verify | PASS; twelve pairs, 15 methods, 157 reference calls | 6.037808 |
| legacy-verify | PASS; 30 pairs, 89 reference tests, no skips | 43.790686 |
| probes | PASS; 23 probes, 17 legacy contrasts | 8.977936 |
| policy | PASS; 712 Markdown files and policy invariants | 135.171529 |

The independent controls retain 24 invocations: twelve failing bases and twelve
passing references. Dev-005's base has three original assertion failures across
three calls; dev-018's base has one failure on its first call. Their references
pass with three and five calls respectively. Earlier exact control profiles
remain unchanged. These are development controls, not model-attempt scores.

| Diagnostic group | Result |
| --- | --- |
| References and list-valued capture results | Both graders pass on identical source bytes |
| Missing validation, lost redaction, wrong return codes and exceptions | Both graders fail; independent causes distinguish assertions and candidate exceptions |
| Tuple-valued validation results | Both graders pass dev-005 and fail dev-018, preserving their different original assertions |
| Reserved map keys | Data reaches the original assertions and fails there; no tag collision |
| Foreign objects and oversized observations | Counted adapter-policy refusal |
| Extra fields in a raw reply | Counted protocol failure |
| Original predicate truthiness carried in a 150 MiB string | Legacy passes; independent adapter-policy refusal, candidate exit 0 |

The initial private diagnostic script incorrectly used input truthiness for the
large-string comparison, so its legacy expectation failed after the preceding
22 probes completed. The corrected diagnostic wraps the original predicate and
preserves its truthiness. The failed log, reports and reconstructed initial
helper are retained privately. No production source, original assertion, frozen
pin or threshold changed for that diagnostic correction. All final probes pass.
Diagnostic admission is patched only inside the private test process; production
admission still refuses any generated source before output creation or launch.

All 124 final report/log bindings were audited against their actual bytes and
oracle-owned counts. Every frozen source and complete hidden-test pin verifies,
including the Rust control's separate hidden revision and corpus. Six legacy
evaluator source/test files are byte-identical to the base. Collection v9 remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
The retained prior independent review is byte-identical to the reviewer original.

## Source and artifact bindings

Artifact labels are relative to the private batch directory in the review
request. Raw task data, candidate projections and diagnostic logs remain private.

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `a63d51e1876e189c6d47a2aa3daf9400c33be0e77b64ae43733c9d02b37034ec` |
| `scripts/coding_eval_oracle_worker.py` | `95185d96825fe4b4d4f4c15411f28edf328849e9b5f09c57bd08252aaec768c7` |
| `tests/test_coding_eval_oracle.py` | `784a3ef13e7b3b3be48c57b0c07bcc866a5f5fd36380460ce8f2c756f99a136c` |
| `tests/test_coding_eval_oracle_worker.py` | `19ecbbe6dc6d259e462fa575961c3efa7eb6b3654042ea13a440808410c9987c` |

| Artifact | SHA-256 |
| --- | --- |
| `initial-tests.log` | `1f228abe38a64453c9119d534f78182358ddb2b0dac7d9bfda789b72b7f5d1ed` |
| `fast-gate.log` | `8c3939a38863ceaacf1d92a3f135382488c01869a3082956b092b8021c68b2dc` |
| `legacy-unit.log` | `52e313f8d25fe55c78007dd583cc9c855f473911b4b47bcf4a285d127f7a12d6` |
| `independent/summary.json` | `719a99a65eaffaa03c4c9013574ec8bddf8bd663720a7bd39df6ed24aa048ce1` |
| `legacy/summary.json` | `275be5006c6a6949b9f68ca2f76a6ec7be8345a3e98eefd8c71c75bbdd0870ca` |
| `probes/summary.json` | `ba90b35af3ccad3e72c84d83ef1815675abb6b5043dc0dae5c5c17218828636f` |
| `probes.py` | `50ee2aab6045a888310f890c9379c4c75f521a073793ee432197756ceaea1818` |
| `policy.log` | `3ee1994e9d822324b6c6150b99ffa87125b05b54680f5b50fd79e695f00eeb71` |
| `verification.json` | `caac23253fd761f00de4ca3abc4838fdc8811055ee73a4f2608ed62e4771ed40` |
| `source-digests.json` | `eea30c156066ce2fbad29ca5cfbd990bbd88d5fa9e2db3c7be68533d2c331fcd` |
| `verify.py` | `dee2a5367e87b2b3018ff2ed2c1e10db31aee1f599dd448b64bb5cf15fb0999d` |
| `audit.json` | `6dfe3bba0892a87abc4eb8ff48bc01572cf1071cf2fde818a79e6db7148612cb` |
| `audit.py` | `57289cc878838c698b4a250110530f4c9aab3d4db98da2ed3c245e774b56daa7` |
| `audit.log` | `c33a77069e034099827b924302066597ed710def15faf68fe85eab44b68e500f` |
| `frozen-audit.json` | `9e6a90756ed19154c35f5d7b8490dfd6dcb1638770de70ee7c50ee876aa440d1` |
| `initial-probes.py` | `a61a831bc0eb8b65edff9e9f37bae3bc8f38200497fd5d7709cb1b6c9ede8219` |
| `initial-probes.log` | `07fc8685c747e80e1f7194098624f84f67cabb6a45c5e3705929883719995ef5` |
| `probes.log` | `b61cf0759ed46956d3c3cbf49c3dcf0c0024ab81b6402ed5fa0030ae454f3772` |

## Limits and next work

T-311.14 stays open for independent exact-commit core review. Parent T-311 stays
open. Independent grading covers twelve development tasks; eighteen more tasks,
aggregate worker memory/process/CPU quotas, protected held-out inputs,
covered-clause admission and a real current-harness baseline remain. Per-process
limits and control-only admission remain unchanged; path result types remain
explicitly ungraded. No model score or coding-quality improvement is claimed.

This batch does not run a model, use the GPU, download anything, tune the harness
or prompts, evaluate held-out data, regenerate supply-chain outputs or historical
evidence, run the long acceptance gate, release or declare owner readiness.
T-301.1 remains deferred. Owned workers exited and the reservation was released;
four earlier evaluation scratch trees remain untouched.

# T-311.11: Independent observation fidelity

Status: builder implementation verified; independent core review required.
T-311.11 and parent T-311 remain open. The implementation base is `d446a0c6`.

This batch resolves the two low findings in the retained
[independent T-311.10 review](../reviews/adf3364a9e8a-local-coding-t311-10-argument-oracle.txt).
The [running verifier](../guides/coding-evaluation-oracle.md) keeps ten frozen
development controls and separate candidate/assertion processes. No task, hidden
method, source pin, selection or threshold changes. Generated scoring is refused.

List and tuple argument results now retain their container tag. The trusted
process reconstructs only those fixed built-in containers, and the original
equality assertions reject tuple helpers. Candidate case-root strings now cross
one additional bounded observation before the original uniqueness assertion.
Duplicate roots fail that assertion. Nothing interprets the strings as paths.
Dev-022 now makes two base calls and four reference calls; the additional call
is an observation, not a test or a relaxed threshold.

Statement-legal observations outside the bounded adapter, including callable
namespace defaults, nonstandard parser exit codes and integer verification
results, produce a fixed adapter-policy tag. The trusted record and parent report
carry a separate sticky flag and `adapter_policy` cause. Malformed tags, payloads
and sequence numbers still produce `protocol`. Both count as failed controls;
forging either tag cannot pass or remove an attempt from the denominator.
No candidate exception payload executes or becomes a trusted exception message.

Twenty tasks still lack independent adapters. Aggregate worker quotas, protected
held-out inputs, covered-clause admission and the real current-harness baseline
remain prerequisites. No model, GPU, held-out input, harness/prompt tuning,
supply-chain cascade, long acceptance gate, release or owner readiness is claimed.

## Verification

All heavy checks used the shared build reservation; no GPU was acquired. The
initial 54-test oracle suite passed in 21.447 seconds after reservation wait.
The final fast gate repeated those tests on the same source bytes. The timings
below exclude the outer reservation wait.

| Check | Result | Seconds |
| --- | --- | --- |
| Scoped fast gate, seven changed paths | PASS; 54 oracle tests | 22.854041 |
| Existing evaluator unit suites | PASS; 58 tests | 19.602716 |
| Independent CLI verifier | PASS; ten pairs, 13 methods, 149 reference calls | 4.945726 |
| Unchanged CLI verifier | PASS; 30 pairs, 89 reference tests, no skips | 43.816156 |
| Persistent synthetic diagnostics | PASS; eleven probes and seven legacy contrasts | 4.288998 |
| Documentation policy validator | PASS; 708 Markdown files and all invariants | 141.656659 |

The independent summary retains 20 control invocations: ten passing references
and ten failed bases. These are controls, not model attempts. No failure is
removed from the denominator. The persistent diagnostic reports retain the
following observations, produced by original synthetic sources in real confined
processes. Their private host process mocks only the historical-source admission
for these probes; actual admission and the public commands stay closed.

| Probe | Observed result |
| --- | --- |
| Tuple helpers, dev-006/007/027 | Each fails its original equality assertion; two calls, one failure; legacy grader also fails |
| Duplicate case roots | Original uniqueness assertion fails; one observation call, one failure; legacy also fails |
| Callable case-root value | Adapter-policy failure before assertions; one call, zero tests, one counted failed attempt |
| Integer predicate results | Adapter-policy failure; legacy passes; strict Boolean policy remains disclosed |
| Callable namespace default | Adapter-policy failure; legacy passes |
| Parser exit code 1 | Adapter-policy failure on call 37; legacy passes |
| Policy tag with extra message | Protocol failure; untrusted payload absent from trusted diagnostics |
| Invalid container tag | Protocol failure; no object reconstruction |
| Forged exact policy tag | Adapter-policy failure; one counted failed attempt, never a pass |

Tests also exercise malformed case-root frames, zero-assertion record validation,
sticky failure, exact tags, unsupported container values, startup refusals,
namespace isolation, output/deadline limits and process cleanup. Zero assertions
are accepted as a failed prerequisite observation only for the case-root task,
with exactly one call and a protocol/policy failure. Other missing assertion
counts remain invalid. The original hidden methods and thresholds are unchanged.

All thirty base/reference source pins, hidden hashes and the Rust corpus pass
input audit. The dedicated frozen Rust hidden revision is used for its test and
corpus, independently of the source reference. The six legacy evaluator source/
test files are byte-identical to the implementation base. Collection v9 remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
All 98 retained parent reports and their raw-log bindings were audited, including
20 independent controls, 60 legacy controls and 18 diagnostic/contrast reports.
Raw source, reports and logs stay private; public evidence contains measurements
and digests only. Existing path-return-type omissions remain explicitly ungraded.

## Verified source and private artifact digests

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `ccd95f5c47c8a6c6348af401be664da510ec175c5641ed2517de890df4a1aef7` |
| `scripts/coding_eval_oracle_worker.py` | `8f691f21c0fc2c8253c174d0ca35be2b449ce51c85a78565b7468f5377ea95a4` |
| `tests/test_coding_eval_oracle.py` | `cb493276322613dd4de068a65b9c87ded7b3e65502459f70ceb2b35b4121c35e` |
| `tests/test_coding_eval_oracle_worker.py` | `d9dc8c19d820d935b858cb9d6cdccd5fb69b5cf22d940021da7a10131de2a02c` |

Artifact labels below are relative to the private batch record, not public links.

| Artifact | SHA-256 |
| --- | --- |
| `initial-units.log` | `c89631f83c8b7c21cb70e01bb49e69fc42d510beb2f094b79874da55feadad23` |
| `gate.log` | `20bcc1ffc563087e078badd388b7eb2ad4b21d408ba11952cfcde1dad0d7b14c` |
| `legacy-units.log` | `ee301467326737fdee285d5fe5d3a34793afac07d3e58281a9b93ce375cdecc4` |
| `controls/summary.json` | `563d5ebd8afa2e06ebf261bea448ac39a6d861c4581e3e3d8b924b56ae87c607` |
| `full-controls/summary.json` | `be6d15eeabfd107628ce68b6ba1ebdd4c1ce762a2ba08174980c81e81385fa48` |
| `probes/summary.json` | `a2f5a20735fe141bb7a48d9adf318a1bbef71ca6ed137ffd50b0f777dcf08523` |
| `probes.py` | `1f4a5bdd7ee90940b1c617dc61f5f317f3086d19383d44da0ba2d016ca2e5a15` |
| `policy.log` | `8d3cd98bf856def3a9bc3a5d67a4c2b4957e50e16bd2e17fcd0c2244f7e1b216` |
| `checks.json` | `20e5233832c41dd6e98b1add98889c7dd0c8a1c198c795afb2b4f9c0f8cbed85` |
| `audit.json` | `e15f11b7515f275893d80b4b340638756949a7794b712e2a67b8890d89d0893a` |

After verification only queue/evidence measurement text changed. Final Markdown,
whitespace, source/input bindings and added-text hygiene are checked separately.
No owned worker or new evaluator temporary tree remains; historical leftovers
are untouched. The builder requests independent core review of the exact commit;
this evidence cannot close T-311.11 or parent T-311.

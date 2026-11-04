# T-311.12: Independent failure-cause fidelity

Status: builder implementation verified; independent core review required.
T-311.12 and parent T-311 remain open. Implementation base: `9d662b5f`.

The [independent first review](../reviews/9563c0a80d55-local-coding-t311-12-failure-causes.txt)
returned FINDINGS: integer truthiness below was verified only for verification
report predicates; Boolean/map tasks still rejected it. The
[review repair](t311-12-truthiness-review-repair-2026-11-03.md) corrects that
coverage gap. Measurements below remain the original v6 observations.

This batch resolves low L1 from the retained
[independent T-311.11 review](../reviews/568d27eb1f45-local-coding-t311-11-observation-fidelity.txt).
The [running verifier](../guides/coding-evaluation-oracle.md) still admits only
historical controls for ten frozen development tasks. No hidden byte, source pin,
method selection, task statement or threshold changes. Generated scoring stays
refused. No model, GPU, held-out input, harness/prompt tuning, download,
supply-chain regeneration, long acceptance gate, release or readiness claim.

## Behavior

Only argument and verification tasks declare adapter policy. An exact policy tag
on another result type now fails as protocol; malformed or payload-bearing tags
always fail. Bounded built-in JSON helper values, including nulls, strings,
numbers, mappings and nested list entries, reach the original equality assertions.
Exact built-in list/tuple helper containers still retain their type. Other
objects remain policy refusals, with the bounds named in report coverage.

Predicate `passed` and `checks` observations likewise carry finite built-in JSON
data within 16 container levels and the existing frame limit. Original frozen
assertions decide their truthiness. Reference results converted to integers now
pass those same assertions. This removes the older strict Boolean adapter
policy without altering any hidden assertion or threshold. Exact Boolean result
types are explicitly ungraded; no full-interface acceptance is claimed.

Unexpected candidate-call exceptions produce one literal candidate-exception tag.
The trusted worker raises a fixed local error, retains a sticky
`candidate_exception_failed` flag and reports cause `candidate_exception`.
Neither class, message, traceback nor exit status crosses the pipe. Missing
required report fields and an absent function after readiness use this cause;
startup/import failures remain infrastructure, and malformed frames remain
protocol. Expected ambiguity, missing-case and parser/helper refusals retain
their existing fixed tags. A forged tag can only fail its own invocation.
These are ordinary adapter-observed causes, not a forensic guarantee against
an adversarial candidate that deliberately corrupts its wire or exits.

The dev-029 base now reports the absent function with one call, one original
test error, candidate exit 0 and oracle exit 1. Its counts and assertions are
unchanged; a broken-pipe exit no longer substitutes for the missing call result.
A failed case-root prerequisite observation can retain zero tests only with
exactly one call and a sticky protocol, policy or candidate-exception flag.
All such invocations remain failed attempts. Other historical profiles stay
unchanged: ten independent pairs, 13 original methods and 149 reference calls.

## Verification

All heavy checks used the shared build reservation. Timings below exclude the
outer reservation wait. The initial 64-test oracle suite passed in 26.630 seconds;
the final fast gate repeated all 64 tests on the final source bytes.

| Check | Result | Seconds |
| --- | --- | --- |
| Scoped fast gate, seven changed paths | PASS; 64 oracle tests | 28.064918 |
| Existing evaluator unit suites | PASS; 58 tests | 19.551061 |
| Independent CLI verifier | PASS; ten pairs, 13 methods, 149 reference calls | 5.097915 |
| Unchanged CLI verifier | PASS; 30 pairs, 89 reference tests, no skips | 43.948157 |
| Persistent synthetic diagnostics | PASS; 37 probes and 30 legacy contrasts | 14.52607 |
| Documentation policy validator | PASS; 709 Markdown files and all invariants | 141.042981 |

The independent summary counts 20 invocations: ten failing bases and ten passing
references. They are controls, not model attempts. The following persistent
builder-authored probes run in real separate confined processes, patching only
the historical-source admission guard in a private diagnostic host process.
Actual `grade` and CLI admission remain closed to modified snapshots.

| Probe group | Observed result |
| --- | --- |
| Six wrong helper values on each of three argument tasks | All 18 fail original equality assertions; legacy likewise fails; no protocol or policy substitution |
| Wrong null/string/integer `passed` observations | All three fail original truthiness assertions; legacy likewise fails |
| Unexpected helper/parser/predicate/path/Boolean/map exceptions | All six use fixed candidate-exception cause; payload absent from wire and trusted diagnostics; legacy fails |
| Missing report checks | Fixed candidate-exception cause; one failed invocation |
| Exception while observing case roots | One call, zero assertions, one failed invocation |
| Reference predicates converted to integer truthiness | Both independent and legacy controls pass; exact Boolean type explicitly ungraded |
| Policy tag on Boolean task | Protocol failure, never a declared policy or passing result |
| Exact forged exception tag | Candidate-exception failure, never a pass |
| Exception tag with a payload field | Protocol failure; payload never reconstructed |
| Allowed argument policy tag | Policy failure, retained in denominator |
| Policy tag with a payload field | Protocol failure, retained in denominator |
| Candidate exits early during a call | Protocol failure, retained in denominator; legacy fails |
| Unexpected `SystemExit` during Boolean call | Candidate-exception failure; legacy fails |

Regression tests also check exact policy coverage/type agreement, malformed
exception tags across all result types, sticky failures without further reads,
scalar and mapping helper values, finite/nesting bounds, zero-assertion record
validation, startup failures, forged counts, hidden/result isolation, deadline
and output limits, interruption and cleanup. None rewrites a frozen hidden test.
The legacy evaluator source and test modules are byte-identical to the base.
All thirty base/reference source pins, hidden hashes and the Rust corpus pass
input audit. Collection v9 stays
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.

All 147 retained report/log bindings were audited: 20 independent controls,
60 legacy controls, 37 independent diagnostic probes and 30 legacy contrasts.
Independent count records equal their parent reports. Their source and adapter
bindings match the final source bytes. Raw source, logs and reports stay private.

## Verified source and private artifact digests

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `fc9ac3b4a958c39a612c083155833819ed9834ed30772c90ebb84bf3f99d5302` |
| `scripts/coding_eval_oracle_worker.py` | `8d68d60ca3741751899f07f88014fae52c51f3d4094829aa1b3154a2d3bed471` |
| `tests/test_coding_eval_oracle.py` | `557e517f6a566c66ec0a920821563f2bcc96b83eb14cc3b16a8e4acbc1dac5ae` |
| `tests/test_coding_eval_oracle_worker.py` | `7bbb7e4f8760372a9371dd82d60a4699b50489a24a993934dfbe14ca388328b8` |

Artifact labels below are relative to the private batch record, not public links.

| Artifact | SHA-256 |
| --- | --- |
| `initial-units.log` | `dabb59b122b0a4a991d675ae2c85c6d601140fcfb8c5cadd265a3c6c56b4c0f8` |
| `gate.log` | `23bb85102657cfea75de830775bb577879df31d09f9c0d65c12e022868eb75b7` |
| `legacy-units.log` | `becc7e0f2c65e205a08eadb1a1ee74f28d7200a57704de94379e4f7a487dace3` |
| `controls/summary.json` | `70557734e554aea4938b08f01ba7e76d3b6c3e5a2c123df2d0c92344ac76edbd` |
| `full-controls/summary.json` | `ea877f68a7319eea732df1bb5bc9979fc4cf5194aa270760c7bc17a1b7f62f79` |
| `probes/summary.json` | `97c3f1a4fdb981fd3e592dda54dc641a80b5489d0f1872ae6f3dd3f28692acb5` |
| `probes.py` | `3a6f17719675d66f8f86513ec3ae0b72029ad5755d11d5add638f71fa0746dc4` |
| `policy.log` | `d7748c5e8c7c0550a3bfe0e7842af1c2bc7e92fbe48405fa0520430b1e366d88` |
| `checks.json` | `da738dcb6c25232d9389c175900fe7dd8a3226cdd423782800c88cbc6e999043` |
| `audit.json` | `a2ececfc7214bd439c7817d99aa113b0851e7b5d2344c2c8eba45fa4544fbad1` |

After verification only queue/evidence measurements changed. Final Markdown,
whitespace and added-text hygiene checks follow separately. Owned workers and
new temporary trees are checked before handoff; historical leftovers stay
untouched. Independent core review of the exact commit is requested; builder
tests and this evidence do not close T-311.12 or parent T-311.

Twenty tasks still lack independent adapters. Aggregate quotas, protected
held-out inputs, covered-clause admission and the actual current-harness baseline
remain prerequisites before generated scoring. Path return-type omissions stay
explicitly ungraded. Neither this evidence nor builder tests close a row.

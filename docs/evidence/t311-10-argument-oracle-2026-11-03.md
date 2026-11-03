# T-311.10: Independent parser and argument observations

Status: builder implementation; independent core review pending. T-311.10 and
parent T-311 remain open. The retained [independent review](../reviews/9665924d2880-local-coding-t311-9-verification-oracle.txt)
has **Verdict: PASS** for exact commit
`9665924d2880f5a07ed60040ae53ea7dca988aab` and closes only T-311.9.
The implementation base is `b899d805`.

## Running behavior

The [independent verifier](../guides/coding-evaluation-oracle.md) extends the
existing six controls to ten frozen development tasks. Three additions exercise
parser/argument construction for route grants, memory and extensions. A fourth
checks that a recorded command failure belongs to its exact command turn and
has the expected payload digest. These are synthetic observations, not product
authority, package execution, catalog operations or a model score.

Both processes retain their separate confined, networkless namespaces. Only
the trusted process imports hidden methods and publishes assertion counts.
The candidate imports its own parser and helper. The trusted parser proxy sends
argument strings, receives bounded scalar namespace data with lexical Path tags,
and hands that namespace to the helper proxy. Only argument strings and fixed
refusal tags return. No supplied callback, parser action, exception class,
message, exit status or executable object is reconstructed on the trusted side.

| Task | Base failures / errors | Base calls | Reference calls |
| --- | --- | --- | --- |
| dev-006 | 0 / 1 | 1 | 19 |
| dev-007 | 0 / 1 | 1 | 52 |
| dev-022 | 0 / 1 | 1 | 3 |
| dev-027 | 0 / 1 | 1 | 37 |

The three parser-task bases lack their requested helper and reject the first
parser call. They remain alive through that call and return the fixed parser
exit tag; launch failure cannot validate those controls. The dev-022 base lacks
its requested case and returns the existing missing-case tag. All ten controls
must match exact startup, function-presence, failure/error and call profiles;
references must pass with exact call counts. Together they cover 13 selected
original methods and 148 reference calls. Hidden method bytes and selections
are unchanged.

## Limits and review dispositions

T-311.9 low L1 is now explicit adapter policy:
`strict-boolean-result-type`. Original truthiness assertions can accept integers;
the verification wire requires actual Booleans. A real integer-return probe must
fail the protocol, and the report names this policy. Low L2 is repaired by fixing
the historical evidence table's delimiter only; historical measurements remain
unchanged. The independent review itself is retained byte for byte.

Parser/argument reports explicitly leave exception messages, parser diagnostics,
original parser object type and argument return container type ungraded. A tuple
return becomes a JSON list even though the original direct list assertion would
reject it; a real tuple probe demonstrates this. Dev-022's preamble asserts
uniqueness of the trusted base's case roots, not the candidate's roots; a real
duplicate-root probe records the `candidate-case-root-uniqueness` omission.
No claim of full statement or interface coverage follows from these controls.

Generated scoring remains refused before output creation and worker launch.
Twenty tasks still lack independent adapters; aggregate memory/process/CPU quotas,
protected held-out inputs, covered-clause admission and a current-harness real-model
baseline remain prerequisites. Per-process limits are unchanged and are not
aggregate quotas. Every returned failed control remains counted. Future model
attempt accounting must retain infrastructure failures without silent retries.
No model, GPU, held-out input, prompt/harness tuning, long documentation gate,
evidence cascade, release or owner-readiness run occurred in this batch.

## Verification

All heavy checks used the shared build slot; no GPU was acquired. The initial
48-test suite passed in 20.002 seconds after reservation wait. The final scoped
fast gate repeated those 48 tests successfully. Command wall times below exclude
the outer reservation wait.

| Check | Result | Seconds |
| --- | --- | --- |
| Scoped fast gate, eight changed paths | PASS; 48 oracle tests | 21.381429 |
| Existing evaluator unit suites | PASS; 58 tests | 19.665243 |
| Independent CLI verifier | PASS; ten pairs, 13 reference methods, 148 calls | 5.012233 |
| Full unchanged CLI verifier | PASS; 30 pairs, 89 reference tests | 43.864367 |
| Documentation policy validator | PASS; 707 Markdown files and all invariants | 132.740343 |

Each of the twenty independent invocations retains a parent report, oracle
counts and candidate/oracle/wire logs. Their report bindings and raw-log hashes
were checked. The unchanged full verifier retains all sixty base/reference
reports. All thirty hidden hashes, source pins and the Rust corpus are verified;
the six legacy evaluator source/test files are unchanged. Collection v9 remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
Private raw observations, reports, logs and input audits remain with the lane;
they contain fixture paths and diagnostics and are not published.

The first independent control run reproduced all frozen behaviors, but its
summary failed because the new dev-006 expected reference count was set to 20.
Inspection and the raw wire established 19: four successful calls and five
helper refusals each require a parser/helper pair, then one parser conflict.
The expectation is corrected to 19; no test, task, hidden byte or threshold
changed. That failed run remains in the private record.

## Verified source and private artifact digests

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `f87ad12e29a7d52ccb509c8ec6f25e71026766f47e63cfc77c1464176f7f506a` |
| `scripts/coding_eval_oracle_worker.py` | `738fc253457a2e61863af15b5ff3f7311134cb9be15095de1d81e5ac37eb2d0d` |
| `tests/test_coding_eval_oracle.py` | `198e9f5a93c50992d29465c36cc747db8fa0a236a0526219a5cebeccff828503` |
| `tests/test_coding_eval_oracle_worker.py` | `cc922c2cf1181f24ecd0eebb5d1bb1d5e75c8a3d85afc226b3162d5b4a1ee52e` |

Labels below are relative to the private batch record, not public file links.

| Artifact | SHA-256 |
| --- | --- |
| `initial-controls/summary.json` | `b23dc79863f5f95da4fd344eabb74d989ccfea870976d11215f1de4d905ef3a1` |
| `initial-units.log` | `1905e0559d6716762706723563a68f755c4086d8dca0176985ca6947effc8bda` |
| `gate.log` | `378d5e0920c7c6ddcc7ebd5ac3254ab80deeeb058e6bed88dfcbe1630e36bddb` |
| `legacy-units.log` | `475cde9d21dcf5f8e0af5d5cf4a69a300f15b05780f69fde1a522c69c7ec21e1` |
| `controls/summary.json` | `246df34f390e7ee845286874e7af60425c559cfc6f6d85479c5c0aefe2668888` |
| `full-controls/summary.json` | `f58be71dd7ca4342a6991a3fe0d02a097775dc9c8a728d1d60abcb1b59ae517b` |
| `policy.log` | `d291bdd7f29f6c7971dfdda7e2e6b22aa789caa6ebd3f1929af2603a0b74d534` |
| `checks.json` | `180e5185cc9a9d03f08e2da4b47294131b13a5227f780f4716e69d7d82905ebf` |
| `audit.json` | `c2dce91bdd04bef7ea6b874a60797e59c79a4c96df0b37ab6589057910f639f8` |

After verification only the queue/evidence measurements changed. Final Markdown,
whitespace, input/hash bindings and added-text hygiene are checked separately.
No owned worker or temporary evaluator tree was left by the batch; existing
historical trees are untouched. The builder requests a separate core review of
the exact implementation commit and cannot close this row.

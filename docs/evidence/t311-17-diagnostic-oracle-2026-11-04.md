# T-311.17 independent failed-command diagnostics

Status: builder verification passed; independent exact-commit core review
required. Implementation base: `67358640`. Row and parent T-311 stay open.

## Scope

Advance dependency-ready work after the third T-311.15 non-PASS review, retained
unchanged in the preceding commit. T-311.16 records that original row's open
medium finding; this batch does not attempt its repair or restart its reviews.
Generated scoring stays disabled. The writable mode fixture and its cleanup
remain unchanged, including the disclosed quadratic host traversal defect.

Oracle v14 adds the existing frozen development task dev-009. Its complete
original method runs in the trusted process. The candidate receives command
strings and a lexical working directory, reconstructs the original tuple and
Path inputs, and calls its own subprocess_runner. The reference runs the real
synthetic failing child inside its confined namespace. Captured stderr and the
closed RuntimeError category with a string message are bounded data; the trusted
proxy replays text and raises only its own built-in RuntimeError. Other errors
use the unchanged fixed failure tag. No candidate class, traceback or executable
object crosses the pipe. Unknown/extra fields refuse before replay. Existing
frame/transcript limits, startup, budgets, failed-attempt counts, networkless
namespaces and historical-controls-only admission remain in force.

The original method checks diagnostic and exception-message substrings. It does
not check successful values, subprocess arguments, exact exception type, output
tail or newline. Coverage labels and passing subclass/long-text contrasts make
those omissions explicit. The [guide](../guides/coding-evaluation-oracle.md)
describes the fixed error observation separately from unexpected exceptions.

## Verification

All checks below ran through the shared build reservation. Times exclude
reservation waits and include command overhead. Source/test bytes stayed fixed
after the passing fast gate. Final Markdown lint covers all five changed
documents; its initial run found zero issues after the measured evidence update.

| Check | Result | Seconds |
| --- | --- | --- |
| Scoped fast gate | PASS; 109 oracle/protocol unit and integration tests | 68.033739 |
| Legacy unit suites | PASS; 58 tests | 20.111561 |
| Independent verifier | PASS; 15 valid pairs, 18 original methods, 160 reference calls | 8.185157 |
| Unchanged full legacy verifier | PASS; 30 valid pairs, 89 reference tests, no skips | 45.063295 |
| Persistent diagnostics, repaired wrapper | PASS; nine probes and nine legacy contrasts | 3.909158 |
| Policy validator | PASS; 717 Markdown files and all policy invariants | 148.677762 |

The report audit passed: 108 reports, 255 log bindings, all 30 frozen task pins,
six unchanged legacy source/test files, the preserved ledger suffix and unchanged
retained review bytes. Fifteen bases fail their exact expected profiles and all
fifteen references pass; control attempts are 30 total, 15 passed and 15 failed.
The frozen collection digest remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
No cleanup recovery record was produced by these retained runs. The deferred
cleanup and mode-fixture functions have unchanged syntax trees against the base.

Initial five-method targeted run failed only
because the new confinement probe correctly refused the legacy output mount,
but its contrast expected a pass. The test now names the separate-mount case and
expects that legacy failure, plus a distinct ordinary real-child pass on both
adapters. No original hidden test, pin, selection or threshold changed. The
unchanged five-method selection passed on repeat (4.015 seconds). Both raw logs
are retained privately.

The first persistent probe wrapper failed before worker launch because its
output parent directory was missing. That unsuccessful invocation and raw
traceback are retained separately (one failed invocation, zero worker launches;
elapsed time unavailable). Only private orchestration was repaired. The measured
rerun includes all nine probes and nine legacy contrasts, without changing a
source, original test or threshold.

No model, GPU, held-out task, prompt tuning, model download, supply-chain cascade,
long acceptance gate, release or owner-readiness action occurred. All model
attempts and GPU minutes are zero; model score and tokens remain null.

## Artifact identity

Raw reports and logs are retained privately. These neutral artifact labels
identify the measured bytes; no host path or task transcript is published.

| File | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `a70d7b849a2e543b9f1fc4a77e9362f82ffbc3bf4b283518414575eaac6d05b7` |
| `scripts/coding_eval_oracle_worker.py` | `595bf72531a5b50887d4d660f4d1a806024e41b0b6af93bd98935f2d03b1848f` |
| `tests/test_coding_eval_oracle.py` | `55c19e3c8151ba4002edf745f33bf22706570bfce897a524d73ee89d620d2461` |
| `tests/test_coding_eval_oracle_worker.py` | `9c776d6fe5bda082e0dbf12b95c84d8e6cdc870826c9fe50ecddae1c00b38114` |
| `fast.log` | `9aa74dd743bdd8c90a8b60d554817e9f0cb32295c3bd8bbd107b99e407c035df` |
| `fast-result.json` | `9ae4e4bf052126a4f95a249b5ecdeae87da4a96e3f98a286da6ef5bb87c78644` |
| `legacy-unit.log` | `8261eb5e5c60c9f5470c909160552aae56c79bb47b83b0f1a44f352d531b5878` |
| `legacy-unit-result.json` | `9960a53bb2791684b546b5f633916ae6f0c9df217f88af247c31644f244748ea` |
| `independent/summary.json` | `31137171fe088180acddf5117d2f4b67ed5a6bc683bfa8f6e7a90754ddea7664` |
| `independent-result.json` | `363a6d973887a02ee579032124de8ec403085c01e1385db5387d6a30e0900827` |
| `legacy/summary.json` | `4aea6b11458e2744f09538ed6009b44b0f23c1b9c985d5a14c2d01cdcb9d1259` |
| `legacy-result.json` | `05b844be67a8325aafcdea56b80d7ab6c8e6b7b9989be0466b36a24955c5f54d` |
| `probes.json` | `a7f28557a0fe9cc713913abc350504629711030b35b37b270f736220b814c15a` |
| `probes-result.json` | `fd398a6ad9df483eb641c8b7b8cbce675b9eff6c74ce1bfbb3df42cc27004d70` |
| `audit.json` | `63292d3f5a1638aae2da30c58a297158d4e13a2bb366e6860ff9ed23db7fbd8c` |
| `source-digests.json` | `124632a8d8405235b6c3913fce31d79fd0e78ed8c0c5c6f2004f280e16f80de8` |
| `targeted.log` | `5562d2471cd349c93fec7bcc467540a242961b1f924cfb7498111f65cf363302` |
| `targeted-repeat.log` | `8e573490112ea3e23319935c7cdfa945ac894a29920e2a8356c94b35c4da93d7` |
| `preflight.json` | `4996b1a3da51a386baa82bb3bd608f307e1799a12de9d834d69bec58e8929a57` |
| `probes-initial.log` | `e7b8428dd280fee90e794981baad025cde9471ecc5a1580038c185883a7d5296` |
| `probes-initial-result.json` | `63cc394e57599eb7f6ddebafb313f1da1c4f159b714f3d7bb3584f829d645995` |
| `probes.log` | `018682ad0a1e47d8933e58c923596b80f179f361433aefb6314ed8a61b5a356a` |
| `policy.log` | `4a53edd6d1a68d3fa87453f0a76bc40235f3855b9b8b98f954c1581a349b4c16` |
| `policy-result.json` | `fd41090b9b13457fd8c857554462fd464d3abd7cc340648f3b5fcbdc07abfbc7` |

# T-311.26: Preserve usage observations before replay

Status: builder verification passed; independent exact-commit core review required.
Implementation base: `a23089ccb5c63da57664fa1f64a3bf75b9023abd`.
T-311.26 and parent T-311 remain open. Review cycle: first submission.

## Running behavior

The independent [T-311.25 core review](../reviews/78c9d2f2f423-local-coding-t311-25-usage-replay.txt)
is retained byte-identically with **Verdict: PASS** for its exact commit. Only
that row closed. This batch addresses its low findings L1-L3 together.

Oracle v25 snapshots the mock's child and direct call histories, last-call fields,
count and called flag after replay. Before the next request and again after the
validated response, a mismatch causes sticky adapter-policy refusal without
resetting or replaying the mock. Unowned child calls refuse ownership. Typed,
bounded encoded snapshots do not alias mutable call arguments and distinguish
equal-valued Boolean/integer substitutions. Each history vector retains the
existing 128-call bound and each encoded snapshot the existing frame bound.

After a candidate call, every retained message root is checked against its
pre-call encoded snapshot. Net root or nested changes, including mutations of
an earlier root, return the fixed adapter-policy outcome before trusted replay.
Candidate mutations are refused rather than propagated. Input packing refusals
also set the sticky flag; catching their `ValueError` cannot produce a pass.
No candidate-supplied executable object or exception reaches the trusted tests.

The [guide](../guides/coding-evaluation-oracle.md) records the limits: nested
identity, equality subclasses and mutation propagation remain unsupported;
transient mutations restored before return and asynchronous changes after the
snapshot are not observed. This does not establish absence of candidate effects
inside its sandbox. Existing generated-scoring and resource blockers remain.

## Verification

Six new pre-fix methods reproduce the review findings with ten failures and seven
errors. The old proxy overwrites history or proceeds to an unavailable response
instead of refusing locally. A synthetic candidate that appends to its message
list passes oracle v24 but fails the original in-process tests. Exact pre-fix
source/test snapshots and raw failures are retained privately.

The persistent old/new/legacy contrast uses that same candidate and frozen
methods: v24 falsely passes, v25 refuses with adapter policy, and the legacy
grader fails its original cache call-count assertion. This is diagnostic input,
not a model-generated attempt. The original frozen tests are never edited.

New checks cover reset/direct/child history drift, argument mutation, direct-call
metadata changes, child history before ownership and history changed while a
response is in flight. Encoding refusals stay sticky after caught exceptions.
Message-mutation checks cover root append, nested mutation, equal-valued type
changes, unsupported values and mutation of a previously retained root. Prior
identity-cache, payload-history and malicious-wire diagnostics still pass their
expected assertions. All previous test methods are unchanged.

Every suite and namespace below ran sequentially inside shared build reservations.
Times exclude reservation waits. No GPU or model ran.

| Check | Result | Seconds |
| --- | --- | --- |
| Pre-fix regressions | Expected FAIL; 6 methods, 10 failures / 7 errors | 0.441047 |
| Initial repair | PASS; 85 tests | 0.732113 |
| Final focused checks | PASS; 85 tests | 0.69322 |
| Scoped fast gate | PASS; 173 tests and lint | 102.539413 |
| Legacy unit suites | PASS; 59 tests | 23.508939 |
| Independent frozen controls | PASS; 19 pairs, 25 methods / 192 reference calls | 9.76802 |
| Legacy frozen controls | PASS; 30 pairs, 89 reference tests | 44.584149 |
| Persistent usage diagnostics | PASS; 7 methods, 27 reports | 3.788904 |
| Retained namespace contrast | PASS; old false pass, repaired policy refusal, legacy test failure | 0.532977 |
| Integrity audit | PASS; 128 reports / 312 log bindings | 1.609267 |
| Repository policy validator | PASS; all policy invariants | 139.783415 |

The independent verifier counts all 38 attempts: nineteen failing bases and
nineteen passing references, totaling 25 original reference methods and 192 calls.
The legacy verifier retains thirty pairs and 89 reference tests without skips.
All thirty frozen task/source/hidden/fixture pins, selections, control profiles
and thresholds match the implementation base. The collection fingerprint remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
Legacy, fast-gate, sandbox, launcher, cleanup and admission functions are unchanged.
Source/test digests stay stable throughout verification; model attempts and GPU
minutes are zero, model score and tokens null. No cleanup recovery record exists.
The policy validator is not the long documentation gate.

## Open scope

Independent exact-commit core review is pending. Parent T-311 and deferred cleanup
rows T-311.15/.16 remain open. Remaining adapters/admission, aggregate and scratch
quotas, host metadata, protected held-out inputs, coverage/dev-003 policy,
policy-accounting contrast and the real current-harness baseline remain open.
No harness/prompt tuning, held-out run, model download, evidence cascade, long
acceptance gate, release or owner-ready claim follows.

## Retained bindings

Source paths are repository-relative; other paths are relative to the private
`t311-26-usage-observations` artifact directory. Reports bind their own raw logs.
Capture wrappers create exclusive logs and must not be rerun in place. Copy the
probe and contrast scripts beside fresh output directories before reproducing;
the contrast also requires the two retained `red-` source snapshots.
`write-evidence.py` generates this document and is not a review command.

| Input | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `db20f04fa603dc5c0c75befe451adb1aa72666696e804d47ea0c3d2c7599ac57` |
| `scripts/coding_eval_oracle_worker.py` | `88b66a80c768cc5cd3e90e77e7b4f0f2c0226e65b5ff0a92bb31db8ca9da0b90` |
| `tests/test_coding_eval_oracle.py` | `f798408e9d37306c2b0d464ab1a20e6e5b7532cbf2fde462c06318d62af9067f` |
| `tests/test_coding_eval_oracle_worker.py` | `c4cee23565e2b74dd8e9d79ccd4b5b696839e5d54d0b4c925e3b185e90e5f7f3` |
| `source-digests.json` | `eeab50dfdbbc7548756927ff817d520fa7e6f8600dc4d868818328ddd4f36d76` |
| `red-source-digests.json` | `ee5602c5d6f72aa1c2660b60eeae5127d4951cc2268acab0cd8145613f937c9b` |
| `capture.py` | `bccabb454b11e49f8ee0cc31596faedf44d9eed79bd79c5f34d080b6a63906fa` |
| `batch.py` | `9e85069a1ecf5308a5ffbaae08ec43c5b333a9bb1be5884328c1c02c88953f12` |
| `batch.json` | `06f0b6260e06e9865073bdd519fd7aa24991f7b1d49475590563f4833f235c45` |
| `probes.py` | `fc29eed67a86f5207e6c8ed4e7a9a33db8f51c0b8b03255db490bdc6ebfc934c` |
| `red-observation.py` | `19f109f523d86df9a6020dd69bb2dda250d56ceb94fc1b25e16f4ff9c933c159` |
| `audit.py` | `c831f63e85be9f4586091e7a932b407269332a9a60c2c1c594ee49cf6dc13919` |
| `audit.json` | `cefdb05bb30afe94b1c7417ed1fa115182cfa916c965873c9d03e8a3ff307e3e` |
| `probes.json` | `000913a8751dd6e4ed70f84a21825e21cb7fa5bc526eb40cb58050775032f171` |
| `red-proof.json` | `fc3b69bdf3e00a678d8ceaeb0921599283cefdc9a236bfccb986822637674b35` |
| `independent/summary.json` | `054fd217f1ff54ae5b57e9fa42a8ebd02d8da92f15d43b8a4d31009a6341a9ef` |
| `legacy/summary.json` | `78dbfea19f6ba20d0cb065b6748c6ba79a0aacf31f433cfb5c7c483d05a7584b` |
| `write-evidence.py` | `0ef701332a43d62cb7f815b679c44bba7556b3939c99adc5128e061cfc47b9c5` |
| `final-docs.py` | `e56d7754b163f3aa58a0516cb0b8a00c5e1225baf2aaeef24267ebf81ff903b5` |
| `final-integrity.py` | `3164bb4f2ae03114f42dfb03e0491d5f6afa09c8ec17b313dc80011458f67ff7` |
| `red-coding_eval_oracle.py` | `3c5888c39b5f797c970ed146a436574ee9deed2e666aefc76d5b3c707e144de1` |
| `red-coding_eval_oracle_worker.py` | `12165b9f2a18f5b061bc9c241167e224d3b91c85a8c1213615fd87823733369a` |
| `red-test_coding_eval_oracle.py` | `f798408e9d37306c2b0d464ab1a20e6e5b7532cbf2fde462c06318d62af9067f` |
| `red-test_coding_eval_oracle_worker.py` | `c4cee23565e2b74dd8e9d79ccd4b5b696839e5d54d0b4c925e3b185e90e5f7f3` |
| `red-result.json` | `9b68e0913af3646a3580010c80089ac9f8cdd47f062039eacee7717dfb85c6df` |
| `red.log` | `ab043585bc3574b9f271a8921261e3a3e18754668c198f3276c899bd30a23dbc` |
| `pilot-result.json` | `95235a646755d1c1b67b78ac38fa8d300cf7ea96948dcb1a4365ee8a322e28fa` |
| `pilot.log` | `5833d17434bf1aebfaf1bcafa44f91e2dd659f30a32ea20c06ff817b46d9f01d` |
| `focused-result.json` | `e57411c8e022b431ae1308e29aefbe87053801527d72cfc735366ce8afe30f33` |
| `focused.log` | `b14c92f9068b9ff7374b5e0b40c56a4cfee9279ecbcce788d490eadcb7b88bfa` |
| `fast-result.json` | `0fbfaf9133c98e0743294bc327c3cb2749389946b77cd7862dafedf90aef8a85` |
| `fast.log` | `e803f04bd3cff2a5167984c37d0cf3260de0ce06fda65d23e922eeb51221b271` |
| `legacy-unit-result.json` | `010cdbb0a874bdc7da333561c81da3190374fc3ac309dbe8e5790b8cf366e899` |
| `legacy-unit.log` | `32b089b726f9b3b4140ed52e1568bd10bdc354e45832bac2c23fd3e35c094332` |
| `independent-result.json` | `906ad8e56e7ef9efee9941b9d3f741ed2150b18291717c39fe5bbffb3813faaa` |
| `independent.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `legacy-result.json` | `8e3f17f0d873706dc4cf4529bcbabcfe1a59870b9d916027de730a3b76334f6f` |
| `legacy.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `probes-result.json` | `4e27b18677676be9327fc8b02ebd214c7ac70305d8d30f25d3dfa0aa13f0d1e5` |
| `probes.log` | `66b72f4fc2c5f22cd79d90348ae44db22af8c6dea4c57ef9ea689924bc2f52bc` |
| `red-proof-result.json` | `d472a3f40d127f772d427b475e8026b198e9aa33a4e18f59a275cebebbfc8b98` |
| `red-proof.log` | `fc3b69bdf3e00a678d8ceaeb0921599283cefdc9a236bfccb986822637674b35` |
| `audit-result.json` | `1d647055a7d824525bd2e7c75574e51012c6c541397ba79796e40ea4c8d8af1e` |
| `audit.log` | `cefdb05bb30afe94b1c7417ed1fa115182cfa916c965873c9d03e8a3ff307e3e` |
| `policy-result.json` | `e8ce23237febfb9cf0ef9697b069473d355c12b383d96a329faf8d25cea48bec` |
| `policy.log` | `1fbb2214160686592e0661857b04423856427bac9c21852e59d6ceaf96596cdb` |

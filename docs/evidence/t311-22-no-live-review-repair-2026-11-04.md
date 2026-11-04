# T-311.22: Same-row no-live refusal repair

Status: builder checks passed; independent core re-review required.
Implementation base: `9270a9ab06f57fe99469be7d010ee465cc93e30a`.
T-311.22 and parent T-311 remain open. Review cycle: second submission.

## Review disposition and running behavior

[Independent review 1](../reviews/9270a9ab06f5-local-coding-t311-22-job-oracle.txt)
is retained byte-identically with **Verdict: FINDINGS**. Medium M1 exposed an
actual false pass: v20 replaced the candidate's no-live helper with a refusal
stub. A helper returning normally or raising ValueError therefore passed the
original refusal assertion in the independent grader, although it failed the
unchanged legacy grader.

Oracle v21 recreates the trusted empty temporary directory at its exact lexical
path under the candidate's private `/tmp`, and leaves the candidate's own helper
installed for calls outside the original signal mock. The historical reference
refuses the absent run record before process lookup or signalling. Expected
HarnessError uses the existing closed refusal tag; unexpected exceptions remain
sticky candidate-exception failures without transporting exception payloads.
The trusted process still replays only into the original signal mock. A valid
forged call cannot invoke its live helper, and a target replaced between request
and response refuses replay. No new mount, network authority or cleanup path.

Low L1's unasserted return mismatch is repaired by discarding stop/control return
values; the frozen method never reads them. The finite signal stub now has the
original `(base, number)` signature and records only calls reaching its body.
A wrong-arity call caught by candidate code no longer contributes a signal.
L1's PurePath/subclass restriction remains explicit adapter policy, stricter
than original equality assertions; return and path-type limits are not evidence
of a frozen assertion failing. Low L2's export test remains an export test with
its comment corrected; new tests exercise control with a plain helper and a
helper replaced after selecting recorded dispatch.

## Verification

New real-namespace regressions were run against v20 before implementation. The
retained red run fails on both reviewer false passes and on all three stricter
transport cases. The repair then passes the same assertions. No hidden test,
source pin, selection, task statement, threshold or control profile changes.
The only prior test behavior adjustments are the old unit expectation that the
fixed no-live stub always raises, and the unsupported-return diagnostic, now
using an asserted export return instead of an unasserted wrapper return.
No failure threshold was relaxed.

All suites, real namespaces and policy validation use the shared build slot.
Times exclude reservation waits. Source/test bytes stay stable during the final
verification batch. This is synthetic evaluator verification, not a model score.

| Check | Result | Seconds |
| --- | --- | --- |
| Focused repair regressions and worker suite | PASS; 70 tests | 7.023632 |
| Scoped fast gate | PASS; 146 tests, syntax, Markdown and whitespace | 94.92957 |
| Unchanged legacy unit suites | PASS; 59 tests, no skips | 23.871492 |
| Independent verifier | PASS; 18 pairs, 23 methods, 187 reference calls | 9.458838 |
| Unchanged legacy verifier | PASS; 30 pairs, 89 reference tests, no skips | 44.516817 |
| Persistent namespace diagnostics | PASS; five methods, 27 reports | 6.949839 |
| Integrity audit | PASS; 123 reports, 302 log bindings | 1.534723 |
| Repository policy validator | PASS; all policy invariants | 151.404679 |

The independent verifier retains 36 controls (18 failing bases and 18 passing
references). Dev-028 keeps its base error after 13 calls and reference success
after 18 calls. No-live normal return now fails the original assertion;
ValueError is a counted candidate-exception failure; HarnessError subclasses
retain original acceptance. Three unasserted-return/arity variants pass both
graders. Existing wrong-signal, missing dispatch, unknown control, export,
bundle, forged assertion, malformed wire and confinement diagnostics remain.
Each independent report counts one pass or one failure. Private exception
payload checks inspect every artifact from the relevant refused candidate.

The audit checks all 30 frozen projections, reference/hidden/fixture hashes,
all report/log bindings, oracle-owned counts, stable source bytes, unchanged
sandbox, launcher, cleanup, legacy and fast-gate code, and the complete preserved
ledger plus all unrelated mission rows. Collection fingerprint remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
No cleanup recovery record was produced. Review 1 and the earlier evidence
remain immutable; this record adds the repair's actual measurements.

## Remaining obligations

Independent exact-commit core PASS is still required; no builder closes a row.
PurePath/subclass policy remains a low backlog item before generated admission.
The T-311.20 lows, T-311.21 duplicate-verdict low and model-lab environment low
keep their existing obligations. T-311.15/.16 stay open after three reviews;
this repair does not restart their review cycle. Remaining adapters,
aggregate/scratch quotas, host metadata, protected held-out inputs, coverage
admission, dev-003 policy and the real current-harness baseline remain open.

Generated scoring remains disabled. Model attempts and GPU minutes are zero;
tokens and model scores are null. No GPU/model execution, downloads, held-out
inputs, harness/prompt tuning, evidence cascade, long acceptance gate, public
release or owner-readiness claim. The policy validator is not the long gate.

## Artifact identity

Raw logs stay private; neutral labels bind their exact bytes.

| File | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `d6804aaf8fd75254e81f45a394ed71a2b657ec5a1e8437d7031a606e44f91e99` |
| `scripts/coding_eval_oracle_worker.py` | `c6ea88b4dab965a46ee807ac6e2ad2554c86b799089f06cdf6f37521eb7ad52c` |
| `tests/test_coding_eval_oracle.py` | `df58350f15d9a6e927bb42273ccf88abfa286b252d129102adee4e2242d85eb9` |
| `tests/test_coding_eval_oracle_worker.py` | `1c6dc9c8e283e1c17f783384f9e3088ffb617e5887524658777690082812f3fe` |
| `source-digests.json` | `3cb81dd51467bbc878db3568684cbc4bc9830402676bea454ff927ee6cb1d006` |
| `red.log` | `a772f966008a246018b0b821a7c9d53b45a8ac20240a3ccee2cc016c9a4dae02` |
| `red-result.json` | `e8675d29f4bc9355f27319b80896e49cb52effdd595f21fbfd54d36f01e8b47b` |
| `focused.log` | `e166b3eafaa6e7f8f76382d3e6410a77cf600d019c28f7dfd3f2da040dcdabee` |
| `focused-result.json` | `43003d405c7a64ecfa392892dc302c930a3b7acc3c266c1978481be152616f9c` |
| `fast.log` | `d4f2775a258d4052559cae0c82b701b67031f969c067321bada8295c37d2825a` |
| `fast-result.json` | `790b631ccecec78334211c2e4749d992fbc275825f116bbf8c5e0c631f3d7d07` |
| `legacy-unit.log` | `d77822a66e3c9cbb911605f174ba0b832f480005e8835ddf47c0927b670b41bd` |
| `legacy-unit-result.json` | `f896b264f0da5c428c9b8a921b222e11ef7a1af380e40918983edfb2cb3b2cde` |
| `independent/summary.json` | `7a00248192cd67b088eb580b832d8b23d4e4ef0807596beda84fe23064a31ce7` |
| `independent-result.json` | `5bfb7fff5031f43c102d9cb6faf4b710c83f0f5fdb615ef3a5a334a8e67d135c` |
| `legacy/summary.json` | `7dcbfb7e5ebd034c8ab0e510616e4fa960ea3c62f5fd9cfed143795b13455503` |
| `legacy-result.json` | `5bbcff841dd3f2d3dcba518fc4e6f84e5e8f91b10629a73de69948fb303a8915` |
| `probes.py` | `57bf9a56d6aa1a685717b4b8a1184da3721852d55343e526d2626d661393f4e0` |
| `probes.json` | `4a122c2fc535d10e8e326567f05f2372d642e8f1f8818130fb7086241a7902af` |
| `probes-result.json` | `a69bf417264efb1d3ca3d4929e660da87e72b3838294b93a88d5fea6d3d81e67` |
| `audit.json` | `ac6e293358f74c9f7ed8bc44334b65b9bab8dfb2dd7cb1fbf189eab989e12067` |
| `policy.log` | `b1d67c9ecbc42e2fb97d852b50f032e7098668b49bf7a2234e352fbd59f9a631` |
| `policy-result.json` | `e71302a5eebf5e78fe221c91d632014b1925e34f17397f0462e65d6df8a8cc10` |
| `run.py` | `4f10e66ef120c14dd8e01dda1ba744dd6708222e990440acb6839bfcbc2cdf9c` |
| `batch.py` | `0c712356e26fb112d9fbd92553f4dc8eb239c82207cb1b21dc505840ab673bc5` |
| `audit.py` | `145ecd6636d332ab4deb808df1c4510eeba363891ca074770ff45efe796e9695` |

# T-311.23: Expired signal mock observations

Status: builder checks passed; independent exact-commit core review required.
Implementation base: `149458e2205f2f309b38078a349f91d24d45a827`.
T-311.23 and parent T-311 remain open. Review cycle: first submission.

## Running behavior and review disposition

The [T-311.22 independent re-review](../reviews/1a1cbe865be5-local-coding-t311-22-no-live-repair.txt)
is retained byte-identically with **Verdict: PASS**; only that row closed.
Its low L1 exposed a false pass in v21: a candidate could retain an earlier
request's signal mock, then send an extra signal through it while handling a
later request. That call appended to a discarded observation list. The original
frozen method uses one mock across requests and fails the duplicate signal.

Oracle v22 expires each per-request signal mock in a `finally` block, including
when candidate execution raises. A later call through the mock or its original
side effect sets a sticky flag shared by the worker's job requests. Every job
operation checks the flag before and after dispatch, including parsing and
export. A caught exception or a reset of mock history cannot clear it. Such an
observation is refused as adapter policy and retained as one failed attempt;
it cannot produce a score. Mere retention without later use still passes.
Duplicate active calls continue to reach the original assertions and fail as
ordinary test failures. This is a bounded adapter policy, not an assertion that
the frozen test raises the adapter's exception category.

No request/reply schema, source pin, hidden byte, selection, threshold or task
statement changes. All old test methods remain unchanged. The sandbox,
launcher, cleanup, legacy evaluator and fast gate are unchanged. Candidate
Python still controls its own interpreter; deliberate mutation of adapter
objects is not prevented by the expiration flag. No trusted assertion, live
signal helper, model, remote endpoint or hidden input is exposed to it.

## Verification

Before the repair, the three new real-namespace stale-call variants all
incorrectly passed v21 while failing the unchanged original grader. The red
record retains three failures plus four worker-unit subtest errors from the
new state parameter not yet existing. After implementation, the focused suite
passes all 68 tests; the final fast gate also covers the subsequently added
retention-only and active-duplicate controls. No test or threshold was weakened.

All suites and namespaces ran through the shared build reservation; times below
exclude reservation waits. Source/test bytes stayed stable throughout the final
batch. Synthetic evaluator controls are not coding-model measurements.

| Check | Result | Seconds |
| --- | --- | --- |
| Expected pre-fix red | Three methods; three failures, four errors | 1.609695 |
| Focused repair and worker tests | PASS; 68 tests | 1.542961 |
| Scoped fast gate | PASS; 150 tests, Python checks, Markdown and whitespace | 98.037256 |
| Unchanged legacy unit suites | PASS; 59 tests, no skips | 23.733664 |
| Independent frozen controls | PASS; 18 pairs, 23 methods, 187 reference calls | 9.53187 |
| Unchanged legacy frozen controls | PASS; 30 pairs, 89 reference tests, no skips | 44.592872 |
| Persistent namespace diagnostics | PASS; 7 methods, 37 reports | 9.279475 |
| Integrity audit | PASS; 133 reports, 327 log bindings | 1.548101 |
| Repository policy validator | PASS; all policy invariants | 121.80112 |

The independent verifier keeps all 36 control attempts: 18 passing references
and 18 failing bases. Dev-028 keeps its 13-call base error and 18-call reference
success. Three stale-call variants (saved mock, saved original side effect,
and a caught call) fail both graders. The independent cause is sticky
`adapter_policy`, without protocol or candidate-exception failure. Retention
alone passes both; an active duplicate fails both via the original assertion.
The existing no-live, mock-arity, unasserted-return, forgery, malformed-wire and
confinement probes remain passing diagnostics. Each report retains its failed
or passing attempt; no denominator drops. Payload checks remain in those tests.

The audit checks all 30 frozen projections and reference/hidden/fixture hashes,
all retained report/log bindings, trusted test counts, stable source bytes,
unchanged sandbox/launcher/cleanup and legacy/fast-gate code, every pre-existing
test method, the old ledger and unrelated mission rows. No cleanup recovery
record exists. The collection fingerprint is unchanged:
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.

## Open scope

T-311.23 requires the separate reviewer's exact-commit core PASS; the builder
does not close it. T-311.22's PurePath/subclass policy remains explicit low
backlog before generated admission, alongside the existing resource/collector
and model-lab environment lows. T-311.15/.16 remain deferred after three reviews;
no review cycle is restarted. Remaining adapters, generated admission,
aggregate/scratch quotas, host metadata, protected held-out inputs, coverage,
dev-003 policy and the real current-harness baseline remain open.

Generated scoring is disabled. Model attempts and GPU minutes are zero;
tokens and model score are null. No model execution, GPU use, download, held-out
run, harness/prompt tuning, evidence cascade, long acceptance gate, public
release or owner-readiness claim. Policy validation is not the long gate.

The first final artifact check caught the missing generator binding in the
evidence table. The binding was added, retaining the 28-file check; its failed
preflight log remains private. Source and test results were unaffected.

## Artifact identity

Raw artifacts remain private. Neutral labels bind their exact bytes.

| File | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `703ae3fd8cca7f7bf67a63be1cb0f391c9499105f5ff83f9d505f9075ddf2549` |
| `scripts/coding_eval_oracle_worker.py` | `3327567bac8844eb14760087e836172f3f41d4f9f3def8739b751e8e4a06e066` |
| `tests/test_coding_eval_oracle.py` | `f8bb6db316c8634ac5141d7774dddc4298010d00710c12a2615d450c99a0d1e4` |
| `tests/test_coding_eval_oracle_worker.py` | `39ffc37c8b80a84a3fa70734507aa1f0c21e39b8d604f9c0c75156654e33aa99` |
| `source-digests.json` | `69ed9870879cf166049c96c73dc77bd2a4c34f29b82acc548be855edb0f2f889` |
| `red.log` | `34d017004eb48cb5cea7071d96c1b69b6dc6cb458e68be4aa6a3bad3cdd0386c` |
| `red-result.json` | `5ecde498eaa4973e275097e1ed4bbef675fd0c08cd256b256eebd1f3329ee134` |
| `focused.log` | `5ea5986cf33b1f74c593abfc5ec6650d3c28618de78b99bfe124a9a7fc6ac949` |
| `focused-result.json` | `e90e62c9d66a19836722885c6f9d16f7bd670ddbe22e31407b6e05db894c5790` |
| `fast.log` | `750a4f19cc174521985404a67fb74cb408e68040a29e07e44b4961c813ca1399` |
| `fast-result.json` | `94fc614b52fd47d43b621eb6e4fdba4015815f45c858756790bc9f7e8e234c9e` |
| `legacy-unit.log` | `02a761512007a979bd4de8278ff3729e1efb769bb98d884436fc40b9c8d3b391` |
| `legacy-unit-result.json` | `06dc159e0f4594cb4acb29eec13d81e9c0680580889c850e7c084c3b3612f683` |
| `independent/summary.json` | `a21ac1c32f674b67b83621a5520db98ca6cddecc30000b67b813f1c3d430278e` |
| `independent-result.json` | `f12b777b76eb447d96b116c33183cae8c173ceebd6772647eaa81337a859496d` |
| `legacy/summary.json` | `2bd23b38b4a533d9cd783be2e6ab52505640255e382a8a305405de8ee512974b` |
| `legacy-result.json` | `7db0e7e892bf3d10400049341dfe8e220de6e1597df6bb2a9a7c871fea767788` |
| `probes.py` | `3e3e06923ab2498fa970457a5348c0f9075e268fb814a051dc8bdf00f0a46d43` |
| `probes.json` | `f5a1fe0a726b520c939edbbc01614ab0ae6bd58a929ee536424957dc6ae967b6` |
| `probes-result.json` | `693a5b028f1e35ff1ca7b8f3e1a11f29647b625ad64e659a91dd8960d0028553` |
| `audit.py` | `f2090e2117e228a27fc04d3f91bfa53238ae3070c85f590d68e8da0291f66842` |
| `audit.json` | `f217f9221450da3eeae5daf6cc617fbcc3016b6cd70db9ab535c14b4309c5ab7` |
| `audit-result.json` | `03db4e833a1b0d0861c842b07f24ec8a331850c7c376f74d1fc9267e93bebef6` |
| `policy.log` | `06f0020ae117bedeea8d0f1f4d4d8bb6e927f49b6e9fdb1d2307d104ec999987` |
| `policy-result.json` | `6a4826e75523e24ed03669c14e22d24b94d3d6ea2837fdda95c04bac71164e29` |
| `run.py` | `d6a98947e2c5043c8929f36702cbe289e4e419af4e697572a0bc12fb234d91b8` |
| `batch.py` | `155b3eb3b72130f79bed4694689ecdb3565ce7365614ec99f0afec1ee6cbdcd1` |
| `write-evidence.py` | `ac61b1feca8edda550dc3f0b00ba9e936ac0fd1144cb720cd0e0f922d57f62ec` |

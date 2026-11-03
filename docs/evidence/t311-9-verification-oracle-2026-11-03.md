# T-311.9: Independent verification functions with private fixtures

Status: builder implementation; independent core review pending. T-311.9 and
parent T-311 remain open. The unchanged independent PASS for exact commit
`f56dba24bf5377388b56e8928babfa6325e8d718` is retained in
[reviews](../reviews/f56dba24bf53-local-coding-t311-8-path-oracle.txt)
and closes only T-311.8.

## Running behavior

The [independent verifier](../guides/coding-evaluation-oracle.md) now runs six
existing frozen development tasks, including dev-020 and dev-021 acceptance
predicates. Candidate code and unchanged hidden assertions execute in separate
confined namespaces. The trusted proxy snapshots synthetic log/file text and
stub observations. Only fixed labels and JSON data cross bounded pipes; no
fixture pathname, shared filesystem, mock object, callback or test source does.
Each candidate invocation rebuilds its own temporary fixture and removes it
after return or exception. The original selected assertions inspect candidate
`passed` and `checks` values; only the trusted oracle owns the verdict and counts.
A fixed missing-case tag reconstructs a local KeyError without transporting
candidate exception data.

| Task | Base methods / failures / errors | Base calls | Reference methods / calls |
| --- | --- | --- | --- |
| dev-010 | 1 / 1 / 0 | 1 | 1 / 4 |
| dev-012 | 4 / 10 / 0 | 12 | 4 / 14 |
| dev-015 | 1 / 1 / 0 | 3 | 1 / 4 |
| dev-020 | 1 / 1 / 0 | 2 | 1 / 3 |
| dev-021 | 1 / 0 / 1 | 1 | 1 / 6 |
| dev-029 | 1 / 0 / 1 | 1 | 1 / 6 |

Every base must reproduce its exact failure/error/call profile; every reference
must pass with its exact call count. The two new methods remain complete and
unchanged. All six controls total nine selected methods and 37 reference calls.
They are synthetic predicate checks, not actual runtime/tool/validation or
model runs. Other report metadata, unselected cases, `run_case` integration and
unselected statement clauses are explicitly ungraded.

Reports now label path return container/element types as ungraded (T-311.8 low
L1). A real probe confirms that path strings can pass after wire conversion;
this adapter cannot establish that interface clause. Each report also counts
one control invocation as either pass or failure, and the summary includes all
12 reports: six expected base failures and six passing references. Tests cover
infrastructure, import/startup, input-integrity, timeout, output, protocol,
candidate-exit and assertion failures without removing them from these counts.
These are control invocations, not model attempts. T-311.8 low L2 remains an
explicit future admission obligation: generated failures must stay in the
attempt denominator, including syntax errors and early exits. Generated scoring
is still refused, so no generated-attempt accounting is claimed.

## Observed verification

The following commands ran sequentially under the shared build reservation.
Times exclude waiting for that reservation.

| Check | Result | Wall seconds |
| --- | --- | --- |
| `oracle-units` | PASS, 39 tests | 11.361906 |
| `existing-units` | PASS, 58 tests | 19.555631 |
| `oracle-controls` | PASS, six valid base/reference pairs | 2.838327 |
| `full-controls` | PASS, 30 valid pairs / 89 reference tests | 43.85183 |
| Scoped fast gate | PASS, seven paths / 39 oracle tests | 12.855587 |
| Documentation policy validator | PASS, 706 Markdown files / all invariants | 132.594679 |

New probes refuse forged assertion patches, always-true verification results,
arbitrary fixture paths, non-Boolean checks and exception payloads. Fixture
cleanup and mock restoration are checked on both success and error. Generated
source edits are rejected before creating output. Existing startup, interruption,
timeout, process cleanup, framing and confinement checks remain selected.
All commands passed on their first completed run in this batch. Afterward only
TASKS.md and these evidence measurements changed; the four measured source
digests stayed identical. Final Markdown, whitespace and added-text hygiene
checks cover those documentation edits. The long acceptance gate did not run.

## Integrity and remaining work

All six legacy evaluator source/test files are byte-identical to `ee490972`.
The 30 hidden inputs, frozen Rust corpus and collection fingerprint were checked
without altering any source pin, statement, method selection or threshold:
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
The retained independent review is byte-identical to its original report.

Adapter version: `coding-independent-oracle-v3`. The other 24 tasks still need
independent result checking. Aggregate worker quotas are unavailable: the lane's
cgroup mount remains read-only; per-process limits are not aggregate quotas.
Protected held-out inputs and a real current-harness baseline remain open.
No harness, prompt, model or task is tuned. Model attempts and GPU minutes are
zero; tokens and model score are null. No long acceptance gate, supply-chain
cascade, host qualification, release or owner readiness is claimed.

## Source and raw-result bindings

Raw logs and summaries remain private and are located by the handoff. The two
control-command stdout logs are empty; the separate bound summaries contain
the results. Source digests bind the actual bytes used for these measurements.

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `30207a1b85a70d523cf3b78a9c829446cba44fe47d0c6df1f2b31a1622690d60` |
| `scripts/coding_eval_oracle_worker.py` | `3dd1e584c800b1ad32d5649fa2f47c9ef7a88f14efc2aea93cf2f374d5c81fd9` |
| `tests/test_coding_eval_oracle.py` | `4d4f3ec456cf1713fb2e03d7d419508267daed85ecb19483fd3c279e38a83a07` |
| `tests/test_coding_eval_oracle_worker.py` | `fcd4eadfd2bfbc0588585f4f60ed561c9c0594d55d7cb260a4f1dd2ebe706b40` |

| Private artifact | SHA-256 |
| --- | --- |
| `oracle-units.log` | `4f11aaada6f1799239e1461a5d70ac54eb9bb7a9a643ca1e5e74cfb22fc12b15` |
| `existing-units.log` | `6206cb3c1e06b0168be53a42f3f686ee3d7a217fabba6fd7deff77fafee91198` |
| `oracle-controls.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `full-controls.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `checks.json` | `e8dff674ac9bdf01f17495343672770bf319960d718a2c465aa5686df622094e` |
| `oracle-controls/summary.json` | `c37d36330667024bf697e3f5c4a53dd4490ca76fcf5ea3fd02a8ce7dc1822a78` |
| `full-controls/summary.json` | `d2370debd0fc0306f7972a2e5c5a175e5f30931b6ab02e90ff13c14d3c1f16bb` |
| `input-audit.json` | `4d5e372516408df2438be468209cfa8dd1f266703deebb8a5a0e8e6c0aa7f3ba` |
| `fast-gate.log` | `df996e13fc1108aa800f32d5ee974ea6b7edcc040abfda1d04bc210dca048a6c` |
| `policy.log` | `a0ab57dc8e7f766d639b9c28cf1ca89c40fd4bd37da57b28f8f3896236238d7b` |
| `final-checks.json` | `9cc3e77d929651374c32a5e19e87765678ea64e16578b813e992c64cfe375f8f` |
| `results-audit.json` | `39b50afd8c570577b9a0c52972e8786ba902eea72087162d9e6b10210c4cd4ed` |
| `hygiene.json` | `3a7a74f3a37b377a2ac19573e7a2fff904480ccd7447d9dff56d5a8c53e4693f` |

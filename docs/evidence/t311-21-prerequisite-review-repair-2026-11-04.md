# T-311.21: Command prerequisite review repair

Status: builder verification passed; same-row independent core re-review required.
Implementation base: `ebc3a18800e4ee2acd8b8d2daa19c7799274b077`.
T-311.21 and parent T-311 remain open.

## Finding and repair

[Independent review 1](../reviews/ebc3a18800e4-local-coding-t311-21-deadline-oracle.txt)
found medium M1: refused command declarations produced valid zero-test records,
but the host rejected them, labelled the attempt infrastructure, and discarded
its counts and sticky flags. The pure validation regression reproduced all three
rejections before the repair; its failing log is retained.

Oracle v19 accepts a ready zero-test record only for the existing case-root or
command-declaration prerequisites, with exactly one call, all six test counts
zero and a sticky protocol, policy or candidate-exception failure. The existing
classifier then retains the corresponding cause, flags, counts and published
`tests.json` after cleanup. These attempts remain failed controls; no successful
zero-test record is accepted. No worker, assertion, task definition, pin, test
selection or threshold changed.

The new real-namespace regression exercises deleted declarations, unsupported
mapping declarations, empty argument lists, raising module attribute access and
a malformed wire observation. It requires each cause, exact zero test counts,
one call, the published record, matching sticky flags and a failed-attempt count
of one. Private exception payloads must be absent from every log and result file.
A pure validator regression rejects altered call/test counts, absent flags,
invalid readiness/function metadata and zero-test records for every supported
task without a prerequisite. Existing case-root and startup tests remain unchanged.

The report ends with FINDINGS and one reason line. The fast gate now accepts that
exact review format, like its existing INCONCLUSIVE format, so the report can be
retained byte-for-byte. A pre-fix regression failed; the repaired checker accepts
this format and refuses empty/extra reasons, reasoned PASS, unknown verdicts and
trailing whitespace. Existing checker tests and all other rules stay unchanged.
This format check establishes no independent approval or row closure.

## Measurements

Pure record and review-format checks are lightweight. Namespace tests and the
complete validation batch used the shared build reservation; queue time is
excluded. An initial pre-fix namespace reservation waiter was cancelled before
any test or namespace started; the retained cancellation note makes no test claim.
All final measured verification commands passed first run. Source and test bytes
remained stable throughout that batch; final document lint follows this record.

| Check | Result | Seconds |
| --- | --- | --- |
| Pure record checks | PASS; four tests after reproduced pre-fix rejection | 0.260467 |
| Review-format checks | PASS; three tests after reproduced pre-fix rejection | 0.098562 |
| Focused checks | PASS; three tests, five real-namespace refusal variants | 1.253636 |
| Scoped fast gate | PASS; 100 tests plus Python, Markdown and unchanged-review format checks | 86.844403 |
| Unchanged worker suite | PASS; 56 tests | 0.104464 |
| Legacy unit suites | PASS; 59 tests, no skips | 23.402443 |
| Independent verifier | PASS; 17 pairs, 22 original methods, 169 reference calls | 8.754894 |
| Full unchanged legacy verifier | PASS; 30 pairs, 89 reference tests, no skips | 43.925077 |
| Retained diagnostics | PASS; 24 reports from new refusals and existing deadline/legacy/confinement controls | 5.527018 |
| Integrity audit | PASS; 118 reports, 286 log bindings, 30 frozen tasks | 0.288136 |
| Policy validator | PASS; 723 Markdown files and all policy invariants | 142.2823 |

The independent verifier retains 34 controls: 17 passing references and 17 failing
bases. Dev-026 keeps its original one deadline failure and passing timeout method
on the base; the reference passes both methods, with three calls for each control.
New prerequisite refusals each retain zero methods, one call and a failed attempt.
Diagnostic-only synthetic admission overrides are confined to tests. Production
entry points still refuse generated snapshots before creating output.

The audit confirms the worker, six legacy files, every existing test method,
launch/sandbox/cleanup functions, all frozen inputs and the old ledger suffix are
unchanged. The review is retained byte-identically. The collection fingerprint
remains `0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
No cleanup recovery record was produced. Independent review must decide whether
M1 is resolved; this builder record does not close it.

## Remaining scope

T-311.15 and T-311.16 stay open under the three-strike rule; no fourth review or
cleanup repair was attempted. Generated scoring remains blocked on remaining
adapters, coverage admission, aggregate/scratch quotas, host-path metadata,
protected held-out inputs, dev-003 policy and a current-harness real baseline.
T-311.20 equality/subclass and sticky-count lows stay recorded; model-lab inherited
environments return before T-312. Collector observation coverage is unchanged.

No model, GPU, held-out input, prompt/harness tuning, download, supply-chain
regeneration, long acceptance gate, release or owner-readiness operation occurred.
Model attempts and GPU minutes are zero; model scores and token measurements
remain null. This is one same-row repair, awaiting review 2.

## Artifact identity

Raw reports and logs remain private. Neutral labels bind the measured bytes.

| File | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `841589362f14b05f34e49c0ea27555afcf8898244d0ece0edf51dd916851b0f4` |
| `scripts/coding_eval_oracle_worker.py` | `9d2642c3f2da5d88f635b6c94ea783655a9315f04172bb5828e32f8e14993de9` |
| `scripts/local_agent_fast_gate.py` | `5ea99ea503afde0a13edcdff29b23525cfd1e8a550f34c57d9cbc4d0d58a1e7e` |
| `tests/test_coding_eval_oracle.py` | `b5e8316bd09beb296d69debce24a624cad2d23c9b0db6cb08886a46168aa705c` |
| `tests/test_coding_eval_oracle_worker.py` | `5120cd304c08b1eff247d0f9ac3a070a5273477d133baa84e62a66a66fb44023` |
| `tests/test_local_agent_fast_gate.py` | `29c4493b23a55040cb62c3b5c7d7e6e62003928c868196fd0b5b6e1e6898dcbd` |
| `source-digests.json` | `74d77def25e102dbc91d9db6974b5e2d7daf0ceb51ecbaae430839108b51843a` |
| `red-result-record.log` | `d12edba0803b2b9bfe025407ae7b6782208d28ba358b3df38fcb73af9fc31cfb` |
| `red-result-record-result.json` | `8f8551bb68f1cd5e2297874f1e059dad26e60b8a41a2592bc562c3f37d6a7a49` |
| `red-review-format.log` | `7632258f1b2fbb96d3326ed407f095bd988fc5ab366006e74b430769126ef6b3` |
| `red-review-format-result.json` | `411f331e3f1b52d29b5176a7bea5153e055473b8196d964022964132351a8ab3` |
| `cancelled-red-namespace.txt` | `b84720d0827eaa058d451656786f28f54c986be615789a59bcabceed11dcbbe5` |
| `focused.log` | `cbd40e866c48ba21676973192cd11bdc75641a13435be9c2e168e4760a7f2f06` |
| `focused-result.json` | `f1d24df203a41e160e0216514e2885e077f79690f65262addb17e1a587f80d4d` |
| `fast.log` | `2dc38340aaab5ae025e55f1a738e10f911490b3256d96e7df371f2cc09fa0b1c` |
| `fast-result.json` | `daf05471694d94c8d2017063d79f65685209eed05b860f827315c35ab97e56f2` |
| `worker.log` | `d039043d1dbe8e4615f0b164b0d92c74088c88a3771f1a2e93db9cc8a3bd3f99` |
| `worker-result.json` | `0e1ceb5ec2bb869365dc59a0bb785a97862d8a2249fea4a302432d4cda800472` |
| `legacy-unit.log` | `7cf968dc3600b68c313391d7f50c8631093022e169a3d0d52c08236b753520d2` |
| `legacy-unit-result.json` | `98c294b314b2fdfeb8aae1acce6a323977e31a7a820696e8f6c8cc4d16b366cd` |
| `independent/summary.json` | `b7e19abdee2004fc8fedb033c621e4424b8cd8a46b5ff1a9f95e382ca7193840` |
| `independent-result.json` | `eda48539e7ecbfadc6348f87009d5f67f8b782c6d8c4bb1701df4e3d34f6392d` |
| `legacy/summary.json` | `c86990d1e363ecc1214cec18891130dff99abc14c4f278159c72b743dca949c3` |
| `legacy-result.json` | `03c219b5d86c8a442af9b84f5dcc16535986b77112a78f45dbe007bab9c04348` |
| `probes.json` | `55c6b368b62e57e2c8eefce2e48224cd3b8b304a08f3013206338ec4f7fc9f7b` |
| `probes-result.json` | `7ddfc7c522c3686b594dd06846d98d8bdfaaf855c281533c7d511e13e7aab34c` |
| `audit.json` | `afec501c32a5c944fe2ba2c10a793fb32ec74a21520ab126e6f40a9df7e34179` |
| `policy.log` | `04ef06ce708e5be964465643682ae74ec9da4dd7fe628f52548414ce99747aa6` |
| `policy-result.json` | `4a2ce605cd1fdc6f2c6d3e7fb4088e28096a84f75ca3021e174343faeaa6bcf3` |

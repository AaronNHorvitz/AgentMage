# T-311.14 capture-stub review repair

Status: builder verification passed; independent core re-review required.
Implementation base: `0d0395bd`. Same row T-311.14 and parent T-311 stay open.

## Repair

The unchanged [independent review](../reviews/dfb751e41a44-local-coding-t311-14-capture-oracle.txt)
of `dfb751e41a447d5b434d2f26b29a423bdfdd0a1b` found medium M1:
dev-018's original fixture uses a mock, while the independent adapter recreated
a completed-process object. Their default attributes differ, allowing the two
graders to disagree in both directions. No generated candidate or model score
was admitted by either version.

Oracle v10 carries a closed `stub_kind` string alongside the existing finite
stdout, return-code and command-list data. The trusted proxy selects
`completed-process` only for the exact type with empty list arguments and null
stderr; it selects `mock` for the original mock fixture. Other shapes fail as
trusted protocol before a request is sent. The candidate recreates that local
kind with the original stdout and return code, preserving mock attribute
behavior. No mock object, callback, exception object or executable payload
crosses the pipe. Fixture commands remain stubbed.

Three regressions compare both graders on identical diagnostic source bytes:
merging stderr, calling a method on stderr, and checking completed-process type.
The two tasks intentionally have different outcomes because their frozen stubs
differ. Complete original hidden methods, selections, source pins and thresholds
remain unchanged. Unsupported request kinds and extra request fields fail
before candidate function invocation. The trusted proxy also refuses nonempty
completed-process arguments or non-null stderr before sending a request.

The repair also addresses the three review lows:

- L1: removes the blank line that broke the original evidence table rendering.
- L2: declares dev-018's home-alias variant `not_graded`; both graders retain
  their original `/tmp` home, so that unchanged fixture branch does not execute.
- L3: preserves adapter `ProtocolError` in the candidate dispatch loop instead
  of converting malformed requests into candidate-exception tags. Tests cover
  capture, argument and verification requests before any candidate call.

The [guide](../guides/coding-evaluation-oracle.md) records these boundaries.
Existing admission, sandbox, quotas, budgets, sticky failures and counting
remain unchanged. This is a same-row repair, not a split or a closure.

## Verification

Checks run through the shared build reservation; times exclude reservation
waits. All runtime source and test edits preceded the final fast gate. Final
Markdown checks are recorded after the evidence and ledger update.

| Check | Result | Seconds |
| --- | --- | --- |
| Scoped fast gate | PASS; 83 oracle unit/integration tests | 44.203765 |
| Legacy unit suites | PASS; 58 tests | 19.747755 |
| Independent verifier | PASS; twelve pairs, 15 methods, 157 reference calls | 6.044379 |
| First full legacy verifier | FAIL; dev-003 reference process-stat race | 43.726092 |
| Unchanged full legacy repeat | PASS; thirty pairs, 89 reference tests, no skips | 44.057169 |
| Existing diagnostics | PASS; 23 probes, seventeen legacy contrasts | 9.039649 |
| Original reviewer stub counterexamples | PASS; six oracle/legacy agreements | 2.741617 |
| Policy validator | PASS; 713 Markdown files and policy invariants | 135.507233 |

All twelve independent bases retain their exact expected failure profiles and
all twelve references pass: 24 counted control attempts. The private replay of
the reviewer's six original stub counterexamples now requires both graders to
agree, as do the added committed integration regressions. The review report is
retained unchanged; no independent PASS is claimed by these builder checks.

| Counterexample | dev-005, both graders | dev-018, both graders |
| --- | --- | --- |
| Merge stdout with default stderr | PASS | FAIL |
| Call a method on default stderr | FAIL | PASS |
| Require completed-process type | PASS | FAIL |

The stderr-method failure for dev-005 is a counted candidate exception in the
independent oracle; the legacy original method also fails. No failing attempt
is dropped. These are development controls, not model scores.

The initial 83-test run failed because
the new contrast test compared its observed tuple with the entire per-task
expectation map instead of selecting that task's entry. The test now selects
the intended existing expectation. The failed raw log is retained. No frozen
assertion, hidden test, collection input or threshold changed for this correction.

The first full legacy verifier completed all thirty pairs but failed dev-003's
reference: its frozen process-lifecycle test read a process stat file after
that process disappeared, raising `ProcessLookupError`. This is the previously
recorded development-fixture race. All initial reports and logs are retained;
an unchanged full repeat is recorded separately, never substituted for the failed
run. No hidden test is edited or skipped. A fixed dev-003 flake policy remains
an obligation before generated scoring, as recorded on T-311.5.

## Integrity and artifact bindings

All 196 retained reports and their log bindings were audited, including every
report from the failed full legacy run. Oracle-owned counts and sticky failure
flags match the parent reports. All thirty frozen source/test/corpus pins verify;
six legacy evaluator source/test files are byte-identical to the implementation
base. The retained review is byte-identical to the reviewer original. Collection
v9 remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
No runtime source or test changed after the passing final fast gate.

Artifact labels below are relative to the private batch directory in the review
request. Raw task data, diagnostics and failed logs stay private.

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `ee181eed09e7f247acc13b5c95fdc8c6ee151326c4a7e2d8c012d7a71a26e989` |
| `scripts/coding_eval_oracle_worker.py` | `37602f03d666ec0b893f549ef2f2bd5498e07c5f4a558721888b544e6c0d6e07` |
| `tests/test_coding_eval_oracle.py` | `3269df29fca770e54157d194961adac5d9e3d7a4851f73bc7b62bcf08b5898a7` |
| `tests/test_coding_eval_oracle_worker.py` | `1f787163b2d238f9f6c40fff49ca9235f81f91256cd63c01b5745c6e70972302` |

| Artifact | SHA-256 |
| --- | --- |
| `initial-tests.log` | `4eae76766db137d30a96f4a7bfe7e0aee0dcbf259fca319b9f4e29e031537ac0` |
| `fast-gate.log` | `b713acd2ee31c670d250964d6ee6c547a4d2d151d434b75d7ae09fbce3befc2d` |
| `legacy-unit.log` | `4c802097db2fa5fe12967cd7db66f5d2f92991d7a57c9e996e56baa04d26981f` |
| `independent/summary.json` | `7a93d7e483d97a000dc7ba6e6edabacf733fb2baf7e4ba935420bb1c8756d93b` |
| `legacy/summary.json` | `67240cd764eb62ae1181906b7a9d962a0f40271061678cf9e4fd64f2b00d26e8` |
| `legacy-repeat/summary.json` | `44d0428d857d5ab62538bb4cacd9fdf34c5816dae2ae71085314d7b6a30f2cf5` |
| `probes/summary.json` | `66a20734b5e4d6cf9e00c2a80bd500e7a4f003b281b93c9bf0f5c94029a9563b` |
| `probes.py` | `50ee2aab6045a888310f890c9379c4c75f521a073793ee432197756ceaea1818` |
| `stub-contrasts/summary.json` | `3131e04ea4a8d9d718aebbad483012232a0cbda8bcaef1e9e3bbb0d87900d0e7` |
| `stub-contrasts.py` | `6ebe5812ae706626fab7e56415afde7f927f15684aa27a02d3d2b55b71f864e2` |
| `stub-contrasts.log` | `afd63bd8940ae0be6093204101b3748eeb76ce87ac61d008df7c106d68d471ad` |
| `policy.log` | `f78ce718e06a19722d3d595b0d15272d63485f93619bdd26428855ff25ce314f` |
| `verification.json` | `587abc4208662d3d5fae144b0a41deeb80e9912edb1e205ba06319a5f2f21a6c` |
| `continuation.json` | `e2abbd19f170b713f6284100d88827a96fc89c352530269cd07b7a2a49c8356b` |
| `source-digests.json` | `6c41d4937cbe73dd419f5412996c1a4615554fa9162cafdedc0ac08ac2e5cf93` |
| `verify.py` | `5383b62195d2ddf49acac82211c4fcd18fb05ff297cfe8d54cf2ffc99bf1eb73` |
| `continue.py` | `ec9504d44f83b6550459b25b7f5271e284741599b73806a476fedf8e033d785a` |
| `audit.py` | `da1c39c1c716f238052c34a621d0f3c7c5bd54afbb2588130209bfeb6241de1f` |
| `audit.json` | `3c66246d0ea0a0f2a136a04eb284de852a6fa47ce437d659050df952cfdc64ec` |
| `audit.log` | `7de1f0a26cb7bee4b0550132f20c6026de922f6073d6e11b05aa09dc0d947f63` |
| `frozen-input-audit.json` | `ba9f42e887b9ece870caac390efd18fef19f8734315aef07823bfe65f16193a9` |

## Limits

Independent grading still covers twelve of thirty development tasks. Generated
scoring, eighteen further adapters, aggregate worker quotas, protected held-out
inputs, covered-clause admission and a real current-harness baseline remain
open. Parent T-311 stays open; T-311.14 requires a second independent exact-commit
PASS with no open high or medium finding. No GPU/model run, held-out evaluation,
harness tuning, model download, supply-chain regeneration, long acceptance gate,
release or owner-readiness claim.

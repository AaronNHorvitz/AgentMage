# T-311.6: thirty frozen development tasks

Date: 2026-11-03. Tier: core. Builder record; independent review required.
Base: `a5b0a4eb`, retaining the unchanged exact-commit T-311.5 PASS.
Parent T-311 remains open. No model, GPU, held-out task or agent tuning is used.

## Running scope and frozen inputs

`coding-development-v8` adds ten distinct historical source changes to the same
`list`, `prepare`, `score` and `verify` commands. The collection now contains
30 development tasks: 29 Python tooling tasks and one Rust contract repair,
with 89 reference regressions. The new tasks select 12 complete original methods
from their complete historical modules, without extracting or changing hidden
test bytes. Each pins the base, reference, source digests, complete editing
projection, hidden module digest, selected names, statement and exact test count.

All 20 accepted task dictionaries remain unchanged. The previous collection
fingerprint is still reproducible from those definitions, including the Rust
task. Only new task data and the evaluator version change in production code;
the sandbox, driver, resource limits, admission guard and cleanup do not change.
The full v8 collection fingerprint is
`91f1eecfa91129f2f8392363aaff0e8f49243aa761947e69d5ec1452d7b4b04d`.
Reports already bind the collection module and verifier summaries bind the full
definition fingerprint. No new dependency, licence, model or external dataset
is introduced. Historical input licensing and import readiness are unchanged.

The [guide](../guides/coding-evaluation.md) gives every base/reference pin;
the [task module](../../scripts/coding_eval_collection.py) fixes the full
contracts. Each addition represents a separate historical source change,
not an assertion promoted to a task. The command list is still development-only;
these public historical inputs are not a blind or held-out evaluation.

## Statement and coverage audit

Statements describe callable names, exact check keys, error behavior, argument
shapes and literal digests where a historical assertion requires them. They also
state which behavior the frozen methods grade. This does not claim complete
statement conformance from a passing selected regression.

| Task | Distinct change | Frozen method coverage and limits |
| --- | --- | --- |
| dev-020 | Multi-file mandatory-record accounting | Synthetic records require nine tools and more than 33 verified artifacts; tests lower each independently. Commands/records are mocked. The `run_case` recording option is specified but not exercised. |
| dev-021 | Read-argument correction and terminal denial | Synthetic correction/denial, retained argument digest, refusal of authority on the rejected turn, extra tool and absent terminal code. No real read worker or model runs. |
| dev-022 | Command artifact ownership | Synthetic tool order and exact failure-output digest/size; moving its artifact to another turn or changing its digest fails. No native command or approval executes. |
| dev-023 | Stable implementation identity | Real grading-snapshot Git metadata and file hashes; three executable paths are substituted with an existing inert system binary. Concurrent mutation and `start` logging are not exercised. |
| dev-024 | GPU observation and sampled pressure | Two methods mock all device output: numeric/malformed/multiple-device parsing and pressure error/peak. No GPU, model, locking, cancellation or aggregate quota is exercised. The guard samples; it does not signal a process. |
| dev-025 | New-file and multi-file fixture construction | Real temporary synthetic Git repositories, expected source presence/absence, and a validation child process that must fail on the intentionally broken fixture. No agent or model runs. Validation is not also run after a repair. |
| dev-026 | Finite collector deadlines | Two methods stub executable lookup and every command, check exact argv/timeout/capture behavior and prevent a timed-out collector from writing a report. No actual long documentation gate runs. |
| dev-027 | Closed extension arguments | Exact argument lists, scope/path/license/digest/identifier refusals and mutually exclusive parser options. No catalog, sample copying or extension execution. |
| dev-028 | Route export and support-directory options | Existing signal-dispatch controls plus route-range, parser and directory validation assertions; signals are mocked. No bundle is written, export approved or `start` propagation exercised. |
| dev-029 | Suspension/continuation observations | Synthetic success and missing suspension/continuation, refused resume, duplicate start and filesystem mismatch. The historical helper checks checkpoint identity/order, not checkpoint digest/sequence equality; no durable runtime resumption occurs. |

The two task types involving GPU counters and long command deadlines use only
synthetic observations. They need no GPU reservation or long suite. Real fixture
creation and its validation process stay inside the existing no-network scoring
sandbox. Other commands are mocked as described; none are model-quality results.

Before freezing these additions, static inspection rejected the original live
control introduction (`ea16322f`): its editable source does not exist at its
parent, so the unchanged projection cannot represent that task. The recipe sample
change (`e71970ed`) requires committed fixtures outside the scripts projection.
Neither was admitted as a task or tested against a model. Other inspected changes
without usable regression changes were not selected. No accepted task was
removed, hidden method rewritten or threshold lowered.

The guide now fixes a once-per-attempt policy for the dev-003 disappearing-process
race disclosed by T-311.5: preserve the failed result, keep it in the denominator,
retain logs and label it; diagnostic replays never replace a score. Generated
report labels and per-task coverage labels still require implementation before
generated admission. The hidden module and scoring behavior remain unchanged.

## Verification

Checks ran through the shared build reservation. Times below exclude waiting;
the full verifier time is the sum of its case timers, not total CLI wall time.

| Check | Observed result | Measured seconds |
| --- | --- | --- |
| New-only historical controls | PASS, ten failing bases / ten passing references | 5.666578 total probe wall |
| Scoped fast gate, base `a5b0a4eb` | PASS, eight paths / five commands, all 57 unit tests | 20.49 sum of command timers; units 18.944 |
| Full verifier, one command | PASS, 30 failing bases / 30 passing references, 89 reference tests | 39.891578 sum of case timers |
| Documentation policy validator | PASS, 701 Markdown files and all policy invariants | Not separately timed |

The full verifier ran 178 test invocations across both controls. Every base has
cause `test_failure`; every reference has cause `none`. No skip, expected failure,
unexpected success, truncation, input refusal or infrastructure failure occurs.
All cases bind the tested evaluator/task-module/Rust-adapter bytes and retained
raw logs. Every case records zero model attempts, zero GPU minutes, null tokens
and null model score. There is no first-attempt or retry model success claim.

| Added task | Base failures/errors | Passing reference tests | Base/reference case wall seconds |
| --- | --- | --- | --- |
| dev-020 | 1/0 | 1 | 0.221309/0.213710 |
| dev-021 | 0/1 | 1 | 0.214581/0.216993 |
| dev-022 | 0/1 | 1 | 0.216896/0.210977 |
| dev-023 | 0/1 | 1 | 0.210128/0.232990 |
| dev-024 | 0/2 | 2 | 0.204170/0.191268 |
| dev-025 | 0/1 | 1 | 0.209540/0.248956 |
| dev-026 | 1/0 | 2 | 0.229982/0.211467 |
| dev-027 | 0/1 | 1 | 0.227278/0.250675 |
| dev-028 | 0/1 | 1 | 0.203256/0.214061 |
| dev-029 | 0/1 | 1 | 0.216006/0.214707 |

Missing new behavior produces ordinary test errors in some bases; these are
retained as errors, not relabeled assertion failures or import failures. The
new-only probe and complete verifier both passed on their first control run.

### Retained failure and audit repair

The initial standalone unit suite ran all 57 tests and failed one new audit.
The audit treated only ordinary Python assignments as existing module names,
so it mistakenly considered the base module's annotated `COMMANDS` assignment
a new interface omitted from dev-026's statement. The repair recognizes both
`ast.Assign` and `ast.AnnAssign`, as the existing evaluator audit already does.
No name-check assertion was removed; no task dictionary, selection, hidden
byte or threshold changed. The final scoped gate reran all 57 tests and passed.
The failed unit log and before/after source hashes are retained privately.

An initial scratch helper filename also shadowed Python's standard-library
`select` import and failed before any task materialization. Renaming the helper
resolved that preparation error; it was not an evaluator or model attempt.

### Bindings and reviewer tooling

The raw reports, logs, definition audit and source snapshots remain private;
the handoff locates them. Publication retains only sanitized measurements and
digests. The already installed pinned Node runtime was copied unchanged into
an ignored build directory to make Markdown lint available through the shared
checkout when the reviewer mounts it. No dependency was downloaded or committed.
The request gives its version, checksum and exact repository-relative PATH.

| Tested source | SHA-256 |
| --- | --- |
| `scripts/coding_eval.py` | `b4898338d065127470ae822509ce225680106c335d0db16876985dc0b47c7edd` |
| `scripts/coding_eval_collection.py` | `ec29234fe7e8af22a974e6d5534118d050b73594db96f710841b8a191918941b` |
| `scripts/coding_eval_rust.py` | `735390c9e14d151c3a3b24645458f1cb307fa120ba23071fda81a69ebd0a9fda` |
| `tests/test_coding_eval.py` | `7111513acde8f87e3f47ebc71c11274f7c87444b73e607d9a990d00607461487` |
| `tests/test_coding_eval_collection.py` | `7f3a97d70ca4d8cfa30a02280ad00012cfa514623dc0cbf9a21d60caef52dad7` |
| `tests/test_coding_eval_rust.py` | `30581a8509c32ff0fccc76f1385b60b8159295c4c25110c61d4ce815eef175e3` |

| Private artifact | SHA-256 |
| --- | --- |
| `probe/summary.json` | `026df5a39e6b669c0bd3a8d7ec7b1ced28f4a8e687f67589a7937834a209aff2` |
| `unit.log` | `8b4766035d3de309eac3a315cc7e41a23ead81d430bb3946d64781c32d1e1371` |
| `verify/summary.json` | `9b6717da0d726f1ce167530671e0d3455e3156469ecdc9b17f4489a1b20a7887` |
| `verification-audit.json` | `d48273fda0da4a1c67ed9c48a6370d8f204e27eba48904012de5b3e83c243232` |
| `fast-gate.log` | `40adef7e083d91d381a870b3e67287a44be17cce0b1d3eafe645dd1617f01287` |
| `source-before-checks.json` | `1e3cbec5561c500997b1bf281ea1dc595c74a1b7b58cafb69fac811913bdb673` |
| `source-after-audit-fix.json` | `ebad05ec7e797df34720796e5843d6d42b5e51e56a2688c62912a49eefa8cdb5` |
| `definition-check.json` | `8c20be0a3b581aae622698e23748c594afbddb739fd2dd98b530efef0933c979` |
| `documentation.log` | `ccd8f5539377be1116844aac7248deb1338597de0c11c29f129a05928c69c339` |

After the measured source checks, only evidence and queue prose is finalized.
Final Markdown, whitespace, tested-source binding and publication hygiene
checks run before the commit. All owned check jobs exited and released their
reservations. No new evaluator workspace remains; the same four historical
leftover directories from the incoming handoff are untouched. No supply-chain cascade, long acceptance gate
or native host qualification is run for this batch.

## Remaining boundaries

Thirty development tasks meet a collection count, not the whole T-311 goal.
Most work is Python tooling; broad application, multi-file, refactor and test-add
coverage is still limited. Separate protected held-out inputs, independent
generated-result checking, aggregate worker quotas, required reporting labels
and a real current-harness baseline remain open. Generated candidates are still
refused before output creation and execution. No first-attempt/retry/token/GPU
score, model qualification or held-out result exists for this increment.

The old ledger/status model, product prompts and runtime, API, permissions,
source bindings, supply-chain outputs and long acceptance gate remain unchanged.
No full native acceptance, release, licence-import readiness or owner-test
readiness is claimed. Only independent exact-commit PASS can close this row.

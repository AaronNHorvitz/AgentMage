# T-311.5: twenty frozen development tasks

Date: 2026-11-03. Tier: core. Builder evidence; independent review required.
Base: `4c0460f6`, retaining the unchanged independent T-311.4 PASS. Parent
T-311 remains open. No model, GPU, held-out selection or agent tuning occurred.

## Executable scope

`coding-development-v7` appends five distinct original historical Python fixes
to the same `list`, `prepare`, `score` and `verify` command. The collection has
20 development tasks, 77 reference tests: 19 Python tasks and the one accepted
Rust task. The new module is task data; admission, sandbox, Python driver,
selection validation, limits and cleanup are unchanged. No dependency, model,
external dataset, other product implementation or licence change is introduced.

Every one of the 15 independently accepted task dictionaries stays identical,
including statements, hidden inputs, source pins, selections and counts. New
tasks pin their own frozen bases, original reference source, complete hidden
test module, complete editing projection and exact method/count selection.
Their source/reference pins appear in the [guide](../guides/coding-evaluation.md)
and [task module](../../scripts/coding_eval_collection.py). The combined
fingerprint is
`cd5fc51018e53364df2f26eaf91ea29a46805e19a6e7e069370372bd98cb8e46`.
Per-case reports additionally bind the new task module's SHA-256; verifier
summaries bind the complete collection fingerprint.

The parent still refuses all generated snapshots before output creation and
worker launch. Only exact frozen base/reference snapshots can execute. Tests
exercise that refusal on the new tasks as well as task-dictionary mutations.
Neither a historical reference nor a builder control is a model attempt.

## Frozen selection and statement audit

Each task represents a distinct historical source change, not another assertion
from an existing task. Six newly selected complete historical methods run with
their original imports, helpers, fixtures, decorators and assertions intact.
No hidden module was edited, extracted or rewritten. Selection and statement
work preceded any model measurement; no model attempts exist for these tasks.

| Task | Change and interface required by the statement | Frozen regression coverage |
| --- | --- | --- |
| dev-015 | `case_passed` checks actual resource/network trial outcomes rather than trusting completion counters; preserve other predicates | One method: synthetic passing memory/network trials followed by nonzero swap/DNS refusal. The other network counters and trial-count mismatch are statement requirements beyond this method's assertions. |
| dev-016 | `OpenAIAdapter.tokenize_messages` uses the existing Unix transport with exact positional argument contract, fixed decoder, disabled prompt caching, result cache and explicit context-error count; `unix_http_request` gains the keyword-only allowed-status set | Two methods: success count, second-call cache, max tokens/top-k/seed/cache flag, and context-error count. No real model or transport; malformed/refused responses and the transport helper's status policy are not independently exercised by these two methods. |
| dev-017 | `make_container_readable` restores directory/file modes and `git_archive` invokes it after extraction | One method: synthetic tree with restrictive directory/data/executable modes. Symlink/special-file refusal and extraction integration are statement requirements beyond this method's assertions. |
| dev-018 | `capture` redacts checkout stdout; `validate_raw` refuses private home/checkout paths, retaining existing command/failure behavior and exact diagnostic | One method with synthetic log strings and a subprocess stub; checks refusal of checkout, home and alias paths, placeholder, exit zero and publishable redacted output. It does not run a compiler or cover all command-failure branches. |
| dev-019 | `extract_deb` sends two exact argument lists to the existing command helper, preserving permissions without archive ownership | One method checks order, every argument, list shape and working directory with a stub. It does not execute archive tools or install a package. |

The statements specify callable names, return/error behavior, exact diagnostics
or argument order where the original assertions require them. The selected
methods use no external evidence fixtures or local installations. Their complete
historical modules are imported, but unrelated methods are not counted as run.
The guide also discloses the accepted Rust corpus coverage gaps from T-311.4 L1;
its statement and hidden corpus remain untouched. Before generated scoring,
reports must label that Rust scope. This batch does not resolve that backlog item
or treat the selected historical assertions as complete statement verification.

Other prospective history was rejected during static inspection before any
model attempt: `006dfdc6`, `b7f93fff` and `549264a4` add symbols imported directly
by their hidden modules, preventing the unchanged base from reaching tests;
`af3024d4` needs an evidence report outside the scripts-only projection;
`765f3af6` merely relaxes an assertion bound that the old implementation already
satisfies. None was added and then dropped in response to a model score. No
hidden file or accepted selection was changed to admit these prospects.

## Measurements

Checks ran through the shared build reservation. The initial reservation wait
was about 23 minutes; command measurements below exclude waiting.

| Check | Observed result | Wall time |
| --- | --- | --- |
| Final scoped fast gate, base `4c0460f6` | PASS, eight paths/five commands, all 53 evaluator unit tests | 16.098027 s; inner units 14.464 s |
| Final full verifier, same one command | PASS, 20 failing bases/20 passing references | 38.224837 s |
| Documentation policy validator | PASS, 700 Markdown files and all policy invariants | 135.010447 s |

All 77 reference tests pass; both controls total 154 test invocations. No skip,
expected failure, unexpected success, truncation, infrastructure failure or
input refusal occurs in the final full verifier. Every base has cause
`test_failure`, every reference `none`, and every row is valid. All reports
have zero model attempts, null model scores/tokens and zero GPU minutes. These
are evaluator controls, not model scores or first-attempt model success.

| Added task | Base failures/errors | Reference tests passed | Base/reference wall seconds |
| --- | --- | --- | --- |
| dev-015 | 1/0 | 1 | 0.088666/0.087347 |
| dev-016 | 0/2 | 2 | 0.111682/0.111456 |
| dev-017 | 0/1 | 1 | 0.188176/0.203375 |
| dev-018 | 1/0 | 1 | 0.203365/0.213037 |
| dev-019 | 1/0 | 1 | 0.224865/0.231789 |

The new-only prospective control run also passed all five base/reference pairs
before full verification; its private summary and per-case reports are retained.
Missing historical helper behavior produces ordinary Python test errors in
some bases; those are recorded as errors, not fabricated assertion failures.

### Failures retained and corrections

The initial fast gate failed one of 53 unit tests: the verifier's mocked
continuation test still expected 30 invocations/14 remaining valid rows. The
collection now requires 40 invocations/19 remaining rows after the deliberate
first infrastructure failure. Both expectations increased. The test still
asserts that the first case fails, its diagnostic survives, every remaining
case runs, and no model score exists. No task's hidden test or passing count
changed. The final 53-test gate passes with these stronger expectations.

The initial full verifier also failed: all 20 bases failed as expected, but
`dev-003`'s unchanged historical reference had one error among its 23 tests.
Its process-stop assertion saw `ProcessLookupError` while reading `/proc` for a
process disappearing between observation and read. It remains a real failed
control result, cause `test_failure`, not an infrastructure relabel or a PASS.
The full original hidden module was not edited, skipped or filtered. Repeating
the entire verifier, into a separate output directory with the same source/task
pins and thresholds, passed all 20 pairs. This known historical test race must
remain visible when preparing generated scoring; the present guard still
prohibits generated candidates. No automatic retry hides failures in the runner.

### Retained bindings

The private handoff locates all raw logs, per-case reports, timing summaries and
both failed and passing runs. Only sanitized results and digests are committed.
The final reports' evaluator/module identities match these source bytes:

| Tested source | SHA-256 |
| --- | --- |
| `scripts/coding_eval.py` | `b0944e1f65b670a06ccefdd571d9a6fbe7af3ea52257e07b4fa3bdc6721e7cc8` |
| `scripts/coding_eval_collection.py` | `ef1938220354388ee639aa82477f91076a4da0d2a2438625505bde6dd412c82c` |
| `tests/test_coding_eval.py` | `aa49a37d8cf831fa01a4f56c9d8a329bef3c709f4329f869f4a4c287229d8f2b` |
| `tests/test_coding_eval_collection.py` | `b9678b0d41e54a4f8a1d243eb5a8ca980058057c0774db7180f382ffafa0a5fb` |
| `tests/test_coding_eval_rust.py` | `b84f9e143311ee15143dfd0b9568e76a9cc17549a1a899c10333dccf6d3998a0` |

| Private artifact | SHA-256 |
| --- | --- |
| `probe/summary.json` | `2f3502f8800a134eb7e8c8a28ebb9e8aeb9916a1be600b791b349beb9df4ad53` |
| `fast-gate.log` | `4e0c1ea50c87026173dc18ae4bf33907f3811e4550041fd15eb093cfadeb3f03` |
| `verify/summary.json` | `2508f5fb18fc3d1f53eba61dc9ceed5d68fda42812e1ab01a405898e658cc287` |
| `verify/dev-003-reference/stderr.log` | `ab8f32a24b642513a8bcef9f7d0756b37a2001fb318b1b7e32ad103bee3efa46` |
| `check-results.json` | `eace5122ae5bed24569ebffede85601c59482f4b1ca1e1e3e2ed6b37d89fe35d` |
| `final-fast-gate.log` | `ac9ccbec17deff822671f0e1a4ba7af3e3fdbd7643e0c288f4a32c95986891a1` |
| `verify-final/summary.json` | `25eb50c203c0458b50eb7bd91139e80d54f873e6e4ac47acf96781a14eabea50` |
| `final-check-results.json` | `e43ca2da315edfd801b6f39da657bcb5ace095d3122b281c39d6295c8000f156` |
| `documentation.log` | `a71d0066f95448dafac91022f7fde72088a463667bb0a1b990401301f1f57e49` |
| `source-before-checks.json` | `b2788e63ebb368d6377658a3e42348e1beb6191ee48cd2ea80a150f812dd53bb` |
| `source-final.json` | `b9919ae2be446294e56a37fc9bc67465587a6e29af698f43eb3235e4aa585fd3` |
| `definition-fingerprints.json` | `002e0f7e361632051134304ee447246d1a4da0e7ef22c2d3fab3083764763461` |

Both owned check jobs exited and released their reservations. No newly owned
worker or evaluator temporary tree remains. The same four historical evaluator
trees present before this session remain untouched; their cleanup backlog is
not resolved here. The private handoff records their identities. After the final
source checks, only this measurement text and the queue description changed;
final Markdown, whitespace, source-binding and publication-hygiene checks run
before the commit. The long acceptance gate and supply-chain cascade did not run.

## Limits and remaining work

These are public-history development controls, not blind or held-out tasks.
Hidden means withheld from the editing projection. The scripts-only projection
and single Rust contract task still give narrow language/work coverage. Reaching
30 varied tasks, a separately protected held-out set, independently checked
generated results, aggregate worker quotas and a real current-harness baseline
remain open. Per-process limits are not process-tree quotas. Generated scoring
remains disabled; there is no model quality, token, retry or GPU score.

Product prompts, runtime, API, permissions and tools are unchanged. The old
ledger/status model, source bindings, supply-chain outputs and long acceptance
gate are unchanged. No native full-suite, release, import-licence readiness or
owner-test readiness is claimed.

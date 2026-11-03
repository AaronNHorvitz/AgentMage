# T-311.3: Pre-measurement statement-contract repair

Date: 2026-11-03. Tier: core. Base: `c86571bb`. Independent re-review remains
open. This repairs the same T-311.3 row, attempt 2; no new row or closure.

## Finding and correction

The [independent review](../reviews/c86571bb85c8-local-coding-t311-3-development-collection.txt)
reproduced all 14 base failures, 14 reference passes and 70 reference tests, but
found medium M1: dev-014 did not specify the subprocess protocol its frozen
mocks require, contradicted the unavailable-revision exception, and omitted the
closure-call order and diagnostic position. The v4 assertion-audit claim was
too broad. The review is retained byte-for-byte; its SHA-256 is
`7007321000bd5588196620442a1c1fbaa055898a59f27fc7a917c53fd9bb0c62`.

Before any model measurement, `coding-development-v5` corrects that contract:

- `complete_source_closure` uses three `subprocess.run` calls on the valid path,
  in order: `git cat-file -t REV`, `git ls-tree -r -l -z --full-tree REV`, then
  `git cat-file --batch`. It reads byte stdout, accepts `commit\n` from the type
  probe and long-format inventory records with sizes, and passes sorted object
  IDs to the batch call. Working directory, check/capture flags and timeouts
  are explicit. Invalid earlier input prevents later calls.
- Git's `subprocess.CalledProcessError` propagates for an unavailable revision;
  `validate_current_source` converts it to a failure. Other closure refusals
  use ValueError or its existing subclass.
- Current validation computes the report revision closure before the local
  closure. Historical and non-equivalent rejections name their required words
  in the first failure.

The same correction addresses low L1: dev-004 names ValueError for its stated
refusals, and dev-011 declares TOOL as a string. Only these three statements
change. A new preservation regression compares every task dictionary against
v4, requiring every other field to remain identical. All hidden blobs, methods,
counts, source pins and thresholds are unchanged. The original three task
definitions and 38-test requirement remain byte-for-byte identical as data.

The v5 fingerprint is
`72442a4cf406abd83dc7684a5ef225474b55ee2cbc4cb6e5d34e6a840f0da618`;
v4 remains recorded in the [historical expansion evidence](t311-3-development-collection-2026-11-03.md).
This is a versioned pre-measurement correction from reviewer feedback, not
feedback from model attempts or held-out evaluation.

## Assertion audit and alternative controls

The builder reread all 32 selected methods in the eleven additions, their
helpers, setup/cleanup and module fixtures, including nested mocks and exact
exceptions, literals, subprocess protocols and order-sensitive assertions.
The existing automatic interface-name audit is only a name-presence check; it
cannot establish the exception or protocol contract. The manual audit below
records what the selected methods require beyond their inherited base APIs.

| Task / selected methods | Contract checked, including helpers |
| --- | --- |
| dev-004 / 9 | Documentation plan command ordering; unknown asset/missing-test ValueError; crate data gets all-target coverage; NUL-separated untracked whitespace checks preserve index and binary acceptance; review metadata/verdict/newline refusals preserve bytes; four real process-tree helpers check normal and signal exit status, stopped children/supervisor, released lock and absence of later checks. The statement now names ValueError explicitly. |
| dev-005 / 1 | `validate_raw` must return the exact private-path diagnostic; synthetic `capture` preserves zero return, one redacted output per command and checkout-relative suffix. Existing marker checks remain required. |
| dev-006 / 1 | Parser and argument builder cover grant/revoke/list, optional list scope, mandatory mutation scope, absolute grant path, identifier rejection as HarnessError and conflicting parser options as SystemExit. No new mocked function protocol. |
| dev-007 / 1 | Exact argument tails for remember/type/list/revoke/delete/source/object; citation/version/path, workspace, text/byte bounds and unrelated-option refusals as HarnessError; conflict/type choice failures as SystemExit. Existing parser namespaces are inherited. |
| dev-008 / 2 | Catalog mock expects Popen argv prefix, no scenario, exact tail and state root; uses existing reservation methods and zero wait/poll results. Sample manifest equals stored JSON; compact insertion-order seal with empty digest, integer version, sorted files, actual UTF-8 length/hash and 0600 mode; invalid target/version/date raises HarnessError. Sample prose has no required literal. |
| dev-009 / 1 | Real synthetic child diagnostic remains in redirected stderr and RuntimeError contains `exited 3`. The statement additionally preserves bounded output and successful behavior. |
| dev-010 / 1 | Four source graphs require production mod.rs and descendants to remain production, test child exemption, and AmbiguousReference for both default module paths. Existing owned helper converts Path results to relative names; no new literal or mock sequence. |
| dev-011 / 2 | Synthetic Git fixture uses TOOL as a JSON key (now explicitly str). Strict later-commit declarations round-trip; altered declarations/sources/types fail. Private names in any case, absolute source keys and invalid result types refuse before destination creation. Existing ROOT and private_names mocks remain part of base interfaces. |
| dev-012 / 4 | AmbiguousReference for five cfg_attr/macro forms; production/test raw identifiers; explicit/include child resolution at lib.rs and engine.rs; default-loaded stem children; mixed-loader ambiguity only when children depend on the loader. No new diagnostic literal or mock call protocol. |
| dev-013 / 3 | lstat metadata stubs cover untrusted ownership, symlink/writable ancestors, bad leaf types/modes/size and unavailable path. No manager probe for untrusted systemctl. Process mocks check three-second timeout, DEVNULL stdout/stderr and exact three-entry environment; fixed readiness/status labels, no exception leakage. The statement specifies all these and stdin/check/argv. |
| dev-014 / 7 | Current report fixtures require exact closure/source types/keys and reject legacy/mutated guest data. Mocked closures are report revision then local revision; historical/non-equivalent words occur in failure zero. Synthetic Git checks equal trees and changed child/mode/docs, size-bound ValueError and unavailable CalledProcessError. Batch mocks provide exactly type/inventory/content results; duplicate inventory rejects before batch; truncation/header-size/trailing data raise ValueError. These previously omitted contracts are explicit in v5. |

Private alternative probes replace one helper or changed branch per new task,
while keeping the remainder of its historical reference and all frozen tests.
They use alternative review parsing, capture assembly, route-grant/memory/doc-pack
argument construction, child diagnostic handling, default-module path construction,
source-path validation, explicit-loader child resolution and executable-prerequisite
checks. For dev-014 both closure construction and current validation are replaced;
the alternate closure uses a different deterministic digest encoding because
only equal-tree stability and change discrimination are specified and tested.

These are builder-authored, reference-based diagnostic alternatives. The builder
has read both tests and references; they are not blind or independent solutions,
and do not prove that every statement-following implementation will pass. The
separate reviewer must assess completeness and can construct blind variants.
No alternative is submitted as an agent attempt. The private probe calls the
unchanged confined worker directly on these fixed trusted controls; production
`grade`/`score` admission is never patched, and generated scoring remains disabled.

## Verification

All checks use the shared build reservation. Unit and full verifier results
are fresh for the final evaluator/test source. Every verifier case report equals
its summary entry. All model-attempt counts are zero and model_score is null.

| Check | Observed result |
| --- | --- |
| Unit suite | PASS: 37 tests, no skips/failures/errors, 6.605 seconds (6.735884 with startup) |
| Full frozen verifier | PASS: 14 bases fail by test_failure; 14 references pass, 70 reference tests / 140 total invocations; 30.068435 seconds |
| Reference counts | No skips, expected failures or unexpected successes |
| Alternative contract probes | PASS: eleven controls, all 32 selected tests, 3.163549 seconds combined worker wall time |
| Scoped fast gate | PASS: seven paths/six commands, 8.388882 seconds; inner unit suite 37 tests in 6.550 seconds |
| Ordinary documentation validator | PASS: 698 Markdown files and all policy invariants; 148.288423 seconds |

The four negative dev-014 controls each execute all seven selected methods and
exit 1 with test_failure, with no skips, timeout or output-limit event. These
are intended failures, never model failures or attempts to repair candidates.

| Negative control | Failures / errors | Wall seconds |
| --- | --- | --- |
| Convert unavailable Git revision to ValueError | 0 / 1 | 0.298347 |
| Use rev-parse instead of specified type probe | 1 / 0 | 0.297272 |
| Read local closure before report closure | 1 / 0 | 0.298757 |
| Put historical diagnostic in the second failure | 1 / 0 | 0.297359 |

The matching alternate dev-014 passes all seven methods. This reproduces the
reviewer counterexamples and checks call order/diagnostic position directly;
the frozen tests are unchanged. No synthetic result is promoted to a score.

| Artifact | SHA-256 |
| --- | --- |
| Evaluator | `2131f2565df4100945b96129141cb2f1ce4b98c49bbd08db8a74fe0b48bb161c` |
| Evaluator unit tests | `7d0cdeb1be167e3daf512714af5fc48a1579c9a1dcdf2584a1debe46ff300290` |
| Verifier summary (private) | `f12c9fef691a591231fa11aece1dac62e43c65b1d32a9ab5f93b6aeb74ef41e7` |
| Unit log (private) | `d1738b254fb018f84ad0e03fbac71ac3cae6e9aed939cbd179500e3795a97689` |
| Fast-gate log (private) | `4aa0e18a6fa985c249aa79c5f6560ae975be99137a01e4bdac98734f93541950` |
| Policy-validator log (private) | `f9c9929f950c083eec2c0556ec8940f0fb4a2b91afbc197908b1593549ba7b6a` |
| Alternative probe driver (private) | `80210d3887b531e0c3f0177be97ced8880279f33a5342d21ddb02665ccfba6a3` |
| Alternative probe summary (private) | `9e43fd30478a5d09df1e5d3fd173b75c5be3a5135748724011cc2af0e7c43e4b` |
| Negative-control driver (private) | `a5ad5aa24f0a4c2dd8f6b23f157921e911ee51e0cf3594da44c130b2984053fa` |
| Negative-control summary (private) | `ad9c1f2242c170f04b00f0258031523313424cc9a64b96bb66df2b4d2b468fe5` |

Raw logs, complete alternate source bytes and result JSON remain private; the
review request gives fresh repeat commands and exact source identities. Final
measurement prose receives scoped Markdown, whitespace and added-text privacy
checks. The independent report remains byte-identical after retention.

## Boundaries

The full 30-task suite, varied language/work types, sealed held-out collection,
independent generated-result checking, aggregate worker quotas, current-harness
adapter and actual model baseline remain open. Fourteen development tasks are
not coding qualification. Parent T-311 stays open; T-311.3 needs exact-commit
independent PASS with no high/medium findings. No model/GPU run, download,
held-out access, harness/prompt tuning, product permission change, hidden-test
change, source-binding change, SBOM renewal, long gate or release occurred.

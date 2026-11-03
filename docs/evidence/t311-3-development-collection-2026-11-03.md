# T-311.3: Fourteen frozen development tasks

Date: 2026-11-03. Tier: core. Builder evidence; independent review remains open.
Base: `455c0a7a`, which retains the exact-commit independent PASS for T-311.2.
Parent T-311 remains open. These are evaluator controls, never model scores.

## Running scope

`python3 scripts/coding_eval.py verify --output <new-private-results>` now
validates 14 distinct historical source tasks in one command. Eleven additions
exercise original Python fixes/features from this repository. They cover process
supervision and input coverage, private-log redaction, closed catalog arguments,
synthetic document-pack sealing, failed-command diagnostics, Rust source
classification and committed-source evidence integrity. They introduce no
third-party dataset, dependency, model, licence change or product comparison.
The [guide](../guides/coding-evaluation.md) records exact base/fix commits;
source definitions pin all bytes and the complete regression selection.

This is still a narrow development collection: 14 tasks, not the required 30,
no held-out collection, no model adapter/attempt or coding baseline. It does not
complete T-311, qualify the current agent or establish import licence readiness.
The product harness, prompts and model selection are unchanged.

## Frozen selections and contract audit

All three original task dictionaries remain identical, including statements,
input/test pins and the 38-test threshold. They still load their entire original
test modules. Their canonical fingerprint remains
`d5f5e365924ffea8c6b46c50c28d00e116c01f69de57ac0a9672ffa50e05e3c5`.

The new collection revision is `coding-development-v4`, fingerprint
`4c4992a5c265aeebfe0f3bcbe81d65f293f7195dc3ec59d07ca5f9e9a8b6d311`.
Each addition pins a distinct historical source change and names the complete
original regression methods changed alongside it. These are newly defined
pre-measurement tasks, with their selections frozen on creation. Every hidden
test module remains the complete, unmodified historical blob. Nothing is
extracted or rewritten; original imports, helpers, setup/cleanup, assertions and
subtests all execute. No method was dropped from an accepted prospective task
after seeing its control result. No model run informed curation.

The driver permits only distinct `Class.test_method` names and an equal exact
integer expected count. There is no operator/CLI/environment selection override.
Names and counts are part of the task identity checked before candidate launch.
Malformed selections are input-integrity refusals. Missing methods and selected
failures/errors/skips cannot pass. Reports retain the selection and task digest.

| Added task | Statement and selected regression scope |
| --- | --- |
| dev-004 | Fast-gate unknown-input refusal, crate data coverage, untouched review bytes, untracked whitespace and supervised process cleanup; names `untracked_whitespace`, `review_text_lint` and existing execution interfaces. Nine original methods, including real process-tree/lock probes. |
| dev-005 | Capture redacts only the checkout; raw validation still detects private paths, preserves command count/return codes and uses the existing shared helpers. One original method with synthetic command results. |
| dev-006 | `route_grant_arguments` and parser forms for grant, revoke and list; workspace/identifier/path constraints and conflicting operations. One method with subtests. |
| dev-007 | `memory_arguments` and parser operations/options, emitted flag ordering and rejection of unrelated fields, malformed citations, private/control-bearing text and invalid identifiers. One method with subtests. |
| dev-008 | `doc_pack_arguments`, shared `catalog` routing without a scenario, root reservation and exact argument tails; `SAMPLE_PACK_FILES` and `write_sample_pack`, sorted file identities, private files and compact JSON sealing. Two original methods. |
| dev-009 | Failed `subprocess_runner` preserves the child's diagnostic and exit code; statement specifies the bounded tail. One original method launching a synthetic failing child. |
| dev-010 | `test_only_files` correctly resolves default mod.rs paths, production references, test-only children and ambiguous default paths. One method with four synthetic source graphs. |
| dev-011 | `retain`/`load_stage` schema-2 source equality, declared later commits, required `TOOL`, tampering/privacy/type refusals before output creation. Two methods using synthetic Git repositories. |
| dev-012 | Raw identifiers, explicit/include child resolution, mixed-loader ambiguity, macro and path-attribute uncertainty in `test_only_files`. Four original methods using synthetic source graphs. |
| dev-013 | `native_executable_prerequisite` and `confinement_diagnosis` report necessary trust prerequisites without claiming admission; exact subprocess environment/timeout and fixed errors. Three original methods with metadata/process stubs. |
| dev-014 | `complete_source_closure`, its bounds and strict batch parsing; `validate_current_source` rejects malformed, legacy or non-equivalent input while preserving historical inspection. Seven original methods, including real synthetic Git histories. No installed-worker/platform execution. |

Tests audit method existence in each pinned original module, unchanged legacy
fingerprints/whole-module loading and the complete 70-reference-test count.
The statement interface audit covers selected methods and their helpers.
Driver tests use the actual confined worker to establish selected success,
failure, skip and missing-method behavior, and unchanged whole-module loading.
A real CLI round trip prepares dev-006, observes the base fail and exact reference
pass, then rejects a comment-only generated edit before output creation. The
existing forged-count attack, interruption, cleanup, input-integrity and network/
read-only worker regressions remain in the unit suite.

## Measured results

All checks below ran through the shared build reservation. Full verification
completed with `passed: true`, `input_refused: false`, `model_score: null`:

| Check | Result |
| --- | --- |
| Evaluator unit suite | PASS; 36 tests, no failures/errors/skips, 6.635 seconds (6.747 seconds including interpreter startup) |
| Complete frozen verifier | PASS; 14 failing bases and 14 passing references, 29.965 seconds |
| Reference regressions | 70 tests; no failures/errors/skips/expected failures/unexpected successes |
| Combined base/reference invocations | 140; base failures below are the intended controls |
| Scoped fast gate | PASS; five selected paths, five commands, 8.441 seconds; unit rerun 36 tests in 6.668 seconds |
| Ordinary policy validator | PASS; 697 Markdown files/all policy invariants, 161.418 seconds |

| Task | Base tests | Base failures / errors | Reference passes | Base / reference wall seconds |
| --- | --- | --- | --- | --- |
| dev-001 | 12 | 0 / 3 | 12 | 0.318481 / 0.293908 |
| dev-002 | 3 | 0 / 19 | 3 | 0.254108 / 0.361895 |
| dev-003 | 23 | 2 / 3 | 23 | 2.328352 / 2.304933 |
| dev-004 | 9 | 11 / 2 | 9 | 16.455391 / 0.921393 |
| dev-005 | 1 | 3 / 0 | 1 | 0.228859 / 0.219233 |
| dev-006 | 1 | 0 / 1 | 1 | 0.213265 / 0.250733 |
| dev-007 | 1 | 0 / 1 | 1 | 0.241477 / 0.255912 |
| dev-008 | 2 | 0 / 2 | 2 | 0.234100 / 0.239858 |
| dev-009 | 1 | 1 / 0 | 1 | 0.205932 / 0.208155 |
| dev-010 | 1 | 1 / 0 | 1 | 0.218307 / 0.200177 |
| dev-011 | 2 | 0 / 2 | 2 | 0.251280 / 0.315622 |
| dev-012 | 4 | 10 / 0 | 4 | 0.192939 / 0.202858 |
| dev-013 | 3 | 0 / 8 | 3 | 0.229487 / 0.222752 |
| dev-014 | 7 | 2 / 25 | 7 | 0.276734 / 0.302320 |

Subtest failures/errors can exceed method counts. Every case has zero model
attempts and GPU minutes, unavailable tokens, a frozen-base/reference label and
`score_scope: trusted-evaluator-validation-only`. Base control failures are not
model failures. dev-004's old supervisor fails its real cleanup probes; the
outer worker namespace still owns and removes its descendants.

| Artifact | SHA-256 |
| --- | --- |
| Evaluator source | `ef99021773901aa098f5630e97e65829956b57d64c4d8d622b1fac6576939ac4` |
| Evaluator unit tests | `cf06bfdff5c09d5112c4cffa76b933ee2c5de6a0384e11a8c52505b3e4cbac86` |
| Verifier summary (private) | `d555643be9677d978dc49f10e3e59f785fc152f17182cc8c24cd7b297dd1b6b4` |
| Unit log (private) | `bf46204df45c06750a9071dc8d3cd9e9b23ab330fda929f3c98e006f9de66513` |
| Scoped fast-gate log (private) | `8733010a630c43fa81374c5511e47455901babedd793279ee5fc8cd4c534beea` |
| Policy-validator log (private) | `d52905e459edbc4ea70a26fab4eec2783c6a5a3c00f69b1a71643e8520b5fc8d` |

Raw logs and reports remain in private lane state; the review request names exact
repeat commands using fresh destinations. Public records contain only sanitized
measurements and digests. Final evidence prose gets scoped Markdown/whitespace
and privacy checks after these measurements are recorded.

The final pre-publication assertion audit added four required diagnostic
substrings to dev-014's statement: historical, not equivalent, exceeds bound
and inventory invalid. This changed no test bytes, selections, counts or
thresholds. The earlier unit/verifier run (36 tests; 14 valid controls;
30.114-second verifier) is retained privately but superseded for source identity
by the complete final rerun above. No model attempt preceded either run.

## Curation failures retained

The initial private prospective-task run accepted eight candidates but rejected
three whose references failed in this projection: recipe samples needed absent
sample files; extension operations included a sample-copy regression with the
same problem; suspend/resume included a changed acceptance helper outside the
editable source. The metadata driver then stopped on a newly added module that
had no base blob. This was a curation error, not a test pass or model outcome.
The remaining prospective run accepted three more and rejected a shared-redactor
addition whose base module could not import new interfaces, producing no valid
base counts. That base infrastructure failure cannot satisfy a regression task.
All initial logs/results and the failed driver are retained privately. Rejected
candidates were excluded in full, with no hidden-test edits or narrowed retries.

## Boundaries and remaining work

Historical-controls-only admission from T-311.2 remains mandatory on every
production scoring entry. Independent checking of generated results and an
aggregate worker resource quota remain unresolved and block generated scoring.
Cgroup controls are read-only in this lane; no bypass or weakened trust check is
introduced. Scope, input snapshots, read-only mounts, offline worker, process
cleanup and resource/output/deadline limits are unchanged. An exact historical
reference is always a control, even if submitted via `score`.

At least 16 further suitable tasks, broader language/work types, a separately
sealed held-out collection, independently checked and aggregate-bounded generated
scoring, current-harness adapter and actual local-model baseline remain open.
The inherited native-host failures and T-301.1 stay deferred, unchanged and
unrelabelled. No GPU/model work, held-out exposure, prompt tuning, SBOM/evidence
cascade, status-model/old-ledger change, long acceptance gate or release occurred.

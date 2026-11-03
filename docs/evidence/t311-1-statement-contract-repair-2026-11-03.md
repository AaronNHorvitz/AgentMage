# T-311.1 statement contract correction — 2026-11-03

Status: builder correction for third independent core review; T-311.1 and parent
T-311 remain open. This batch measures the evaluator, with no model attempts,
held-out work, harness tuning or coding-score claim.

## Independent findings and correction

The unchanged [second review](../reviews/b1734b9957ce-local-coding-t311-1-evaluator-repair.txt)
for exact commit `b1734b9957cebfbfbe8a2ebefa33322f3f5982eb` returned
**Verdict: FINDINGS**. It independently confirmed the maintenance-race repair
(M1), worker boundary assertions (L3), repeated verifier passes and retained
measurements. Residual medium M2-R remains its verdict; the builder cannot
approve or close this row. The retained report SHA-256 is
`4a823c7f6e69a31f5794ecb7fde4494289f6c1ff880f6d71012de8c057e524de`.

`coding-development-pilot-v3` corrects only two statements:

- dev-002 now requires `diagnose()` output `schema_version` to change from 1 to
  2, while the persisted run record stays at version 2 and gains no model fields.
  The v2 instruction to preserve version 2 incorrectly described the base
  diagnosis, which actually emitted version 1.
- dev-003 explicitly requires rejection of `Verdict: FAIL` and
  `Verdict: BLOCKED`. Both were accepted by the base; asking only to reject
  malformed endings failed to communicate their removal.

No original statement-prefix preservation is claimed for v3: the contradictory
sentence is replaced. The original v1 and v2 definitions remain in Git history.
All non-statement task fields, source/projection pins, hidden tests and expected
counts are identical to v2. A regression compares every non-statement field
against the exact v2 source and restricts the changed task set to dev-002/003.
The full definition fingerprint is
`d5f5e365924ffea8c6b46c50c28d00e116c01f69de57ac0a9672ffa50e05e3c5`.
No model has run against any pilot statement revision.

## Manual assertion audit

This is the reviewer's manual-audit alternative to a general literal-diff
checker. The existing module-attribute AST check remains deliberately limited.
The builder read all three frozen hidden modules, their pinned base sources
and the base-to-reference source changes. Each row below accounts for the
assertions within one test method, including subtests and mock callbacks.
Names omit the `test_` prefix. Line numbers refer to the frozen hidden module,
not the current checkout. Inherited means the base already implements the
assertions; the task must preserve that behavior, without an additional change.
These public development tests are unsuitable as unseen held-out tasks.

### dev-001: 12 methods

| Frozen method / line | Assertions and statement basis |
| --- | --- |
| untracked_source_inventory_binds_new_bytes_and_ignores_build_outputs / 15 | New inventory returns the untracked relative path and exact byte count, ignores build outputs, and changes the digest when same-length bytes change. Stated Git enumeration, row schema and byte identity. |
| untracked_source_inventory_refuses_aliases_special_files_and_large_inputs / 34 | Symlink, FIFO substituted after enumeration, and 16 MiB + 1 file raise the specified alias, nonregular and limit errors. All three codes and enumeration via subprocess.run are explicit. |
| read_rejection_requires_exact_bytes_no_authority_and_terminal_denial / 59 | Inherited acceptance helper checks exact rejected arguments, no permission/effect, terminal denial and bounded tools. Helper projection stays byte-identical. |
| multifile_regression_requires_nine_tools_and_recording_beyond_old_limit / 101 | Inherited acceptance helper requires nine tools and 35 retained artifacts, rejects missing artifacts and six tools. Helper projection stays byte-identical. |
| native_command_failure_artifact_must_belong_to_exact_command_turn / 128 | Inherited acceptance helper requires unique case roots, exact failed-command turn and artifact digest; rejects changed turn/digest. Helper projection stays byte-identical. |
| acceptance_retains_prelaunch_failure_without_outcome / 170 | Inherited failure record cannot pass without exit/terminal evidence; zero observed events. Helper projection stays byte-identical. |
| implementation_identity_pins_source_and_all_three_binaries / 186 | Existing 40-character HEAD, three nonempty SHA-256 binary identities, tracked-diff digest and wrapper path remain. New inventory uses relative paths and 64-character hashes; aggregate uses the exact stated canonical JSON encoding. ASCII and UTF-8 produce identical bytes with ensure_ascii=True. |
| setup_creates_one_private_clean_path_bound_fixture / 200 | Inherited activation, clean branch, scripted qualification, private directory/marker modes and second-setup refusal. Preserve authority checks. |
| pid_reuse_and_substitution_cannot_receive_stop_signal / 216 | Inherited refusal of a substituted record; preserve authority checks. |
| setup_builds_exact_new_and_multi_file_fixtures / 220 | Inherited absent/new and present/multiple file layouts; initial fixture validation fails. |
| gpu_observation_requires_one_exact_numeric_device / 238 | Inherited exact numeric device pair; absent, multiple or nonnumeric observations raise. Test mocks the observation; no GPU is used. |
| candidate_guard_cancels_only_its_run_on_sampled_pressure / 248 | Inherited sampled-pressure refusal, existing error code and peak observation; no real GPU is acquired. |

The unchanged statement also requires sorting, count/aggregate limits and
concurrent-change refusal. The frozen tests do not fully exercise those claims;
this audit adds no assertion or threshold and makes no coverage claim for them.

### dev-002: 3 methods

| Frozen method / line | Assertions and statement basis |
| --- | --- |
| only_one_closed_model_argument_is_reported_without_serving_claims / 18 | For every MODEL_CHOICES member, exact four-key result: supported selection, owned-process-arguments, not-observed serving, not-assessed qualification. All names and literals are explicit. |
| missing_conflicting_unknown_and_private_arguments_are_not_inferred_or_echoed / 29 | Missing, empty, uppercase, non-ASCII, unknown/private, equals syntax and repeated flags yield None/unavailable and the same serving/qualification literals; private content is absent. All refusal classes are explicit. |
| owned_non_model_process_reports_request_and_preserves_legacy_record_lifecycle / 47 | Across six argument variants, persisted record version 2 has neither model key and stays byte-identical. Reserved has no selection; exact owned child is recognized; starting diagnosis now has version 2, scope and fixture_profile_id, without old profile_id/top-level qualification or private text. Running preserves request observation; changed process identity and dead child become stale-record with no selection; closed reservation becomes ready. Version migration, exact output contract, non-echo and lifecycle preservation are explicit. |

The lifecycle test also asserts child readiness and mocks GPU acquisition to
raise: readiness establishes the synthetic non-model child before observation;
existing `diagnose()` does not acquire resources. No coding model is involved.
`CandidateResourceGuard.acquire` and `RunRecordReservation.identify/close` already
exist in the base class definitions; no new nested class method is required.
The string mock target `sys.stdout` is standard runtime output capture, not a
missing candidate interface. All `mock.patch.object` targets in these tests
exist in the base except the new interfaces explicitly named in the statement.

### dev-003: 23 methods

| Frozen method / line | Assertions and statement basis |
| --- | --- |
| changes_include_committed_staged_unstaged_untracked_and_deleted / 44 | Inherited exact change set including unusual filename, excluding ignored output. |
| base_cannot_be_a_git_option / 67 | Inherited option refusal. |
| reverse_dependencies_are_included_and_unrelated_crate_is_not / 72 | Inherited dependency selection in both directions tested. |
| workspace_configuration_requires_every_crate / 78 | Inherited workspace-wide selection for each configuration input. |
| rust_plan_runs_all_targets_without_filters_or_ignores / 84 | Inherited exact all-target test command and denied-warning lint flags. |
| document_only_change_has_no_build_or_evidence_generator / 92 | Inherited exact document-check plan and selected path. |
| unknown_change_and_missing_tests_fail_closed / 98 | Inherited refusal of every unknown path/missing test mapping. |
| crate_data_inputs_keep_full_rust_coverage / 105 | Inherited full-target coverage for crate data and extensionless licence input. |
| untracked_whitespace_uses_git_rules_without_staging / 114 | Inherited unchanged index and whitespace refusal for both filenames. |
| text_review_has_its_own_format_check_without_editing_it / 128 | Inherited dedicated plan, byte preservation and malformed/unsupported report refusal. |
| script_and_test_changes_select_one_test_module / 146 | Inherited exact test-module plan. |
| review_verdicts_match_independent_reviewer_format / 151 | PASS/FINDINGS and INCONCLUSIVE plus one reason preserve bytes; reject FAIL, BLOCKED, missing/blank reason and extra reason lines. Every accepted/rejected ending class is now stated, including the two removed base values. |
| cleanup_keeps_exited_leader_until_all_group_signals_are_sent / 166 | New observe_exit returns 7 or negative SIGTERM, leaves the leader waitable and returncode unset during both group signals, sends SIGTERM then SIGKILL and preserves the final status. Stated nonreaping observation and original status. |
| python_lint_checks_syntax_indentation_and_whitespace / 191 | Inherited invalid Python/whitespace refusal and acceptance of valid input. |
| failure_is_retained_even_when_later_check_passes / 199 | Inherited nonzero result, reported exit 7 and FAIL despite later success. |
| execution_limits_build_and_test_concurrency / 208 | Inherited successful command and fixed 2/4 build/test allocation. |
| missing_executable_cannot_pass / 214 | Inherited missing-command failure. |
| timeout_fails_and_terminates_owned_children / 218 | Preserve timeout failure and descendant stopped or zombie state. |
| normal_exit_cleans_descendants_and_releases_reservation / 317 | Preserve exit 0, stop descendants and release lock. Shared helper assertions check stopped state and reservation acquisition. |
| termination_cleans_descendants_and_starts_no_next_check / 322 | Preserve failure on TERM/HUP/INT, stop descendants/supervisor, release lock and prevent next check. |
| sigkill_of_gate_group_cleans_descendants_and_releases_reservation / 334 | Preserve gate signal status, descendant/supervisor stop and lock release. |
| supervisor_termination_cleans_check_group / 340 | Preserve gate failure, descendant/supervisor stop and lock release. |
| empty_change_cannot_claim_green / 346 | Inherited empty-change failure. |

Nested `gate.os.killpg` is the existing standard-library import with its existing
method. The mock checks leader ownership during cleanup; it imposes no new name
beyond `observe_exit`. Other object mocks target existing change discovery and
standard runtime argv. In dev-001, nested `hashlib` and `subprocess` members,
including the mocked enumeration, also exist in the base. No `mock.patch` string
names a new scripts interface in any of the three modules. No new nested method
or hidden output literal remains unexplained by the statement or base behavior
in this manual audit. Independent review must still assess its completeness.

## Verification

Reserved unit suite: **PASS**, 23 tests, zero failures/errors/skips, 1.639 seconds.
Full documented verifier: **PASS**, three bases fail by test_failure and all
three historical references pass, 38 reference tests / 76 test invocations,
zero skips or expected failures. Every report matches its summary entry,
and retained log hashes were checked against the actual bytes.

| Task | Candidate | Tests | Failures / errors | Wall seconds |
| --- | --- | ---: | --- | ---: |
| dev-001 | base | 12 | 0 / 3 | 0.316525 |
| dev-001 | reference | 12 | 0 / 0 | 0.322265 |
| dev-002 | base | 3 | 0 / 19 | 0.252000 |
| dev-002 | reference | 3 | 0 / 0 | 0.344033 |
| dev-003 | base | 23 | 2 / 3 | 2.252605 |
| dev-003 | reference | 23 | 0 / 0 | 2.354949 |

No product implementation or Rust crate changed. The unchanged native baseline
and its deferred owner-action blocker are not retested or claimed green.

| Artifact | SHA-256 |
| --- | --- |
| Evaluator | `b1fbab8684e2a5e6d97c6450360ea64b91502bf17156ed6e64f3d04c777071d1` |
| Evaluator tests | `c49524b8277395d7ed767cdbb4f78aae05c18dbaebffb4236c9fd9c84bdf99a9` |
| Verifier summary (private) | `7e74fbe2efd7841e4475715dcf94975963d7d07f29ef2cdf5da876f35c4bb975` |

| Case | Raw stderr SHA-256 |
| --- | --- |
| dev-001 base | `b0832f48d72516f5767042ebcb62df7c0da194751abc7164134d8c776cecb1c0` |
| dev-001 reference | `940a3ec30c53e3d1599a8a58bab7bdf06fea5968cc3d86236da7423ebd63de83` |
| dev-002 base | `c01fdaa2e3dafd2e18115f2373bce93fd98fadba397260f49366bee86527bb02` |
| dev-002 reference | `fdd2946443ac7609610acf2e4d9ace26f918e77596a87ad1d4ecae42e53e09d6` |
| dev-003 base | `b20e3f715b777012e6fe9aa9df3c112a78758af5a7b3d85f4138313694074c5e` |
| dev-003 reference | `8d1a494089f64335308aeadd76d0b29b2f695acc447763d906f64e805e94609b` |

Both reviewer counterexamples were reproduced through the actual `prepare`
and `score` CLI and confined worker. Each starts from the unchanged historical
reference, restoring exactly the disputed base behavior. Each exits 1 with
test_failure; the full references above pass. These are deliberate negative
controls, not candidate repair attempts or model scores.

| Counterexample | Tests | Failures / errors | Wall seconds |
| --- | ---: | --- | ---: |
| dev-002 diagnosis version stays 1 | 3 | 6 / 0 | 0.305531 |
| dev-003 keeps accepting FAIL/BLOCKED | 23 | 2 / 0 | 2.396401 |

Scoped fast gate: **PASS**, all six commands including Markdown lint, retained
review validation, Python lint and 23 tests (1.627 seconds). Reserved ordinary
documentation validation: **PASS**, 695 Markdown files and all policy invariants.
Final measurement prose passes scoped Markdown lint and whitespace checks.
These are the fast/ordinary checks, not the deferred long acceptance gate.

| Artifact | SHA-256 |
| --- | --- |
| Counterexample summary (private) | `74cc09caa05f1e8a8bf50ad7d415d3432ad787f24a33a2886a0a968d636ebbba` |
| Counterexample driver (private) | `d3c2c6058ba73a79ffee2e6f022830cb45d1289fb6e10289ec4ea0f256a27b5e` |
| Fast gate raw log (private) | `1f4d683645273e5c9e04a927fd8b90ab8b648a453135cae290a876704b52ea08` |
| dev-002 counterexample raw stderr | `1c74fa2f62f5da103e1f965661b9a008a3e0b442d82833451173a65c76f08f16` |
| dev-003 counterexample raw stderr | `4eba79ad555981a4a113fd1b21a0090b31256983c5381e263b8f0325788733d5` |

## Remaining boundaries

Low findings L1 (result forgery) and L2 (aggregate resource quotas) remain
prerequisites before any generated-candidate scoring. L4 (input-integrity refusal
classified as infrastructure/exit 1) and L5 (stale counts/temporary artifacts after
late infrastructure failure) remain backlog from the second review. No such
failure is relabeled a PASS. Only trusted frozen historical candidate bytes and
the two explicit reviewer counterexamples are scored here.

The complete 30-task suite, sealed held-out collection, current-harness coding
baseline and model-quality measurements remain open. No GPU reservation, model
inference/download, product permission change, hidden-test change, long gate,
supply-chain renewal, release or owner-test readiness is claimed. If review 3
is not PASS, apply the three-strike rule without splitting this repair to evade
review, preserving the original row and the remaining findings.

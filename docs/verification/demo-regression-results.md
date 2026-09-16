# Linux demo regression and security checks

Run on the actual Fedora machine on 2026-09-15 America/Chicago
(2026-09-16 UTC). These are demo-supporting regression results, not production
release qualification. Real inference and browser workflow evidence are recorded
separately; these tests do not substitute for that acceptance run.

## Rust regression tests

Command:

```sh
cargo test -p agentmage-host -p agentmage-kernel-engine -p agentmage-capability-knowledge -p agentmage-capability-read-only
```

Exit status **0**: **1,710 passed, 0 failed, 15 intentionally ignored**.

| Target | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Knowledge library | 284 | 0 | 0 |
| Knowledge coding-corpus integration | 3 | 0 | 0 |
| Read-only library | 26 | 0 | 0 |
| Host library, including five document-demo regressions | 282 | 0 | 8 |
| Host package/bootstrap binary | 15 | 0 | 0 |
| Kernel engine library | 1,030 | 0 | 7 |
| Kernel integration targets combined | 67 | 0 | 0 |
| Kernel compile-fail documentation tests | 3 | 0 | 0 |

All remaining selected binary/documentation targets contained zero tests.
The host library took 8.23 seconds, package/bootstrap binary 5.41 seconds, and
kernel library 53.41 seconds. These are regression durations, not inference latency.

The five document-demo regressions cover immutable source snapshots and citation
membership; unknown evidence and bounded summaries; symlink file/directory escapes,
traversal, hidden/non-UTF8/unsupported inputs; inventory limits and visible context
omissions; file/total/depth/root-path limits; and exact UTF-8/CRLF source-range
extraction. Multiple related assertions share a test.

The previously reported native/managed manifest failure did not reproduce. All 15
host package/bootstrap tests passed, including native bootstrap tests. This does not
qualify an installed production package or reopen production gates.

Ignored host tests retain their existing conditions:

- `story_22_1_native_tool_terminal_resume_child`: subprocess helper.
- `story_22_1_native_tool_terminal_resume_matrix_never_replays_or_invents_state`: explicit 100-resume campaign.
- `story_39_1_native_write_checkpoint_process_child`: subprocess helper.
- `story_48_2_linux_git_adapter_runs_the_approved_plan_without_a_shell`: supported systemd user session and Bubblewrap lane.
- `approved_read_is_receipted_replay_safe_and_restart_verifiable`: systemd user session and Bubblewrap lane.
- `every_generic_tool_worker_returns_verified_result_one_receipt_and_no_workspace_mutation`: installed root-owned worker lane.
- `lifecycle_matrix_retains_one_terminal_receipt_and_never_reports_false_completion`: installed lifecycle fixture lane.
- `stale_and_cancelled_previews_start_no_worker_and_publish_no_receipt`: installed worker manifest lane.

Ignored kernel tests include six subprocess helpers and the explicit reference-hardware
load campaign. Their existing parent crash/restart matrices ran where enabled; ignored
entries themselves are not represented as independently executed acceptance tests.

## Build and strict lint

```sh
cargo clippy -p agentmage-host -p agentmage-kernel-engine -p agentmage-capability-knowledge -p agentmage-capability-read-only --all-targets -- -D warnings
npm run product:build
git diff --check
```

All three commands exited **0**. Product build includes
`cargo build --workspace --all-targets --locked` and the VS Code shell TypeScript build.
`npm run product:lint` also passed workspace Clippy with warnings denied and VS Code
ESLint before its strict-local baseline check failed as described below.

AST syntax checks passed for `scripts/demo.py`, `scripts/demo_smoke.py`, and
`scripts/demo_offline_smoke.py` without starting an additional application.

## Security checks

Live requests to the running demo's `/api/status`, without printing or persisting its
bearer token, produced:

| Request | Observed HTTP status |
| --- | ---: |
| Missing local access token | 403 |
| Valid token, wrong Origin | 403 |
| Valid token, wrong Host | 403 |
| Valid token, authorized loopback Origin and Host | 200 |

The probe read only the application's own launch-state file and used Python
`http.client.HTTPConnection` to its loopback port. It made no privileged changes.
Code inspection confirmed exact Host checks, Origin checks, constant-time token
comparison, bounded JSON requests, restrictive CSP, no-store responses, and DOM text
insertion for model/source content. Model output is parsed as JSON; it never supplies
shell commands. Document admission uses the Rust host's fd-anchored `openat2`
`BENEATH`/`NO_SYMLINKS`/`NO_MAGICLINKS` path and regular-file checks.

The model launcher uses an authenticated Unix socket within an application-owned
0700 state directory, artifact/runtime hashes, `bwrap --unshare-net`, and the runtime's
offline flag. Those source checks support, but do not replace, the separately recorded
real-model namespace/offline acceptance test. Storage is not represented as encrypted.

```sh
npm run effect-boundary:check
```

Exited **0**: structural effect mediation boundary validated.

## Initial failures and pending follow-up

- Initial `npm run product:check` exited **1** during format-check: rustfmt wanted
  the `demo_documents` module export before `desktop_experience` in `shells/host/src/lib.rs`.
- Initial `npm run product:lint` exited **1** at strict-local-source check because
  `security/strict-local-source-policy.json` still bound the pre-demo
  `shells/host/Cargo.toml` digest. The only manifest change is the new demo binary registration.
- Initial `npm run hostile-network:check` exited **1** because that same strict-local
  baseline had one stale-manifest finding. It did not report an undeclared network path.
- A read-only diagnostic replaced only that digest **in memory** and reran the audit;
  it returned zero findings. This is diagnostic evidence, not a passing on-disk gate.
  The authorized reviewer must retain the binding and record the reviewed updated digest.
- `npm run product:test` exited **101**: the workspace run stopped at the unrelated
  repository-map scaffold library with **43 passed and 5 failed**. Every failure was
  `LicenseMismatch` in its baseline fixture. That fixture reads the repository's
  preserved Business Source License, while package-scaffold admission intentionally
  pins an Apache-2 license artifact. The failing tests were
  `collisions_stale_conventions_license_changes_and_present_roots_fail_closed`,
  `every_approved_language_emits_license_tests_docs_config_source_and_commands`,
  `every_scaffold_composes_an_exact_inert_atomic_root_application`,
  `plan_and_file_mutations_never_verify`, and `scaffold_application_mutations_never_verify`.
  No source or license check was changed to hide these failures. A possible later
  correction is a dedicated Apache scaffold fixture, if that scaffold policy remains
  intended. The repository Business Source License must stay intact.
  The workspace command did not reach its later Rust targets or VS Code test phase;
  the selected host/kernel/knowledge/read-only run above completed independently.
  A separate `npm run test --workspace @agentmage/vscode-shell` run then passed
  **95 tests, 0 failed, 0 skipped** (84.814 ms reported test-runner duration).

## Verified correction of initial check failures

After the owner-delegated integration work formatted `lib.rs` and reviewed/renewed
the host manifest's exact digest in the strict-local policy, these commands were
rerun against the on-disk working tree:

```sh
npm run product:format-check
npm run strict-local-source:check
npm run hostile-network:check
npm run product:lint
```

All exited **0**. Cargo formatting and VS Code Prettier passed, strict-local audit
reported zero undeclared network paths, and the hostile-network gate blocked all
**8 cases before execution**. The manifest binding was retained. The initial failures
above remain recorded as investigated failures, not current blockers for those checks.
The complete product-lint rerun passed workspace Clippy with warnings denied, VS Code
ESLint, strict-local audit, hostile-network injection, and effect-boundary checks.

The broad `product:check` umbrella is not represented as passing because its
`product:test` phase retains the five scaffold fixture failures described above.

Raw development logs are in `/tmp/agentmage-demo-cargo-regression.log`,
`/tmp/agentmage-demo-clippy.log`, and `/tmp/agentmage-demo-product-*.log`; they are
temporary local logs, not required release evidence inputs.

## Final delegated-governance regressions

After the final source batch, real acceptance and single SBOM renewal:

```sh
python3 -m unittest tests.test_architecture_decision tests.test_planning_scope tests.test_requirement_registry tests.test_requirement_coverage tests.test_status_model tests.test_schema_evolution_plan tests.test_remaining_plan_blocker_audit
python3 scripts/status_model.py
python3 scripts/demo_evidence_check.py
```

All exited **0**. The focused suite ran **97 tests in 1.302 seconds**. It validates
unique accepted decision identities, the explicit delegated normative snapshot,
registry/coverage/planning integrity, scoped native demo promotion, retained schema
migration bindings and accurate blockers. The acceptance checker independently
verified all four current whole-file-bound real demo reports after the SBOM renewal.
The temporary raw focused-test log is `/tmp/agentmage-demo-final-governance-tests.log`.
A separate `python3 -m unittest tests.test_context_safety_registration` run passed
all **5 tests**, and `npm run task-graph:check` exited **0**.

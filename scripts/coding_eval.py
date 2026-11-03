#!/usr/bin/env python3
"""Score frozen repository development tasks without changing the agent harness.

Original evaluation tooling. Historical inputs remain pinned and are never edited.
This development collection is not the complete T-311 suite or a model score.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import re
import selectors
import shutil
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time

if __package__:
    from . import coding_eval_collection, coding_eval_rust
else:
    import coding_eval_collection
    import coding_eval_rust

ROOT = Path(__file__).resolve().parents[1]
VERSION = "coding-development-v7"
MAX_FILE = 1024 * 1024
MAX_TREE = 16 * MAX_FILE
TIMEOUT = 90
# Pre-measurement v3 corrects residual statement contradictions under review M2-R.
# Test/input pins remain unchanged. Future expansion adds tasks; never rewrite pins.
TASKS = [{'id': 'dev-001',
  'split': 'development',
  'base': 'aa3134b224e887fbf5962fbf724b8d3d54cbe8ec',
  'reference': '14e33a4565e34663fd84fe0939b3ccc5bc2e633d',
  'hidden_test': 'tests/test_coding_harness.py',
  'hidden_sha256': '781687002b761c6d8b0c5aae80d7ed4384cf1a7590655ea8fc1f4a03f19bd049',
  'source': 'scripts/coding_harness.py',
  'base_sha256': '38590b031cc6bed5c12d3c86bb923d383a006b2a6a00d6ea3653342b6982537d',
  'reference_sha256': '4b1c37aebb77d0d752e59fcba67f79ee05c826c63abdbf79c23009817f494672',
  'projection_sha256': '06ff2dcd1c9cf9c4a71c5f4e5c891c7c90d863248eef17a5486028248e4b056c',
  'statement': "Extend the development harness's implementation identity to bind untracked source "
               'bytes, not just their names. Enumerate untracked files using Git; ignore '
               'Git-ignored build outputs. Return stable path, byte-count and SHA-256 identities '
               'in sorted order. Refuse aliases, non-regular files, over 4096 paths, files over 16 '
               'MiB or aggregate content over 64 MiB, and concurrent changes. Include the '
               'identities and their canonical aggregate digest alongside the existing '
               'tracked-diff, HEAD and binary identities. Preserve existing authority checks. '
               'Interface contract: add untracked_file_identities(root: Path) -> list[dict], '
               'returning rows with path (relative string), bytes (integer) and sha256 (hex). '
               'Enumerate once through subprocess.run with Git ls-files --others '
               '--exclude-standard -z and consume its NUL-separated byte stdout. Raise '
               'HarnessError with coding.harness.source-inventory-alias for symlinks, '
               'coding.harness.identity-not-regular for non-regular files and '
               'coding.harness.source-inventory-limit for size/count limits. '
               'implementation_identity() adds untracked_files (the rows) and '
               'untracked_files_sha256: SHA-256 of the UTF-8 encoding of '
               'json.dumps(rows, sort_keys=True, separators=(",", ":"), ensure_ascii=True).',
  'expected_tests': 12},
 {'id': 'dev-002',
  'split': 'development',
  'base': '6d1cb28424866cb95c1e5460427b4f0bbfe44384',
  'reference': '0898b05a6c82bd531704e3f13db412f537ca4f75',
  'hidden_test': 'tests/test_coding_harness_model_diagnostics.py',
  'hidden_sha256': '33247a099c00ce191309949f52623d9154b9a5b9a66aea4ef35373e01f57d9a4',
  'source': 'scripts/coding_harness.py',
  'base_sha256': '111c4ef587a76f0a70fb8a968c6448718481c3679eed905371ce31e263a826f0',
  'reference_sha256': '3f7f4e3d8c53fc7140324261ef0b501a2882fc2a570b6922b1ff4ff2a5b216eb',
  'projection_sha256': '7995b218e914be7443fb82b5e8fa8b8d7cffbe00882314f19da18b7fcd100445',
  'statement': 'Report the requested model separately from serving or qualification in development '
               'diagnostics. Infer it only from a currently owned process with exactly one --model '
               'argument and a closed supported value, never from an unowned process, missing or '
               'conflicting arguments. Do not echo unknown/private values, change the run-record '
               'schema or claim that a requested model is running or qualified. Preserve '
               'reservation, startup, running and stale-record lifecycle behavior. '
               'Interface contract: add MODEL_CHOICES containing scripted, muse and gpt-oss, '
               'and _model_request_diagnosis(arguments: list[bytes] | None) -> dict. '
               'Always return selection, observation, serving and qualification. '
               'For one exact --model followed by a supported ASCII value, selection is '
               'that string and observation is owned-process-arguments. Otherwise selection '
               'is None and observation is unavailable, including any --model= argument, '
               'duplicates, missing values or unknown bytes. Always set serving to '
               'not-observed and qualification to not-assessed. diagnose(base) adds '
               'model_request using owned arguments only (unavailable otherwise), adds '
               'scope=development-diagnostics-only, renames profile_id to fixture_profile_id '
               'and removes the top-level qualification field. diagnose() output sets '
               'schema_version to 2 (was 1); the persisted run record keeps '
               'schema_version=2 and gains no model fields.',
  'expected_tests': 3},
 {'id': 'dev-003',
  'split': 'development',
  'base': '4fda20f7d9f335cfe859cb3e01206573ea5e3e08',
  'reference': 'e88a9b42cd35821d81f6c8d706b65f151aadcbe2',
  'hidden_test': 'tests/test_local_agent_fast_gate.py',
  'hidden_sha256': 'efe56d376453b501c99f73aeee5a2813e9b9c0d9ab9c7b68df2eb7731a769c23',
  'source': 'scripts/local_agent_fast_gate.py',
  'base_sha256': 'ecdcfac774e042e975b5aeef1c487522b9770e9eb867642e82b267ce35fda965',
  'reference_sha256': '07b48ae80bd9d26a055e128184c289f583665634cf55d6da796cf42e0e034f04',
  'projection_sha256': 'b2e690696102c28664af7689b08372f9ac70761d9870c3c00d9e7a971ec015cf',
  'statement': "Correct the development fast gate's process cleanup: an exited check leader must "
               'remain unreaped while both process-group cleanup signals are sent, preventing '
               'group-ID reuse. Preserve the original exit or signal status, timeout and '
               'interruption behavior. Also accept the actual retained review endings PASS, '
               'FINDINGS, or INCONCLUSIVE followed by exactly one nonempty reason, while rejecting '
               'malformed endings and preserving report bytes. Stop accepting Verdict: FAIL '
               'and Verdict: BLOCKED. Interface contract: add '
               'observe_exit(process: subprocess.Popen) -> int | None. Observe with '
               'os.waitid using WEXITED | WNOHANG | WNOWAIT; return None while running, '
               'the nonnegative exit status on normal exit or negative signal number '
               'on signal exit. stop_check must use observe_exit while keeping the '
               'leader unreaped until both group cleanup signals have been sent.',
  'expected_tests': 23}]


# Append-only development tasks; v5 corrects three new statements before measurement.
# No pilot statement, hidden test, selection, count or pin changes.
# Selections name complete original historical methods, chosen before any model run.
ADDITIONAL_TASKS = [{'base': '4d7e84d669d7a8ce85926deee90336527cad5b05',
  'reference': '4fda20f7d9f335cfe859cb3e01206573ea5e3e08',
  'source': 'scripts/local_agent_fast_gate.py',
  'hidden_test': 'tests/test_local_agent_fast_gate.py',
  'id': 'dev-004',
  'split': 'development',
  'statement': "Repair the development fast gate's coverage and process supervision. Retain "
               'whitespace checking of changed tracked files, and add untracked_whitespace(root: '
               "Path): enumerate Git's NUL-separated untracked, nonignored names and run Git's "
               'whitespace check without staging; accept a binary addition with no diagnostics, '
               'reject whitespace or command errors, and handle spaces/newlines in names. Add '
               'review_text_lint(root: Path, paths: list[str]) for original UTF-8 reports directly '
               'under docs/reviews/ ending in .txt. Require a final newline, no trailing '
               'whitespace, a first line Commit: followed by a full lowercase 40-hex commit, a '
               'nonempty second-line Scope:, and a final explicit PASS, FINDINGS, FAIL or BLOCKED '
               'verdict. Never change report bytes. plan() schedules --lint-untracked for every '
               'batch and --lint-reviews for retained reviews, before any applicable builds; '
               'documentation-only plans contain Git, untracked lint and Markdown lint only. '
               'Accept .rs/.toml/.json/.sql/.txt crate inputs and the existing contracts LICENSE '
               'and extension-sample package source files with full Rust coverage; reject unknown '
               'assets and unknown text outside docs/reviews/, including inside crates. Keep '
               'missing Python-test refusal. Add stop_check, supervise and execute_checks as '
               'needed: each check has a separately owned process group and a supervisor outside '
               "the gate's session, watching a pipe held only by the gate. Clean descendants and "
               'release inherited reservation descriptors after normal check exit, timeout, gate '
               'termination, SIGKILL of the gate group, or supervisor termination. '
               'SIGINT/SIGTERM/SIGHUP stop the gate with failure and never start a later check. '
               'Preserve the execute(root, commands, timeout) interface and existing check '
               'failure/timeout results. untracked_whitespace, review_text_lint and plan '
               'raise ValueError on the refusals described above.',
  'test_names': ['FastGateTests.test_document_only_change_has_no_build_or_evidence_generator',
                 'FastGateTests.test_unknown_change_and_missing_tests_fail_closed',
                 'FastGateTests.test_crate_data_inputs_keep_full_rust_coverage',
                 'FastGateTests.test_untracked_whitespace_uses_git_rules_without_staging',
                 'FastGateTests.test_text_review_has_its_own_format_check_without_editing_it',
                 'FastGateTests.test_normal_exit_cleans_descendants_and_releases_reservation',
                 'FastGateTests.test_termination_cleans_descendants_and_starts_no_next_check',
                 'FastGateTests.test_sigkill_of_gate_group_cleans_descendants_and_releases_reservation',
                 'FastGateTests.test_supervisor_termination_cleans_check_group'],
  'expected_tests': 9,
  'base_sha256': '2c714eb10a3a4c4b655631084efd473c212c4342924d5c072c732303c0624b69',
  'reference_sha256': 'ecdcfac774e042e975b5aeef1c487522b9770e9eb867642e82b267ce35fda965',
  'hidden_sha256': '2b9882485b4b881e616447cf8b1348f6cf50c927487f31c838f8bde557175572',
  'projection_sha256': 'ad5bb9381431ed6713969498ef92114ce13455d941fa3c8396cbf1f7e8805b38'},
 {'base': '55bef249c7fe719cb53ced5be0b1f37471fd2dcd',
  'reference': 'd57374f43b66b6e070854e8e23ce71755e89a760',
  'source': 'scripts/storage_migration_compatibility_evidence.py',
  'hidden_test': 'tests/test_storage_migration_compatibility_evidence.py',
  'id': 'dev-005',
  'split': 'development',
  'statement': 'Fix storage migration compatibility log privacy. In validate_raw(text), retain all '
               'marker/failure checks and additionally reject any remaining checkout or '
               'home-directory path with the diagnostic raw results name a private checkout or '
               "home path. In capture(), replace the exact ROOT spelling in each child command's "
               'captured output with <repository-root>, preserving command headers, return codes, '
               'other output and the number of command results. Use the existing evidence_core '
               'privacy helpers so unrelated home paths stay detectable. A synthetic compile '
               'progress path ROOT/kernel/engine must become (<repository-root>/kernel/engine), '
               'never leak ROOT. This is a text-capture fix only; do not execute a build or renew '
               'historical evidence as part of the solution.',
  'test_names': ['StorageMigrationCompatibilityEvidenceTests.test_raw_results_never_name_the_checkout_or_home_directory'],
  'expected_tests': 1,
  'base_sha256': 'e23b0c53458bcd7d329d1b85f1c5002f3df39554cd8eef365e932d3061215e4b',
  'reference_sha256': '613106edbded15a2429277d4ac84612533608ef9708c1adb4189e6baef2abe30',
  'hidden_sha256': 'a463c2ac2a165578ef3471b3a37a9db225eae4eac0bd0982afe2c150ba4d62b4',
  'projection_sha256': '1cda1dae26446b5e278767523dccc2ab914043d01f9cdd18fbbbe83b53433e62'},
 {'base': 'a84646265bce483cee76aa3a51714389be2d1e05',
  'reference': 'd649b6e4d2320e8d3b2bd291ffe72077a06540fb',
  'source': 'scripts/coding_harness.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'id': 'dev-006',
  'split': 'development',
  'statement': 'Add closed route-grant command arguments to the development harness. Expose '
               'route_grant_arguments(arguments: argparse.Namespace) -> list[str] and parser() '
               'subcommand route-grant with required --root, exactly one of --grant (Path), '
               '--revoke or --list, and optional --workspace. Workspace/grant identifiers match '
               '[A-Za-z0-9][A-Za-z0-9._:-]{0,127}. Listing emits --route-grant-list, with '
               '--route-grant-workspace VALUE only if supplied. Grant/revoke require a workspace; '
               'grant files must be absolute. Emit --route-grant PATH or --route-grant-revoke ID, '
               'followed by --route-grant-workspace VALUE. Refuse relative paths, absent required '
               'workspaces, and invalid identifiers with HarnessError; parser-conflicting '
               'operations fail. Route the operation through the existing catalog owner, '
               'preserving typed confirmation and authority rather than executing grant effects in '
               'Python.',
  'test_names': ['CodingHarnessTests.test_route_grant_operations_reach_the_catalog_host_in_their_closed_forms'],
  'expected_tests': 1,
  'base_sha256': '8324d9dd19b3d81344502ad2edaefdb1f572feaaf382b9fbc4a4bc32f4d0d16a',
  'reference_sha256': '5f26e019919111bf9508b2e4add17378a7465442aa49775b7e65d275346113f5',
  'hidden_sha256': '642d905ee52c458db423a3f1efc17c716a332b4e2947bc61e229f021342754b8',
  'projection_sha256': '8285c6fcc845d75a1d0cc6922a7a006daaa1a7ed2c945ef8aab68b78a90f7228'},
 {'base': '935cfdd8f92aafb2b54cc4a25d3317e21679efa1',
  'reference': '025e3fd5cbb9cac0d4776d16e8cfffda959cfa8d',
  'source': 'scripts/coding_harness.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'id': 'dev-007',
  'split': 'development',
  'statement': 'Extend the development harness with memory_arguments(arguments: '
               'argparse.Namespace) -> list[str] and a memory parser subcommand with required '
               '--root and mutually exclusive --remember TEXT, --list, --revoke ID, --delete ID, '
               '--revoke-source SOURCE. Support --workspace, --cite, --type '
               '(semantic/preference/procedural/episodic, no default value) and --object. Remember '
               'requires workspace and a citation of the form '
               'lowercase-pack-id@major.minor.patch:nonempty-path (at most 4096 characters); '
               'nonblank single-line UTF-8 text is capped at 16 KiB, may not start with -- or '
               'contain ASCII/C1 controls, and object is invalid for remember. Emit '
               '--memory-remember TEXT --memory-workspace W --memory-cite C, then --memory-type '
               'TYPE if supplied. List emits --memory-list, optionally --memory-workspace W; '
               'reject cite/type/object. Revoke/delete emit --memory-revoke ID or --memory-delete '
               'ID and forbid workspace/cite/type/object; IDs match memory-[a-z0-9-]{0,121}. '
               'Revoke-source requires a workspace and lowercase source matching '
               '[a-z0-9._:-]{1,128}, emitting --memory-revoke-source SOURCE --memory-workspace W, '
               'optionally --memory-object OBJECT, matching [a-z0-9._:-]{1,256}. Workspace IDs '
               'match [a-z0-9._:-]{1,128}. Invalid combinations/content raise HarnessError; '
               'invalid parser choices or multiple operations fail parsing. Preserve existing '
               'permission ownership and route catalog operations through the existing owner.',
  'test_names': ['CodingHarnessTests.test_memory_operations_reach_the_catalog_host_in_their_closed_forms'],
  'expected_tests': 1,
  'base_sha256': '6dd9ba8e28c962e487ac5de0de0c662c8810fa5d1d9e7c3ed84ee3e79233dec5',
  'reference_sha256': '0625e783a891fddea8c05e88066be41dc3a5ced0fe7f9bd15692091e1f8eb587',
  'hidden_sha256': '017e12ef95ff6cc9da7a620df09eda7246960d3f76847fddb0b91d6faf103448',
  'projection_sha256': 'ae4feda0fa4a4b2b27b72092921536a38fc0a7dd210a012645897c259129ed51'},
 {'base': 'bc1e2d3924990eab93be5657064b9a8f66b68079',
  'reference': '0e818e86b037688864bc4d2cdfa4de17057dc0d9',
  'source': 'scripts/coding_harness.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'id': 'dev-008',
  'split': 'development',
  'statement': 'Add documentation-pack catalog operations to the development harness, preserving '
               'root reservation and refusing invalid arguments with HarnessError. Provide '
               'doc_pack_arguments(namespace) and parser subcommand doc-pack with required --root '
               'and exactly one of --import directory, --list, --inspect ID, --delete '
               'ID[@version], --search query. Import needs an absolute existing directory and one '
               'or more --allow-license identifiers matching [A-Za-z0-9.+-]{1,64}; optional '
               '--refresh-version is a full three-component nonnegative decimal version without '
               'leading zeroes. Emit --doc-pack-import DIR, repeated --allow-license values, then '
               'optional --refresh-version. Search emits --doc-pack-search QUERY, optional '
               '--doc-pack ID and --include-history; reject blank/over-2048 queries. Other '
               'operations emit their --doc-pack-* flag and value. Pack IDs match '
               '[a-z][a-z0-9.-]{0,63}; reject unrelated options on each operation. Introduce '
               'catalog(base: Path, arguments: list[str]) -> int; ended_run delegates to it, and '
               'both launch the reserved agentmage binary as --json code --development with '
               'state/disposable/workspace roots and the exact argument tail, no --scenario. Also '
               'add SAMPLE_PACK_FILES with synthetic guide/cache.md, guide/tests.md and notes.txt '
               'and write_sample_pack(directory: Path, version: str, retrieved_on: str) -> dict. '
               'Refuse existing/linked/relative targets, malformed full versions and dates outside '
               'YYYY-MM-DD; create private 0700 directories and 0600 files. Write a manifest.json '
               'identical to the returned manifest with integer version.major/minor/patch, sorted '
               'files entries (path, byte_len and SHA-256 matching actual UTF-8 bytes) and '
               'manifest_sha256. Seal by hashing compact JSON in insertion order with '
               'manifest_sha256 empty, then store its hex digest. Preserve native confirmation '
               'boundaries; this task adds catalog argument construction and synthetic sample '
               'generation, not direct catalog authority.',
  'test_names': ['CodingHarnessTests.test_catalog_operations_pass_closed_arguments_to_the_catalog_host',
                 'CodingHarnessTests.test_the_sample_pack_is_sealed_as_the_knowledge_component_seals_it'],
  'expected_tests': 2,
  'base_sha256': '484c8ed16f93da0e26aff781d2554aec3506e54a5c58fb77413794cc09e3c091',
  'reference_sha256': '6dd9ba8e28c962e487ac5de0de0c662c8810fa5d1d9e7c3ed84ee3e79233dec5',
  'hidden_sha256': '655b5f5e1a25b2a1886737f6fe2c9ddfad3457df29c17723d01a733457c06e77',
  'projection_sha256': '587e632a702c5aad42dc76749bb26988ae1dc18fee583e0580099d152a6d5af2'},
 {'base': '1458cef48d289ff735ae1f5b8364c6e260f54e49',
  'reference': '3dcaabf3fc744daec71f6870c4be31f190841251',
  'source': 'scripts/configuration_result_evidence.py',
  'hidden_test': 'tests/test_configuration_result_evidence.py',
  'id': 'dev-009',
  'split': 'development',
  'statement': 'Preserve diagnostic output when '
               'configuration_result_evidence.subprocess_runner(command, root) observes a nonzero '
               'child exit. Combine its captured stdout and stderr, keep at most the last 16,384 '
               'characters and print that tail to sys.stderr, with a terminating newline. Then '
               'raise RuntimeError identifying the failed executable and including exited N for '
               'the actual exit code. A child that prints a diagnostic and exits 3 must leave that '
               "diagnostic in the caller's captured stderr. Keep the existing successful return "
               'behavior and invocation confinement unchanged.',
  'test_names': ['ConfigurationResultEvidenceTests.test_a_failed_child_command_leaves_its_output_in_the_log'],
  'expected_tests': 1,
  'base_sha256': '67df8af0b3c3b3c91d0402a870fe9fd94007d72795bf366215e376aafb49579d',
  'reference_sha256': '03b46323130d5dcedd8b6535127e1344cd71726e7ef479936d2f0ad5be82c1f5',
  'hidden_sha256': 'a788ea021dc5b94a7ae81aab4254d3dbda7e877fadc1db6827b6e5fe2d7ff973',
  'projection_sha256': 'a7d6fa0fd572126ab8f67f833dcc16e49f00b3004826975a0b9a1243a5ce365b'},
 {'base': '1458cef48d289ff735ae1f5b8364c6e260f54e49',
  'reference': '3dcaabf3fc744daec71f6870c4be31f190841251',
  'source': 'scripts/rust_test_ownership.py',
  'hidden_test': 'tests/test_rust_test_ownership.py',
  'id': 'dev-010',
  'split': 'development',
  'statement': 'Fix rust_test_ownership.test_only_files so ordinary Rust module resolution '
               'includes name/mod.rs as an alternative to name.rs. A production reference must '
               'keep that file and its production descendants out of the test-only set, including '
               'when a test-only include or path declaration also points at that file. Test-only '
               'children of a mod.rs owner remain eligible. If both default module paths exist, '
               'raise AmbiguousReference rather than choose one. Preserve existing test-only '
               'propagation, conservative ambiguity handling, string/comment masking and the '
               'public Mapping[Path, str] -> frozenset[Path] interface. This is source '
               'classification, not a replacement for compiling Rust.',
  'test_names': ['RustTestOwnershipTests.test_mod_rs_modules_are_production_references'],
  'expected_tests': 1,
  'base_sha256': '7b49cd60f9e8a78059ebb8dc7f15709fd37c747ccbac98001cde375d8506c86f',
  'reference_sha256': '74b1db6953f3a3ffeffbc44210bb689c4efa54092d78a92cce0e11f694487491',
  'hidden_sha256': '3d05c65b512cebc0dd37671612073ae6cf21b89281430a846920c10e758a7cd2',
  'projection_sha256': 'a7d6fa0fd572126ab8f67f833dcc16e49f00b3004826975a0b9a1243a5ce365b'},
 {'base': '1458cef48d289ff735ae1f5b8364c6e260f54e49',
  'reference': '3dcaabf3fc744daec71f6870c4be31f190841251',
  'source': 'scripts/verification_batch.py',
  'hidden_test': 'tests/test_verification_batch.py',
  'id': 'dev-011',
  'split': 'development',
  'statement': 'Strengthen verification_batch.retain and load_stage without weakening historical '
               'evidence bindings. Introduce the string TOOL = "scripts/verification_batch.py". New schema-2 '
               'stages must bind TOOL and every source path/digest to the exact source_revision. '
               'Add retain(..., changed=tuple[(relative_path, later_commit)]) as a '
               'keyword-compatible final argument: a differing source may be declared only once, '
               'must be included among sources, must differ from the named source revision, and '
               'must equal the blob in a strictly later descendant commit. Record changed_sources '
               'rows with path, sha256 and full commit, and recheck them when loading. Reject '
               'omitted/redundant/foreign declarations, missing tool binding, source digest '
               'mismatch or tampered records using RecordError. Source paths are closed '
               'repository-relative names, never absolute or containing . or .. components. Before '
               'creating any output directory, check every written field, including source names, '
               'metadata and logs, for private names case-insensitively after redaction; keep '
               'existing private_names behavior. exit_code must have exact int type, seconds '
               'int/float (not bool), nonnegative; reject strings or other values both before '
               'retain and on schema-2 load. Preserve the legacy schema-1 historical inspection '
               'behavior and current log integrity checks.',
  'test_names': ['VerificationBatchTests.test_stage_sources_are_verified_against_their_revision',
                 'VerificationBatchTests.test_every_written_field_is_checked_and_typed_before_anything_is_written'],
  'expected_tests': 2,
  'base_sha256': '36c357d189a64484420eb50a3168b7ce171f18b1b1a29ea762495de9dae2b250',
  'reference_sha256': 'd22df0b2e034fac633681ce51517ff88488751c8cd03fa6973c75ff834172dd9',
  'hidden_sha256': 'f578cab015f8548aa30cb46dd39bec7e759282e2b18460f4eef9c249e2801d38',
  'projection_sha256': 'a7d6fa0fd572126ab8f67f833dcc16e49f00b3004826975a0b9a1243a5ce365b'},
 {'base': '28dd0e4435214e5e6d488912792d5986e4b4b774',
  'reference': '41e3f432df95b290e3a079193ee04fb3e942a7cf',
  'source': 'scripts/rust_test_ownership.py',
  'hidden_test': 'tests/test_rust_test_ownership.py',
  'id': 'dev-012',
  'split': 'development',
  'statement': 'Make rust_test_ownership.test_only_files conservative for compiler module loading. '
               'Recognize raw identifiers in mod r#name declarations, with the same '
               'production/test context as ordinary names. Default children of files loaded via an '
               'explicit #[path] or include! live beside that loaded file, regardless of whether '
               'the owning file is lib.rs or engine.rs. Default-loaded non-mod.rs files retain '
               'their stem child directory. If a file is loaded both by default and explicitly and '
               'its default children would depend on its loader, raise AmbiguousReference; the '
               'same mixed loading with no children is allowed. Treat path-selecting cfg_attr '
               '(multiline and inner attributes included), unrecognized path attributes, '
               'macro_rules bodies that can declare modules and mod $fragment forms as ambiguous, '
               'preventing any whole-file test exemption. Preserve the test_only_files '
               'Mapping[Path,str] -> frozenset[Path] interface, exact #[cfg(test)] handling and '
               'conservative unresolvable-reference behavior. This lexical classification does not '
               'replace native compilation.',
  'test_names': ['RustTestOwnershipTests.test_forms_that_can_select_a_production_file_are_ambiguous',
                 'RustTestOwnershipTests.test_raw_identifier_declarations_are_production_references',
                 'RustTestOwnershipTests.test_children_of_path_loaded_and_included_files_resolve_beside_them',
                 'RustTestOwnershipTests.test_a_file_whose_children_depend_on_its_loader_is_ambiguous'],
  'expected_tests': 4,
  'base_sha256': 'fc2944fc2dd99a61957c990d6cd291c1201b51be66cefe37895e81ed678f4302',
  'reference_sha256': '7b49cd60f9e8a78059ebb8dc7f15709fd37c747ccbac98001cde375d8506c86f',
  'hidden_sha256': 'b37e7574c94f62a8de33f77851b8d99a6084c76af9e91b8d8e8467e200a3fd32',
  'projection_sha256': '5c39e5d6a3752f64c704e54f5ae8f9d736dd014ac27f10a4d642a4fd83a86d89'},
 {'base': 'eef257aadc077a259b676e0b24232013162ac997',
  'reference': '50f0dbb9728a68657c404f0f00cd53feef5e192a',
  'source': 'scripts/coding_harness.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'id': 'dev-013',
  'split': 'development',
  'statement': "Replace the development harness's mere file-presence check for confinement "
               'prerequisites with truthful diagnostics. Add native_executable_prerequisite(path: '
               'Path) -> str: use lstat on its non-root path ancestors and itself; non-root '
               'ownership, group/world writability or a symlink yields untrusted-path. A '
               'missing/unreadable path yields unavailable; an empty/nonregular/nonexecutable leaf '
               'yields not-executable; otherwise available. Add confinement_diagnosis() -> dict '
               'with scope=prerequisites-only, native_executables for '
               'bwrap/systemd-run/systemctl/git, user_manager and ready. Probe systemctl only when '
               'that executable is trusted; otherwise user_manager=not-probed-untrusted-systemctl. '
               'Use [/usr/bin/systemctl, --user, --no-pager, show, --property=Version, --value], '
               'timeout 3 seconds, all standard streams DEVNULL, check=False, and only LANG=C, '
               'XDG_RUNTIME_DIR=/run/user/UID and '
               'DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/UID/bus in env. Map '
               'success/nonzero/timeout/OSError to available/unavailable/timeout/unavailable '
               'without exposing command output or exception text. ready requires every executable '
               'and manager available; this never claims confinement admission. diagnose() '
               'includes confinement_prerequisites and derives its confinement field from ready.',
  'test_names': ['CodingHarnessTests.test_diagnosis_does_not_admit_present_but_untrusted_native_files',
                 'CodingHarnessTests.test_native_diagnostic_rejects_alias_writable_missing_and_nonexecutables',
                 'CodingHarnessTests.test_diagnosis_reports_missing_or_timed_out_manager_without_exposing_output'],
  'expected_tests': 3,
  'base_sha256': '4b1c37aebb77d0d752e59fcba67f79ee05c826c63abdbf79c23009817f494672',
  'reference_sha256': 'ad2278426f5a9ed69217c8eb0a0e4fe9c4f598feeb208d4fe39603dcaf8655f7',
  'hidden_sha256': '8c5f92ad5edfceeb5f732f0a47a765b35470a6ede6465de8baf512cb022c05a9',
  'projection_sha256': '9991c0ff0ab524bbc191929aa51e02c418a2095dac6ed2842641ca415d1c8940'},
 {'base': 'f38f9f85fdbc02c1f7d37b0b89e5fb2bef2ba491',
  'reference': 'c16884163206ebacfa8885d3f7416e684c44eb7c',
  'source': 'scripts/sprint_16_linux_worker_evidence.py',
  'hidden_test': 'tests/test_sprint_16_linux_worker_evidence.py',
  'id': 'dev-014',
  'split': 'development',
  'statement': 'Bind installed-worker evidence to the complete committed source tree while '
               'preserving the original source bindings. Add complete_source_closure(revision) -> '
               'dict and MAX_SOURCE_FILES=20000, MAX_SOURCE_BYTES=256 MiB. Only accept a full '
               '40-lowercase-hex commit; obtain its recursive NUL-delimited Git inventory and '
               "committed blob bytes, never worktree substitutes. Include every file's path, "
               'executable/symlink mode, byte size and content in a deterministic digest; equal '
               'trees across commits give equal closures, while changed child modules, mode bits '
               'or documentation change the result. Return exactly schema_version=1, '
               'scope=complete-committed-source-tree, positive file_count, nonnegative total_bytes '
               'and sha256. Refuse empty/oversize inventories, duplicate paths, invalid object '
               'identities/types/modes, malformed or truncated batch headers/content, and trailing '
               'batch bytes. Add validate_current_source(report, revision) -> list[str]: accept '
               'only a structurally valid schema-2 worker report whose original sources and '
               'complete_source_closure equal actual committed source at its source_revision and '
               'whose complete closure equals revision; refuse unavailable commits and malformed '
               'inputs as failures. Preserve validate_report for historical schema-1 reports, but '
               'never accept them as current. Schema versions, file_count, bytes and total_bytes '
               'require exact integer types, not booleans/floats; closure keys are exact. Current '
               'build_report emits schema_version=2 and the closure. Changed guest results, '
               'duplicate/missing sources, malformed digests/closures and changed source must '
               'fail. These tests use synthetic Git histories only; no installed worker or other '
               'platform qualification is established. Git call contract for complete_source_closure: '
               'on valid input make exactly three subprocess.run calls in order, reading each '
               '.stdout as bytes: [git, cat-file, -t, REV] must return b"commit\\n"; '
               '[git, ls-tree, -r, -l, -z, --full-tree, REV] returns long-format inventory '
               'records with object sizes; [git, cat-file, --batch] receives newline-delimited '
               'object IDs on input. Use cwd=ROOT, check=True, capture_output=True and timeouts '
               '30, 30 and 60 seconds respectively. Refuse invalid input or inventory before '
               'making subsequent calls. Let subprocess.CalledProcessError propagate for an '
               'unavailable revision; validate_current_source catches it and returns failures. '
               'That validator checks the report source_revision closure before the local '
               'revision closure (exactly two closure calls on the equivalence path). '
               'Diagnostic contract for other closure refusals: ValueError (or the existing '
               'WorkerEvidenceError subclass); closure-limit errors include exceeds bound and '
               'duplicate/invalid inventories include inventory invalid. For current validation, '
               'legacy rejection has historical in the first returned failure, and non-equivalent '
               'source rejection has not equivalent in the first returned failure.',
  'test_names': ['Sprint16LinuxWorkerEvidenceTests.test_identical_complete_source_at_different_commits_is_eligible',
                 'Sprint16LinuxWorkerEvidenceTests.test_legacy_matrix_is_historical_not_current',
                 'Sprint16LinuxWorkerEvidenceTests.test_schema_versions_require_integer_not_python_equal_boolean_or_float',
                 'Sprint16LinuxWorkerEvidenceTests.test_missing_forged_duplicate_and_incomplete_closures_fail',
                 'Sprint16LinuxWorkerEvidenceTests.test_changed_or_unavailable_committed_source_fails',
                 'Sprint16LinuxWorkerEvidenceTests.test_real_git_closure_covers_child_modules_modes_and_docs',
                 'Sprint16LinuxWorkerEvidenceTests.test_batch_integrity_refuses_truncation_identity_and_trailing_bytes'],
  'expected_tests': 7,
  'base_sha256': '7cdb64099130ea09b9fc15bde65d45cedc059c000dba22c6ba2d0c7f56c8c0d4',
  'reference_sha256': '1e934618af71dfa58f6f4b9485705b06d76bb69d2289742954cb1fdeafae1e66',
  'hidden_sha256': 'fd51481a51edcdf124ec7ce8103eb35d86f708b38dd2746de47461b6f57f0b3d',
  'projection_sha256': 'df86d856aa06b2de7b88a350b09e770dc817f52f7295a41e90690ecef14783b9'}]
TASKS += ADDITIONAL_TASKS


class EvaluationError(ValueError):
    """A fixed diagnostic; task/candidate contents stay in private output."""


class InputIntegrityError(EvaluationError):
    """Frozen evaluator inputs are unavailable or changed; this is not a test failure."""


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git(*args: str) -> bytes:
    return subprocess.check_output(
        ["git", "--no-optional-locks", *args], cwd=ROOT,
        stderr=subprocess.DEVNULL, timeout=30,
    )


def pinned_blob(revision: str, name: str, expected: str) -> bytes:
    try:
        data = git("show", revision + ":" + name)
    except subprocess.CalledProcessError:
        raise InputIntegrityError("evaluation.input-unavailable") from None
    if sha(data) != expected:
        raise InputIntegrityError("evaluation.input-hash-mismatch")
    return data


def projection(task: dict) -> dict[str, bytes]:
    """Frozen editing inputs; no Git history, hidden regressions or reference fix."""
    if task.get("language") == "rust":
        return coding_eval_rust.projection(task, sys.modules[__name__])
    entries = [line for line in git("ls-tree", "-r", task["base"], "--", "scripts").splitlines()
               if line.endswith(b".py")]
    if sha(b"\n".join(entries) + b"\n") != task["projection_sha256"]:
        raise InputIntegrityError("evaluation.projection-hash-mismatch")
    archive = git("archive", "--format=tar", task["base"], "scripts")
    if len(archive) > 32 * MAX_FILE:
        raise EvaluationError("evaluation.archive-too-large")
    result = {}
    with tarfile.open(fileobj=io.BytesIO(archive)) as stream:
        for member in stream:
            path = Path(member.name)
            if path.suffix != ".py":
                continue
            if (not member.isfile() or path.is_absolute() or ".." in path.parts
                    or path.parts[0] != "scripts" or member.size > MAX_FILE):
                raise EvaluationError("evaluation.archive-entry-refused")
            result[member.name] = stream.extractfile(member).read()
    if sum(map(len, result.values())) > MAX_TREE:
        raise EvaluationError("evaluation.projection-too-large")
    if sha(result[task["source"]]) != task["base_sha256"]:
        raise InputIntegrityError("evaluation.base-hash-mismatch")
    return result


def new_directory(path: Path) -> None:
    # Existing paths (including dangling aliases) are never adopted or overwritten.
    path.mkdir(mode=0o700)


def write_file(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(content)


def publish(path: Path, value: dict) -> None:
    write_file(path, (json.dumps(value, sort_keys=True, indent=2) + "\n").encode())


def read_candidate(candidate: Path, original: dict[str, bytes], allowed: str) -> dict[str, bytes]:
    """Snapshot only an exact projection with one editable path; refuse extra inputs."""
    result = {}
    total = 0

    def walk(directory: int, prefix: str) -> None:
        nonlocal total
        for name in os.listdir(directory):
            relative = prefix + name
            info = os.stat(name, dir_fd=directory, follow_symlinks=False)
            if stat.S_ISDIR(info.st_mode):
                if not any(p.startswith(relative + "/") for p in original):
                    raise EvaluationError("evaluation.candidate-directory-refused")
                child = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=directory)
                try:
                    walk(child, relative + "/")
                finally:
                    os.close(child)
                continue
            if stat.S_ISLNK(info.st_mode):
                raise EvaluationError("evaluation.candidate-directory-refused")
            if relative not in original:
                raise EvaluationError("evaluation.candidate-file-refused")
            descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=directory)
            with os.fdopen(descriptor, "rb") as stream:
                before = os.fstat(stream.fileno())
                if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or before.st_size > MAX_FILE:
                    raise EvaluationError("evaluation.candidate-file-refused")
                data = stream.read(MAX_FILE + 1)
                after = os.fstat(stream.fileno())
            fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
            if (len(data) != before.st_size or len(data) > MAX_FILE
                    or any(getattr(before, f) != getattr(after, f) for f in fields)):
                raise EvaluationError("evaluation.candidate-changed")
            if relative != allowed and data != original[relative]:
                raise EvaluationError("evaluation.out-of-scope-edit")
            result[relative] = data
            total += len(data)
            if total > MAX_TREE:
                raise EvaluationError("evaluation.candidate-too-large")

    descriptor = os.open(candidate, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        walk(descriptor, "")
    finally:
        os.close(descriptor)
    if result.keys() != original.keys():
        raise EvaluationError("evaluation.candidate-file-missing")
    return result


def require_historical_candidate(task: dict, files: dict[str, bytes]) -> str:
    """Only frozen controls may run with hidden tests in a candidate process.

    Candidate Python or Rust can forge its test result. Until independently controlled
    scoring and separate aggregate quotas exist, refuse all generated candidates
    before creating output or launching a worker. Host callers are trusted; this
    check is outside every candidate mount and process.
    """
    registered = next((value for value in collection() if value == task), None)
    if registered is None:
        raise EvaluationError("evaluation.task-definition-refused")
    expected = projection(registered)
    source = registered["source"]
    if files == expected:
        return "frozen-base"
    expected[source] = pinned_blob(registered["reference"], source, registered["reference_sha256"])
    if files == expected:
        return "frozen-reference"
    raise EvaluationError("evaluation.untrusted-candidate-disabled")


@contextmanager
def case_workspace(output: Path):
    """Retry transient cleanup failures; retain a private recovery record otherwise."""
    root = Path(tempfile.mkdtemp(prefix="agentmage-eval-"))
    failed = False
    try:
        yield root
    except BaseException:
        failed = True
        raise
    finally:
        try:
            remove_case_workspace(root, output)
        except (EvaluationError, OSError):
            # Preserve the original exception, especially interruption: cleanup
            # must not turn a stop request into a case failure and continue verify.
            if not failed:
                raise


def remove_case_workspace(root: Path, output: Path) -> None:
    for attempt in range(3):
        try:
            shutil.rmtree(root)
            return
        except OSError as error:
            if isinstance(error, FileNotFoundError) and not root.exists():
                return
            if attempt == 2:
                publish(output / "cleanup.json", {"removed": False, "attempts": 3,
                                                  "private_workspace": str(root)})
                raise EvaluationError("evaluation.cleanup-failed") from None
        time.sleep(0.05)


DRIVER = b'''import importlib.util
import json
import sys
import unittest
sys.dont_write_bytecode = True
sys.path.insert(0, "/workspace")
spec = importlib.util.spec_from_file_location("hidden_eval", "/grader/test.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
suite = unittest.defaultTestLoader.loadTestsFromModule(module)
result = unittest.TextTestRunner(stream=sys.stderr, verbosity=2).run(suite)
report = {"tests": result.testsRun, "failures": len(result.failures),
          "errors": len(result.errors), "skipped": len(result.skipped),
          "expected_failures": len(result.expectedFailures),
          "unexpected_successes": len(result.unexpectedSuccesses)}
with open("/output/tests.json", "x") as stream:
    json.dump(report, stream, sort_keys=True)
raise SystemExit(0 if result.wasSuccessful() and result.testsRun > 0
                 and not result.skipped and not result.expectedFailures else 1)
'''


def task_driver(task: dict) -> bytes:
    """Freeze optional regression selection without altering historical test bytes.

    Legacy tasks still load the whole module. New development tasks select full
    original test methods and include that list in their frozen task identity.
    There is no CLI/environment override and no score-dependent selection.
    """
    if task.get("language") == "rust":
        return coding_eval_rust.DRIVER
    if "test_names" not in task:
        return DRIVER
    names = task["test_names"]
    if (not isinstance(names, list) or not names
            or any(not isinstance(name, str) or re.fullmatch(
                r"[A-Za-z_][A-Za-z0-9_]*\.test_[A-Za-z0-9_]+", name) is None for name in names)
            or len(set(names)) != len(names) or type(task["expected_tests"]) is not int
            or len(names) != task["expected_tests"]):
        raise InputIntegrityError("evaluation.test-selection-refused")
    return DRIVER.replace(
        b"unittest.defaultTestLoader.loadTestsFromModule(module)",
        ("unittest.defaultTestLoader.loadTestsFromNames(" + json.dumps(names) + ", module)").encode(),
    )


def limits() -> None:
    resource.setrlimit(resource.RLIMIT_AS, (512 * MAX_FILE, 512 * MAX_FILE))
    resource.setrlimit(resource.RLIMIT_FSIZE, (32 * MAX_FILE, 32 * MAX_FILE))
    resource.setrlimit(resource.RLIMIT_CPU, (60, 60))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def sandbox_command(workspace: Path, grader: Path, output: Path) -> list[str]:
    return [
        "/usr/bin/bwrap", "--unshare-all", "--unshare-user", "--die-with-parent", "--new-session",
        "--disable-userns", "--clearenv", "--cap-drop", "ALL",
        "--ro-bind", "/usr", "/usr", "--symlink", "usr/bin", "/bin",
        "--symlink", "usr/lib", "/lib", "--symlink", "usr/lib64", "/lib64",
        "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp",
        "--ro-bind", str(workspace), "/workspace",
        "--ro-bind", str(grader), "/grader", "--bind", str(output), "/output",
        "--setenv", "PATH", "/usr/bin", "--setenv", "HOME", "/tmp",
        "--setenv", "LANG", "C.UTF-8", "--chdir", "/workspace",
        "/usr/bin/python3", "-I", "-B", "/grader/run.py",
    ]


def read_regular(path: Path, maximum: int) -> bytes:
    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or before.st_size > maximum:
            raise EvaluationError("evaluation.result-file-refused")
        data = stream.read(maximum + 1)
        after = os.fstat(stream.fileno())
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    if (len(data) != before.st_size or len(data) > maximum
            or any(getattr(before, f) != getattr(after, f) for f in fields)):
        raise EvaluationError("evaluation.result-file-changed")
    return data


def initialize_grading_repository(workspace: Path) -> None:
    """One candidate-only commit for Git-dependent diagnostics, without source history."""
    env = {"PATH": "/usr/bin", "LANG": "C.UTF-8", "HOME": str(workspace),
           "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
           "GIT_TERMINAL_PROMPT": "0", "GIT_AUTHOR_DATE": "2000-01-01T00:00:00+00:00",
           "GIT_COMMITTER_DATE": "2000-01-01T00:00:00+00:00"}
    for arguments in (
        ["init", "--quiet", "--template=", "--initial-branch=evaluation-candidate"],
        ["add", "--", "scripts"],
        ["-c", "user.name=Synthetic", "-c", "user.email=synthetic@example.invalid",
         "-c", "core.hooksPath=/dev/null", "commit", "--quiet", "-m", "Candidate snapshot"],
    ):
        subprocess.run(["/usr/bin/git", "-c", "maintenance.auto=false", "-c", "gc.auto=0",
                        *arguments], cwd=workspace, env=env, check=True,
                       stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, timeout=30)


def grade_case(task: dict, files: dict[str, bytes], output: Path) -> tuple:
    driver = task_driver(task)
    hidden = pinned_blob(task.get("hidden_revision", task["reference"]), task["hidden_test"], task["hidden_sha256"])
    data = None
    with case_workspace(output) as root:
        workspace, grader, result_dir = root / "workspace", root / "grader", root / "results"
        workspace.mkdir(mode=0o700)
        grader.mkdir(mode=0o700)
        result_dir.mkdir(mode=0o700)
        for name, content in files.items():
            write_file(workspace / name, content)
        rust = task.get("language") == "rust"
        if rust:
            coding_eval_rust.materialize(workspace, task, sys.modules[__name__])
        else:
            initialize_grading_repository(workspace)
        write_file(grader / "test.py", hidden)
        write_file(grader / "run.py", driver)
        command = (coding_eval_rust.sandbox_command(workspace, grader, result_dir, sys.modules[__name__])
                   if rust else sandbox_command(workspace, grader, result_dir))
        timed_out = False
        output_limited = False
        process = None
        try:
            with (output / "stdout.log").open("xb") as stdout, (output / "stderr.log").open("xb") as stderr:
                process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                           stderr=subprocess.PIPE, start_new_session=True,
                                           preexec_fn=coding_eval_rust.limits if rust else limits)
                deadline = time.monotonic() + TIMEOUT
                exit_code = None
                totals = {stdout: 0, stderr: 0}
                with selectors.DefaultSelector() as events:
                    events.register(process.stdout, selectors.EVENT_READ, stdout)
                    events.register(process.stderr, selectors.EVENT_READ, stderr)
                    while True:
                        for key, _ in events.select(timeout=0.01):
                            data = os.read(key.fileobj.fileno(), 65536)
                            if not data:
                                events.unregister(key.fileobj)
                                key.fileobj.close()
                                continue
                            remaining = 2 * MAX_FILE - totals[key.data]
                            key.data.write(data[:remaining])
                            totals[key.data] += min(len(data), remaining)
                            if len(data) > remaining:
                                output_limited = True
                        status = os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                        if status is not None and not events.get_map():
                            exit_code = status.si_status if status.si_code == os.CLD_EXITED else -status.si_status
                            break
                        if output_limited:
                            break
                        if time.monotonic() >= deadline:
                            timed_out = True
                            break
        finally:
            if process is not None:
                # Bubblewrap's private PID namespace also kills escaped descendants.
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
                for pipe in (process.stdout, process.stderr):
                    if pipe is not None:
                        pipe.close()
        tests = None
        try:
            data = read_regular(result_dir / "tests.json", 4096)
            tests = json.loads(data)
            fields = {"tests", "failures", "errors", "skipped", "expected_failures", "unexpected_successes"}
            if (not isinstance(tests, dict) or tests.keys() != fields
                    or any(type(v) is not int or v < 0 for v in tests.values())
                    or tests["tests"] != task["expected_tests"]):
                raise ValueError
        except (OSError, ValueError):
            tests = None
    # Counts are promoted only after process and temporary-tree cleanup succeeds.
    if tests is not None:
        write_file(output / "tests.json", data)
    return exit_code, timed_out, output_limited, tests


def grade(task: dict, files: dict[str, bytes], output: Path) -> dict:
    candidate_kind = require_historical_candidate(task, files)
    # Operator errors (including an existing destination) still fail before adoption.
    new_directory(output)
    started = time.monotonic()
    infrastructure_error = None
    input_error = None
    try:
        exit_code, timed_out, output_limited, tests = grade_case(task, files, output)
    except (EvaluationError, OSError, subprocess.SubprocessError) as error:
        # Keep a failed case and continue validation; never turn partial counts into PASS.
        if isinstance(error, InputIntegrityError):
            input_error = refusal_detail(error)
        else:
            infrastructure_error = refusal_detail(error)
        exit_code, timed_out, output_limited, tests = None, False, False, None
        (output / "tests.json").unlink(missing_ok=True)
        for name in ("stdout.log", "stderr.log"):
            if not (output / name).exists():
                write_file(output / name, b"")
    passed = (infrastructure_error is None and input_error is None and exit_code == 0 and not timed_out
              and not output_limited and tests is not None
              and all(tests[k] == 0 for k in tests if k != "tests"))
    cause = ("none" if passed else "input_integrity" if input_error else "output_limit" if output_limited
             else "timeout" if timed_out else "infrastructure"
             if tests is None else "test_failure")
    report = {"version": VERSION, "task": task["id"], "split": task["split"],
              "base": task["base"], "hidden_sha256": task["hidden_sha256"],
              "language": task.get("language", "python"),
              "fixture_sha256": task.get("fixture_sha256"),
              "test_names": task.get("test_names"),
              "evaluator_sha256": sha(Path(__file__).read_bytes()),
              "collection_module_sha256": sha(Path(coding_eval_collection.__file__).read_bytes()),
              "rust_adapter_sha256": sha(Path(coding_eval_rust.__file__).read_bytes()),
              "task_sha256": sha(json.dumps(task, sort_keys=True, separators=(",", ":")).encode()),
              "candidate_sha256": sha(files[task["source"]]), "passed": passed,
              "cause": cause, "exit_code": exit_code, "tests": tests, "output_truncated": output_limited,
              "infrastructure_error": infrastructure_error, "input_error": input_error,
              "candidate_kind": candidate_kind, "score_scope": "trusted-evaluator-validation-only",
              "model_score": None,
              "wall_seconds": round(time.monotonic() - started, 6),
              "gpu_minutes": 0, "tokens": None, "model_attempts": 0,
              "logs": {name: sha(read_regular(output / name, 2 * MAX_FILE)) for name in ("stdout.log", "stderr.log")}}
    publish(output / "report.json", report)
    return report


def collection() -> list[dict]:
    """Keep the accepted Python task dictionaries intact when adding languages."""
    return TASKS + coding_eval_collection.TASKS + coding_eval_rust.TASKS


def verify(output: Path) -> dict:
    """Validate evaluator discrimination, never count reference patches as agent attempts."""
    new_directory(output)
    rows = []
    for task in collection():
        original = projection(task)
        base = grade(task, original, output / (task["id"] + "-base"))
        fixed = dict(original)
        fixed[task["source"]] = pinned_blob(task["reference"], task["source"], task["reference_sha256"])
        reference = grade(task, fixed, output / (task["id"] + "-reference"))
        rows.append({"task": task["id"], "base": base, "reference": reference,
                     "valid": not base["passed"] and base["cause"] == "test_failure" and reference["passed"]})
    summary = {"version": VERSION, "kind": "evaluator-validation-only", "tasks": rows,
               "collection_sha256": sha(json.dumps(collection(), sort_keys=True, separators=(",", ":")).encode()),
               "passed": all(row["valid"] for row in rows), "model_score": None,
               "input_refused": any(case["cause"] == "input_integrity" for row in rows
                                    for case in (row["base"], row["reference"]))}
    publish(output / "summary.json", summary)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("list", "prepare", "score", "verify"))
    parser.add_argument("--task", choices=[t["id"] for t in collection()])
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.operation == "list":
        print(json.dumps({"version": VERSION, "tasks": [
            {k: task[k] for k in ("id", "split", "base", "source", "statement")} for task in collection()
        ]}, indent=2))
        return 0
    if args.output is None:
        parser.error("--output is required")
    if args.operation == "verify":
        report = verify(args.output)
        return 0 if report["passed"] else 2 if report["input_refused"] else 1
    if args.task is None:
        parser.error("--task is required")
    task = next(t for t in collection() if t["id"] == args.task)
    files = projection(task)
    if args.operation == "prepare":
        new_directory(args.output)
        for name, content in files.items():
            write_file(args.output / name, content)
        return 0
    if args.candidate is None:
        parser.error("--candidate is required")
    files = read_candidate(args.candidate, files, task["source"])
    report = grade(task, files, args.output)
    return 0 if report["passed"] else 2 if report["cause"] == "input_integrity" else 1


def interrupted(signum: int, frame: object) -> None:
    raise KeyboardInterrupt


def cli() -> int:
    """Expose fixed diagnostics without exception messages containing private paths."""
    try:
        return main()
    except (EvaluationError, OSError, subprocess.SubprocessError) as error:
        print("evaluation.refused " + refusal_detail(error) + "; inspect private output", file=sys.stderr)
        return 2


def refusal_detail(error: Exception) -> str:
    code = (str(error) if isinstance(error, EvaluationError) else
            "evaluation.io-failed" if isinstance(error, OSError) else "evaluation.process-failed")
    return f"code={code} type={type(error).__name__}"


if __name__ == "__main__":
    for termination_signal in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(termination_signal, interrupted)
    raise SystemExit(cli())

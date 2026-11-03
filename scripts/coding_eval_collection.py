"""Append-only frozen development tasks from original repository history.

No model attempt informed selection; hidden modules remain historical bytes.
These controls do not admit generated scoring or a held-out collection.
"""

TASKS = [{'id': 'dev-015',
  'split': 'development',
  'base': 'ac6985c0877e8e5921bf592ded56595199e0e146',
  'reference': 'a89cc70e18043e88f1eb8fcc978d87cf70cd0093',
  'source': 'scripts/model_feasibility.py',
  'hidden_test': 'tests/test_model_feasibility.py',
  'statement': 'Fix case_passed(case) in scripts/model_feasibility.py so resource and network '
               'trials require measured outcomes rather than trusting trials_completed. For every '
               'case require len(trials) == trials_expected. For MEM-001 each trial must have '
               'swap_growth_bytes equal to zero and unload_returns_to_bounded_baseline true. For '
               'NET-001 each trial must have zero dns_queries, outbound_connection_attempts, '
               'egress_bytes and undeclared_listeners. Return false if any trial fails; preserve '
               'the other case predicates. Change only this source file. No real model, network or '
               'memory measurement is part of this task; the regression uses synthetic trials.',
  'test_names': ['ModelFeasibilityTests.test_resource_and_network_cases_require_measured_outcomes'],
  'expected_tests': 1,
  'base_sha256': 'a686ea4ce4248aa8caa972a90c1128cde4bef0dedc395228260890c718f8c245',
  'reference_sha256': 'e0853d42d06a4e58df3bb70dd1f79974a6a3ad07b1d65aa14a84ec41e4d91ea0',
  'hidden_sha256': '3ca3adf274065fc7c68a85064f16f456b063435cdf0f92ba2a2d1a702d84dffc',
  'projection_sha256': 'b38aa38ea46c2c7b87adc9f4acf256bcf1f9e3f427c1bd9ab7fcbf50b2025d49'},
 {'id': 'dev-016',
  'split': 'development',
  'base': 'dcb2c824e93ac10e0732653c0369b1e1ca072a42',
  'reference': '0d132ecd6cdd6ffae37e956d5e5d1cf925b579bb',
  'source': 'scripts/model_feasibility.py',
  'hidden_test': 'tests/test_model_feasibility.py',
  'statement': 'Repair OpenAIAdapter.tokenize_messages usage-token counting in '
               'scripts/model_feasibility.py. Retain the existing cache by canonical message '
               'identity: a second call for the same messages must not make another request. For '
               'usage_token_counter=True use unix_http_request, requiring socket_path, with '
               'positional arguments POST, self.socket_path, self.path_prefix plus '
               '/v1/chat/completions, and the payload; timeout=600.0 and '
               'allowed_statuses=frozenset({400}). Add that keyword-only allowed_statuses '
               'parameter to unix_http_request with an empty frozenset default; it permits only '
               'those non-2xx responses while preserving other HTTP refusals. Retain the existing '
               'deterministic probe payload, including max_tokens=1, top_k=1, seed=4242 and '
               'configured model, and add cache_prompt=False. Decode the returned JSON bytes: a '
               'successful response uses usage.prompt_tokens; HTTP 400 is eligible only when '
               'error.type is exceed_context_size_error, and then uses error.n_prompt_tokens. '
               'Require a positive integer count. Raise FeasibilityError for an absent socket, '
               'malformed JSON, other HTTP 400 or missing/invalid count; cache only successful '
               'counts. Preserve non-usage tokenization. Change only this source file; tests stub '
               'transport and use no model or external service. This task grades the probe path, '
               'not historical result-schema migration.',
  'test_names': ['ModelFeasibilityTests.test_usage_probe_token_counter_is_cached_and_uses_fixed_decoder',
                 'ModelFeasibilityTests.test_usage_probe_accepts_exact_count_from_context_error'],
  'expected_tests': 2,
  'base_sha256': '0b9d9f6d321f9ec5312e271946b0abfe51987b8edb9a7e0d51cc00e6c8cf3f7b',
  'reference_sha256': '348c8cfb3fb6d98acfdca8bdd675bcd0365a0fc330e6407ee0f59f05e1d6c325',
  'hidden_sha256': '6921346d35c2dadecdd2f3f5b0416c1c865eb1217d12dc0f8a74030593669899',
  'projection_sha256': 'a3bb385c23a6c8cdf446b2bc70abf7b9e3f5a7bce00ff4affd0d2a7c311e1189'},
 {'id': 'dev-017',
  'split': 'development',
  'base': '360d814bbb19ff3114cf514cba0a7c326888e0bc',
  'reference': 'f0c5ba524372ca8537640f5fd18aeaa0050cb783',
  'source': 'scripts/path_platform_conformance.py',
  'hidden_test': 'tests/test_path_platform_conformance.py',
  'statement': 'Repair conformance archive permissions in scripts/path_platform_conformance.py. '
               'Add make_container_readable(root: Path) -> None, traversing root and all '
               'descendants. Refuse symlinks and non-directory/non-regular entries with '
               'PathPlatformConformanceError. Set every directory to mode 0755. Set regular files '
               'with any existing execute bit to 0755 and all other regular files to 0644, '
               'regardless of the current umask or group-write bit. Call this helper after the '
               'existing data-filtered extraction in git_archive. Keep content bytes and the rest '
               'of archive validation unchanged. Edit only this source file. The frozen regression '
               'grades the helper on a synthetic directory tree, not a container or platform '
               'qualification.',
  'test_names': ['PathPlatformConformanceTests.test_container_source_modes_are_independent_of_restrictive_umask'],
  'expected_tests': 1,
  'base_sha256': '020030b30129fabd41bbcfc2100c0fbc2c77e950e64e9c595fedd727854ce11b',
  'reference_sha256': '58a944dcc0d48e47cda2251f662fb2a671b7ef3356db48f50cc7e06de7760a27',
  'hidden_sha256': 'ab25d4eca2272b3c217fd615942aaecc80203d6eada124cf24685bd4dbb18573',
  'projection_sha256': 'ecec6099848ea9ded0eac9890831aecce80da38c3efcf52732e1de1074858d59'},
 {'id': 'dev-018',
  'split': 'development',
  'base': 'e41056d023c81aff372f5acd17cffd84eb9ed30a',
  'reference': 'cff66354016e5d2bb058a277d1150c2033c73eb3',
  'source': 'scripts/story_13_4_dispatch_preflight_evidence.py',
  'hidden_test': 'tests/test_story_13_4_dispatch_preflight_evidence.py',
  'statement': 'Keep dispatch-preflight logs public-safe in '
               'scripts/story_13_4_dispatch_preflight_evidence.py. capture() replaces only the '
               'checkout ROOT in command stdout with <repository-root>, preserving the existing '
               'command order, command headers, early failure return code and newline behavior. '
               'validate_raw still checks its existing markers and prohibited failure text, and '
               'additionally returns the exact diagnostic dispatch-preflight results contain a '
               'private host path if the raw text contains ROOT, Path.home(), or the home alias '
               'with an initial /var removed when the home starts /var/home/. Never silently '
               'redact unrelated home content into a publishable log. Expose ROOT_PLACEHOLDER with '
               'the exact replacement string; private_prefixes() returns the distinct private '
               'prefixes longest-first. Change only this source file; the regression uses '
               'synthetic compiler output and a stub subprocess, not actual compilation.',
  'test_names': ['Story134DispatchPreflightEvidenceTests.test_private_host_paths_are_redacted_or_refused'],
  'expected_tests': 1,
  'base_sha256': '4015731d548935c3b0918c9e60f290726a556f848a2499527affed1401ceda30',
  'reference_sha256': '8d161092939c0a39de27b625d96e3b965e6f44b44e216493f2711bad23977c6e',
  'hidden_sha256': '419045284a7c2998cc9887b7d3bcc14de9611dac1f6e84eb9f2cb22d87a03bb7',
  'projection_sha256': 'fb73e03543eb69cef4ec86c65b5a8390a49cf1625667e381f46defc217c3b4dd'},
 {'id': 'dev-019',
  'split': 'development',
  'base': 'c666a1da7fac9250f1abc4389bf741ecaf1fdf9e',
  'reference': 'dc99587fa1940516aaf32de88ac4373df9cb47bc',
  'source': 'scripts/package_lifecycle.py',
  'hidden_test': 'tests/test_package_lifecycle.py',
  'statement': 'Fix extract_deb(package: Path, root: Path) in scripts/package_lifecycle.py so '
               'extraction preserves archive permissions under a restrictive caller umask without '
               'restoring archive ownership. Keep exactly two command calls in order: first [ar, '
               'x, str(package)], then [tar, --extract, --gzip, --file, data.tar.gz, '
               '--no-same-owner, --same-permissions], both with cwd=root. These are string '
               'argument lists passed to the existing command helper, never a shell string. '
               'Preserve its error propagation and the other package paths. Change only this '
               'source file. The frozen regression checks calls using a stub; it does not install '
               'or extract a real package.',
  'test_names': ['PackageLifecycleTests.test_deb_extraction_preserves_manifest_permissions_under_restrictive_umask'],
  'expected_tests': 1,
  'base_sha256': '0e55defb25dbe5af867b772c883e7f25f82240c8a40a6fddef1ebfae1745438f',
  'reference_sha256': '37aa705fc4e5134bd22ea6fb2e858d9933f7b4839555c89da4ff1cb1364b24c7',
  'hidden_sha256': 'd465057c1f1321a48737f0c8746352cf7e7fb0c832b64fe44cb207eb62129ef6',
  'projection_sha256': '206e171f889c0a40263fb410582efd3a08f39d6da3663689825aa98d2bc56234'}]


# Ten separate source changes, frozen before any model attempt.
ADDITIONAL_TASKS = [{'id': 'dev-020',
  'split': 'development',
  'base': '99cc4e1630ea1e658a815e82893f650fe736c884',
  'reference': 'd8557c993838ba54a85dba7a364a9495b6d92496',
  'source': 'scripts/coding_harness_acceptance.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'statement': 'Strengthen scripts/coding_harness_acceptance.py multi-file acceptance. In '
               'verify_case retain the existing exit, filesystem, scenario, terminal and '
               'validation checks, and add checks named "recorded-nine-tool-regression" (the '
               'outcome tool_call_count equals 9) and "mandatory-records-beyond-old-count-limit" '
               '(strictly more than 33 observed rows have type runtime_artifact_verified). They '
               'must participate in the returned passed value. In run_case pass '
               'record_session=True to coding_harness.start only for the multi-file case, false '
               'otherwise. Preserve every other case. The regression uses mocked observations and '
               "validation, not a live runtime; it grades verify_case's record-count refusals, not "
               'run_case integration. Edit only this source file.',
  'test_names': ['CodingHarnessTests.test_multifile_regression_requires_nine_tools_and_recording_beyond_old_limit'],
  'expected_tests': 1,
  'base_sha256': '06549ed0f16d276bd6fd7b327e131c789cd32b8cd7b8b18eb79955f397475190',
  'reference_sha256': '2c7d0fb4e1ce9e29f0f66e82ee715f12f3ca7236ffd45efa16efe5439e67b7c8',
  'hidden_sha256': 'a64d73fce140b238cac05446edd1f906ca907925eca402b0cca23f3c3f189406',
  'projection_sha256': 'a3c6062b968da9653e7b4ee165d35319214591864c695905e07d18fbb91f6975'},
 {'id': 'dev-021',
  'split': 'development',
  'base': 'd8557c993838ba54a85dba7a364a9495b6d92496',
  'reference': 'ad28b5ca7862a483bb4b9bf74c199d5f994ae43f',
  'source': 'scripts/coding_harness_acceptance.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'statement': 'Add read-arguments-correction and read-arguments-denied cases to '
               'scripts/coding_harness_acceptance.py. EXPECTED entries are respectively '
               '(same-named scenario, "repair", True, False, 0, "SUCCESS") and (same-named '
               'scenario, "repair", True, False, 8, "FAILED"); CASE_ROOTS assigns c21 and c22. '
               'Correction expects [" M src/calc.py"], denial expects no changes. Reuse the '
               'protocol/argument rejection checks: exactly one rejection, no '
               'permission_requested, permission_decided, tool_started or tool_completed on its '
               'turn. Correction requires the existing six-tool repair sequence, 8 turns, a '
               'verified artifact and the exact repaired add function already used by other repair '
               'cases. Add "retained-muse-read-arguments": some tool_requested row must name '
               'scripted-invalid-read-paths and arguments_sha256 '
               'aa960a422a134655e7cc9eeebc922e74d9d10883d3fdc1e5721693c9544b5028. Denial requires '
               'no completed tools, 1 turn and "terminal-rejection-code": unresolved_codes exactly '
               '["runtime.proposal.invalid"]. Preserve existing checks and cases; these checks all '
               'affect passed. The frozen test uses synthetic records, checks byte-digest '
               'mismatch, authority leakage, an extra completed tool and missing terminal code; it '
               'runs no model or native worker. Edit only this source file.',
  'test_names': ['CodingHarnessTests.test_read_rejection_requires_exact_bytes_no_authority_and_terminal_denial'],
  'expected_tests': 1,
  'base_sha256': '2c7d0fb4e1ce9e29f0f66e82ee715f12f3ca7236ffd45efa16efe5439e67b7c8',
  'reference_sha256': '0ca8b2f10feb0c12a8d4ab76c6741b5bd430d38842f8b76640960c2f017c7951',
  'hidden_sha256': 'b046e2c8c39f8b21ab0d5935622a63b32f689030ada542b114fb2f06dfa027df',
  'projection_sha256': '64c8383c0cb1fcc8e271ba3b6d78596cb48d456be7680054d642debf67d37943'},
 {'id': 'dev-022',
  'split': 'development',
  'base': 'ef4a05d823ea0987229e2ab389867b15bb4f8282',
  'reference': '7b1953d544c6f37cd11bb2c809db8b0aaa8564b7',
  'source': 'scripts/coding_harness_acceptance.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'statement': 'Extend scripts/coding_harness_acceptance.py with native-command-repair '
               'verification. Add EXPECTED tuple ("native-command-repair", "repair", True, False, '
               '0, "SUCCESS"), unique CASE_ROOTS c20 and expected status [" M src/calc.py"]. '
               'verify_case must retain ordinary exit/filesystem/scenario/terminal checks and add '
               '"exact-tool-order": completed tool_call_ids exactly '
               'scripted-native-ordered-command, scripted-validation-failing, '
               'scripted-inspect-preimage, scripted-repair-patch, scripted-validation-passing, '
               'scripted-git-diff, scripted-git-status. Bind artifact_created IDs to the completed '
               'scripted-native-ordered-command turn only. For '
               '"generic-command-failure-output-verified", require a runtime_artifact_verified row '
               'with such an ID and exact byte_size and payload_sha256 of UTF-8 '
               'json.dumps(sort_keys=True) plus newline for '
               '{schema_version:1,status:"assertion_failed",passed:0,failed:1,skipped:0,duration_ms:1,failed_names:["fixture::add"],artifact_ids:[],retry_count:0,initial_failure_sha256:null}. '
               'Also require "bounded-turns" equal 8, "verified-artifacts" and "exact-repair" '
               'using the existing repaired add text. Every check participates in passed. Preserve '
               'other cases. The regression stubs records and checks wrong-turn and wrong-digest '
               'refusal; it runs no native command. Edit only this source file.',
  'test_names': ['CodingHarnessTests.test_native_command_failure_artifact_must_belong_to_exact_command_turn'],
  'expected_tests': 1,
  'base_sha256': '3e80638936d1a635f2e7de155cc13e602e449a21604d4f5e6e36990dedb90d4d',
  'reference_sha256': '4fd2d7cea615891a966e2f68628755b44f89c3c661163bf728625ab0b6fd9bc6',
  'hidden_sha256': '7e44a1c7ccb758356a48bbd7cde4ced5252fc9109062c9bf9e819821a74e2939',
  'projection_sha256': '4c0fb9c95a7f872237be90069ba034ef94c137de8d051f0a20b1a104ed51ff09'},
 {'id': 'dev-023',
  'split': 'development',
  'base': 'a13fd15e22ad659a6a3e60692caa4b7dea6a7a3e',
  'reference': 'eca3f405ceb198d2e01e2764703a554b44f0b381',
  'source': 'scripts/coding_harness.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'statement': 'Add implementation identity measurement in scripts/coding_harness.py. '
               'file_identity(path: Path) returns {"path": str(path), "bytes": size, "sha256": '
               'lowercase SHA-256}. Hash an opened regular file; use fstat before/after and '
               'compare device, inode, size, mtime_ns and ctime_ns, raising '
               'HarnessError("coding.harness.identity-not-regular") or '
               'HarnessError("coding.harness.identity-changed") as applicable. '
               'implementation_identity() returns head_commit (40-character HEAD), worktree_status '
               '(Git porcelain-v1 including all untracked files), tracked_diff_sha256 (Git diff '
               'HEAD --binary --no-ext-diff bytes), binaries (identities of binary("agentmage"), '
               'binary("agentmage-host"), binary("agentmage-read-only-worker") in that order), and '
               'wrapper (identity of resolved __file__). Git calls run at ROOT, check errors, '
               'capture output, use timeout=30 and --no-optional-locks. This is diagnostics, never '
               'executable admission. The frozen regression grades the identity dictionary with '
               "/usr/bin/true substituted for binaries and the grading snapshot's Git history; not "
               'timing, concurrent mutation, or start() recording. Preserve existing behavior and '
               'edit only this source file.',
  'test_names': ['CodingHarnessTests.test_implementation_identity_pins_source_and_all_three_binaries'],
  'expected_tests': 1,
  'base_sha256': 'a5bd8f32723f357349d1bfdd461b8cac50b05e4b42b102865eb5f7390868c333',
  'reference_sha256': '6c321f3627c2761e9c527a20285e7a2226a7375a3b929f479f67a16b7c56f593',
  'hidden_sha256': '1fe0a081d67fb477ff9e727a75b5f717d86dc30b6cf4c477043bac162842ceed',
  'projection_sha256': '93ac8c84719e5efc7a3210cb8ed0765af91e25d8964cb3199c2ccf52a72c9dee'},
 {'id': 'dev-024',
  'split': 'development',
  'base': 'fe898a96bbea44d025b986997bd030136e178d1e',
  'reference': '0dcd13d04c61d2325504b344a13d493788ea2c26',
  'source': 'scripts/coding_harness.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'statement': 'Add bounded candidate observation helpers in scripts/coding_harness.py. '
               'gpu_memory() invokes /usr/bin/nvidia-smi with --query-gpu=memory.used,memory.free '
               '--format=csv,noheader,nounits, check=True, capture_output=True, text=True and '
               'timeout=10; returns (used_mib, free_mib). Require exactly one stdout line '
               'containing two comma-separated integers. Refuse multiple or absent lines with '
               'HarnessError("coding.harness.candidate.single-gpu-required"); invalid counts with '
               'HarnessError("coding.harness.candidate.gpu-observation-invalid"). Preserve normal '
               'subprocess exceptions. CandidateResourceGuard(descriptor, profile, resources, '
               'used, free) retains those inputs and baselines, starts error=None and '
               'peak_total_gpu_used_mib_sampled=used. sample() observes gpu_memory(), updates that '
               'peak, and when used exceeds profile["maximum_total_vram_used_mib"], sets '
               'error="coding.harness.candidate.gpu-headroom-exceeded". On HarnessError, OSError '
               'or subprocess.SubprocessError set '
               'error="coding.harness.candidate.gpu-observation-failed"; return whether error is '
               'None. sample never signals or kills processes; its caller owns cancellation. '
               'Preserve other harness behavior. The two frozen regressions mock every GPU '
               'observation: they grade valid/malformed numeric output and the over-limit '
               'error/peak with descriptor=-1; no actual GPU, locking, cancellation or complete '
               'resource-admission claim. Edit only this source file.',
  'test_names': ['CodingHarnessTests.test_gpu_observation_requires_one_exact_numeric_device',
                 'CodingHarnessTests.test_candidate_guard_cancels_only_its_run_on_sampled_pressure'],
  'expected_tests': 2,
  'base_sha256': '66f6a169d4af995ce913e9111aec887b605cba85fba7ebad237f1d7e98264299',
  'reference_sha256': '23a57fcb5ec8021faed91c4904d9dc40b57ef4ad6a9fadb647cc4a918fc12167',
  'hidden_sha256': '4bd157837ac7a86f352bac075c19e600004d70f9fc93c36b2289f7f2ca725a96',
  'projection_sha256': '93e9316235d4a50c1e5489130de2f7071bdbe7ecc63fcdba0aef5f5b87bf00d1'},
 {'id': 'dev-025',
  'split': 'development',
  'base': 'ea16322f4cc232ababdaa1c839aceeef6bd0a2d8',
  'reference': '868234916b99f2491b0d9249a2e9366f1736962e',
  'source': 'scripts/coding_harness.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'statement': 'Extend scripts/coding_harness.py setup(base, fixture="repair") with "new-file" and '
               '"multi-file" fixtures, refusing other fixture names with '
               'HarnessError("coding.harness.fixture-denied"). Keep existing private directories, '
               'marker, branch, local Git identity, clean commit and diagnosis. For new-file do '
               'not create src/calc.py initially; retain a validation script requiring add. For '
               'multi-file create src/calc.py defining broken_add and src/subtract.py defining '
               'broken_subtract; validation imports add and subtract, checks add(2,3)==5 and '
               'subtract(7,2)==5, emits the existing schema_version=1 test-result JSON shape '
               '(failed_names fixture::add/fixture::subtract, counts for two tests) and exits '
               'nonzero until repaired. Track only actual created source files, the marker and '
               'validation script. Preserve the default repair fixture. The frozen regression '
               'grades absence/presence and failing validation using synthetic repositories and '
               'substituted executable paths; no real agent run or approval. Edit only this source '
               'file.',
  'test_names': ['CodingHarnessTests.test_setup_builds_exact_new_and_multi_file_fixtures'],
  'expected_tests': 1,
  'base_sha256': '44ce906b2cfee5c9cbe263616ed1baa4d6993fbd3a1057c9e059e0796408d4da',
  'reference_sha256': '8cf1a6fcbb4dfdc3a7f376d9cd30e7d94c7c8cc7a033141ebe592bd79ba2418a',
  'hidden_sha256': '66e07dceb2db4f5debdb4b5bc927414f719667f865be5a738365786da8734fc7',
  'projection_sha256': '904222bc45becb3896d25f07a32875fad862aee7d544d6eb9775c69a0d55bad6'},
 {'id': 'dev-026',
  'split': 'development',
  'base': 'b7964ea2e8afceb32039c27aa3ce3f0b14d2c59f',
  'reference': 'be002e26b79ef68a2b6cbf1897d9abd9a733fbc3',
  'source': 'scripts/sprint_48_evidence.py',
  'hidden_test': 'tests/test_sprint_48_evidence.py',
  'statement': 'Repair scripts/sprint_48_evidence.py run_commands() finite collection deadlines. '
               'Use timeout=2700 seconds only for the existing command whose identifier is '
               '"documentation-gate"; every other command stays at 1800. Preserve command order '
               'and count, resolve argv[0] with shutil.which, and pass a tuple (resolved '
               'executable, *argv[1:]) with cwd=ROOT, check=False, capture_output=True to '
               'subprocess.run. Preserve unavailable-executable records and output digests/skip '
               'accounting. subprocess.TimeoutExpired must propagate through main() without '
               'writing a passing report. This is a collector deadline, not a change to product '
               'acceptance timing or test thresholds. The two frozen tests stub all commands and '
               'timeout; no long gate or compiler runs. Edit only this source file.',
  'test_names': ['Sprint48EvidenceTests.test_only_full_documentation_has_the_longer_finite_collection_deadline',
                 'Sprint48EvidenceTests.test_collection_timeout_cannot_write_a_passing_report'],
  'expected_tests': 2,
  'base_sha256': '68b9543f0c4233ccaaabcabe924210975b8155fc1dc3d75d48d1e675740671d0',
  'reference_sha256': '621d28f0fe8d1b0ce2eeb81c5aaf845f0eb2806ab5a944d81bca6273fcd5f9fa',
  'hidden_sha256': 'e946c10fa78d1d869325497670f7181bc5fb914f461929189acafdb872851382',
  'projection_sha256': '18a13dbdcf1192e2024eba9ff1125d8194ce19b09b288f7199f9c91071e9fe31'},
 {'id': 'dev-027',
  'split': 'development',
  'base': '025e3fd5cbb9cac0d4776d16e8cfffda959cfa8d',
  'reference': '709f8d7a13bcd467317c0598fc5ba939c4e78892',
  'source': 'scripts/coding_harness.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'statement': 'Add closed extension CLI argument routing in scripts/coding_harness.py. parser() '
               'gains extension --root PATH, exactly one of --trust PATH, --distrust DIGEST, '
               '--install PATH, --uninstall ID, --revocations PATH or --list, plus optional '
               '--workspace and --allow-license. extension_arguments(namespace) returns a list '
               'starting with the corresponding --extension-* option and value (list has none), '
               'install additionally --extension-allow-license LICENSE; append '
               '["--extension-workspace", workspace] when present. Non-list operations require '
               'workspace; labels follow existing MEMORY_LABEL, package/license IDs match '
               '[A-Za-z0-9._:-]{1,128}, key digests match lowercase [0-9a-f]{64}, file/directory '
               'paths must be absolute. An allow-license value is required exactly for install. '
               'Raise HarnessError for invalid workspace '
               '(coding.harness.extension-workspace-denied), missing scope or misplaced license '
               '(coding.harness.extension-arguments-denied), relative trust/revocations paths '
               '(coding.harness.extension-path-denied), bad install path/license '
               '(coding.harness.extension-install-denied), bad digest '
               '(coding.harness.extension-key-denied), or bad uninstall ID '
               '(coding.harness.extension-package-denied). Parser mutual exclusion failures remain '
               'SystemExit. Preserve other commands. The frozen regression grades argument lists '
               'and refusals only, with no catalog, fixture-copy or extension execution. Edit only '
               'this source file.',
  'test_names': ['CodingHarnessTests.test_extension_operations_reach_the_catalog_host_in_their_closed_forms'],
  'expected_tests': 1,
  'base_sha256': '0625e783a891fddea8c05e88066be41dc3a5ced0fe7f9bd15692091e1f8eb587',
  'reference_sha256': '92f3b5246e10e1b038031d0e67b83c2e927d930d87e5a28b188044ae189202f1',
  'hidden_sha256': 'fb2cb4b6b41c99ed7c8cc25274e8ac78d2fde6299a5eaf6fb09517d005988543',
  'projection_sha256': '99a9f21f0f219573432daac2f21c64d4544b5173e7174d69b80bf3178eae76e8'},
 {'id': 'dev-028',
  'split': 'development',
  'base': 'e5f3491047fde65c665329696bc011c34e786e54',
  'reference': '69c99f66a110496c7599111b03b6c45e0fab00fc',
  'source': 'scripts/coding_harness.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'statement': 'Extend scripts/coding_harness.py diagnostics options. export_selection(value) '
               'keeps existing CHAIN:FROM:TO validation (positive canonical integers, ordered, TO '
               '<= 512, argparse.ArgumentTypeError on refusal), and additionally accepts chain '
               '"routes" alongside "effects" and "job-control". Add start --support-bundle parsed '
               'as Path. support_bundle_directory(value: Path) returns the existing absolute '
               'directory unchanged, otherwise raises '
               'HarnessError("coding.harness.support-bundle-directory-denied"); privacy/ownership '
               "and export approval remain the actual CLI's duty. Pass that validated directory to "
               'the CLI as --support-bundle when supplied, preserving existing job control: stop '
               'sends SIGINT, pause SIGUSR1, resume SIGUSR2 through signal_running_cli, unknown '
               'control raises KeyError and no live run raises HarnessError. Preserve start '
               '--suspend-resume-probe and --action-history-export behavior. The frozen regression '
               'grades parser/helper refusals, routes and mocked signal dispatch; no bundle is '
               'written and start propagation/approval is not exercised. Edit only this source '
               'file.',
  'test_names': ['CodingHarnessTests.test_each_job_control_sends_its_own_signal_to_the_exact_cli'],
  'expected_tests': 1,
  'base_sha256': 'a93c667ac03a0ed16b7082b40c49a68a1ce9ac848d95818d144494fc48d0668c',
  'reference_sha256': '1c29f82ef799c06fae0e1f2dea164443f5c96f8a47abd0a2a9f2e3c40d1f3988',
  'hidden_sha256': '1eeefff24ae644166ba7937b5acb1f7bbdb7a4dee31b81207953edbc651697b7',
  'projection_sha256': 'f1a434687b1bcd067d39a62551e1f466491e858f02ed3a3671042f857c21b453'},
 {'id': 'dev-029',
  'split': 'development',
  'base': '8a3a341ea45eedcc5d2ef3cd66ee514fab3bb3db',
  'reference': '3709eabe0ddc4ed04027e982dee9f9e204a53790',
  'source': 'scripts/coding_harness_acceptance.py',
  'hidden_test': 'tests/test_coding_harness.py',
  'statement': 'Add suspend_resume_checks(exit_code, observed_rows, observed, status) to '
               'scripts/coding_harness_acceptance.py. Return booleans: "exit" for exit_code==0; '
               '"terminal" for nonempty observed_rows whose last state is SUCCESS; "one-start" for '
               'exactly one kind.event=run_started before the outcome; "checkpoint" for a '
               'checkpoint_committed event there; "suspended-once" for exactly one run_suspended '
               'notice; "continued-from-it" for exactly one run_continued notice with the same '
               'checkpoint_id as the first suspension and occurring later in observed; "answers" '
               'for all job_control_answer notices having (action, decision.decision) exactly '
               '[("suspend","applied"),("resume","applied")]; "job-completed" for exactly one '
               'job_state notice, available=True, job.phase completed and controls actions exactly '
               '["suspend","resume"]; "filesystem" for status exactly [" M src/calc.py"]. Missing '
               'observations fail the relevant check. Preserve other acceptance behavior. The '
               'frozen regression uses synthetic notices and grades missing '
               'suspension/continuation, refused resume, duplicate start and filesystem mismatch; '
               'it does not grade checkpoint digest/sequence equality, an actual runtime or '
               'durable resumption. Edit only this source file.',
  'test_names': ['CodingHarnessTests.test_the_suspend_resume_case_requires_one_suspension_and_one_continuation'],
  'expected_tests': 1,
  'base_sha256': '0ca8b2f10feb0c12a8d4ab76c6741b5bd430d38842f8b76640960c2f017c7951',
  'reference_sha256': '87157ab58ce1c24f4c25da006ce728d7742b2d3af16e2a446cbf77d6b4fa327f',
  'hidden_sha256': '87eb84b00a8b373915c39958fa4aa6fec66c40e49b9e40a30dbe9a2ffad2489c',
  'projection_sha256': '0ea481cd3ecc13be0b11985db1bfba823a81ad1978266c842de0d7ffdfe5483c'}]

TASKS += ADDITIONAL_TASKS

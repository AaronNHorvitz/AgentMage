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

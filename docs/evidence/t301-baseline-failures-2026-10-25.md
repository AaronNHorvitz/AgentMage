# T-301: Baseline failure identities, 2026-10-25

The workspace all-target test command exited 101. The current 72 `FAILED` test
identities equal the union of the retained host and Linux baseline failure
identities in these unchanged repository-relative logs:

- `artifacts/verification-logs/standalone-evidence-host-2026-10-04/batch35-source-2/11.log`
- `artifacts/verification-logs/standalone-evidence-host-2026-10-04/batch35-source-2/07.log`

These are failed tests, not exclusions from the fast gate. This is an identity
comparison, not proof that every failure has one cause. No runtime or sandbox
boundary was changed to obtain a passing result. The [baseline](t301-baseline-2026-10-25.md)
records the command, counts, timing and limits. Private raw output is not copied.

## Host library (53)

- `linux_coding_runtime::tests::a_write_outside_the_recipe_plan_is_refused_before_any_grant_or_approval`
- `linux_coding_runtime::tests::command_git_validation_observers_preserve_consumption_failure_and_reopen`
- `linux_coding_runtime::tests::durable_coding_run_persists_continuation_artifact_and_checkpoint`
- `linux_coding_runtime::tests::read_cancellation_after_consumption_retains_exact_receipt_and_reopens`
- `linux_coding_runtime::tests::read_precancel_receipt_precedes_original_acknowledgements_on_reopen`
- `linux_coding_runtime::tests::retained_muse_missing_read_is_rejected_before_authority_and_not_path_absence_evidence`
- `linux_coding_runtime::tests::s_048_mvp_e2e_controlled_create_test_git_and_verify_path`
- `linux_coding_runtime::tests::s_048_mvp_e2e_denial_and_user_cancellation_start_no_effect`
- `linux_coding_runtime::tests::s_048_mvp_e2e_failed_validation_requires_a_repair_before_success`
- `linux_coding_runtime::tests::s_048_mvp_e2e_repository_exploration_finishes_as_verified_no_op`
- `linux_coding_runtime::tests::story_22_1_long_native_session_blocks_drift_and_preserves_exact_history`
- `linux_coding_runtime::tests::story_22_2_linux_generated_file_is_published_and_checkpoint_bound`
- `linux_coding_runtime::tests::story_22_2_linux_restart_recovers_large_terminal_model_artifact`
- `linux_coding_runtime::tests::story_22_2_linux_restart_rejects_lost_continuation_without_replaying_the_tool`
- `linux_coding_runtime::tests::story_22_2_linux_restart_restores_checkpoint_without_replaying_the_tool`
- `linux_coding_runtime::tests::story_22_2_linux_restart_restores_large_command_and_test_artifacts_without_replay`
- `linux_coding_runtime::tests::story_39_1_native_write_checkpoint_process_matrix_recovers_without_replay`
- `linux_coding_runtime::tests::story_48_2_fake_model_completes_real_git_patch_test_git_and_verify_path`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_applies_one_exact_structured_patch`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_asks_then_derives_one_exact_operation_grant`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_creates_one_exact_absent_file`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_denial_and_create_collision_are_inert`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_executes_one_exact_bounded_command_after_approval`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_executes_one_kernel_checked_git_inspection`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_normalizes_targeted_validation_after_one_granted_command`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_persists_stale_patch_invalidation_without_overwrite`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_refuses_a_selection_over_a_changed_file`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_rejects_substitution_staleness_and_unbudgeted_actions`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_user_denial_is_inert_and_non_replayable`
- `linux_coding_runtime::tests::story_48_2_linux_runtime_writes_only_the_selected_hunks_of_a_narrowed_patch`
- `linux_coding_runtime::tests::story_48_2_native_command_json_order_preserves_exact_approved_call`
- `linux_coding_runtime::tests::story_48_2_native_command_reserialization_after_approval_remains_denied`
- `linux_coding_runtime::tests::story_50_2_and_sprint_95_child_caller_use_the_real_no_op_runtime_path`
- `linux_coding_runtime::tests::story_50_2_exact_path_preauthorization_uses_kernel_write_grant_and_unknown_path_asks`
- `linux_coding_runtime::tests::story_50_2_exact_registered_command_preauthorization_uses_fresh_grant`
- `linux_coding_runtime::tests::story_50_2_session_preauthorization_issues_fresh_grants_and_exhausts_exact_budget`
- `linux_coding_runtime::tests::story_50_2_session_preauthorization_revocation_and_expiry_fail_closed`
- `linux_coding_runtime::tests::story_50_2_workflow_caller_waits_for_user_and_denial_starts_no_effect`
- `linux_coding_runtime::tests::uncertain_command_and_validation_never_publish_completion_artifacts_or_replay`
- `linux_coding_runtime::tests::uncertain_command_event_is_atomically_published_and_reopens`
- `linux_coding_runtime::tests::uncertain_git_event_is_atomically_published_and_reopens`
- `linux_coding_runtime::tests::uncertain_git_never_publishes_observations_or_replays_consumed_authority`
- `linux_read::tests::absent_read_and_tool_approvals_do_not_require_a_clock_sample`
- `linux_read::tests::authenticated_host_frontier_review_renders_once_and_never_delivers`
- `linux_read::tests::diagnostic_export_preview_and_approval_use_the_authenticated_workflow`
- `linux_read::tests::doctor_uses_the_same_workflow_and_reports_unproved_components_unavailable`
- `linux_read::tests::generic_tool_preview_binds_every_exact_object_and_cancels_without_receipt`
- `linux_read::tests::generic_tool_preview_rejects_duplicate_unscoped_and_wrong_kind_projections`
- `linux_read::tests::handoff_source_drift_and_missing_acknowledgment_fail_closed`
- `linux_read::tests::handoff_transport_requires_current_review_and_never_delivers`
- `linux_read::tests::model_picker_transport_is_unavailable_until_verified_and_never_falls_back`
- `linux_read::tests::shared_runtime_is_unavailable_until_explicitly_composed_and_then_host_framed`
- `linux_repository_map::tests::live_projection_is_git_aware_policy_aware_and_non_following`

## Linux library (19)

- `command_runner::tests::manifest_binds_every_registered_executable_digest`
- `command_runner::tests::task_ceilings_below_the_linux_minimum_fail_before_launch`
- `local_commit::tests::native_candidate_tree_uses_only_owned_index_and_preserves_user_state`
- `local_commit::tests::native_signed_commit_verifies_exact_content_and_only_updates_task_branch`
- `repository_safety::tests::compare_and_swap_updates_only_one_unchecked_branch_and_stale_old_fails`
- `repository_safety::tests::concurrent_user_changes_survive_worktree_removal_and_cleanup`
- `repository_safety::tests::exact_worktree_lifecycle_preserves_active_checkout`
- `repository_safety::tests::interrupted_branch_fast_forward_preserves_every_user_artifact`
- `repository_safety::tests::interrupted_worktree_create_preserves_every_user_artifact`
- `repository_safety::tests::manifest_is_minimized_stable_and_detects_untracked_and_hazards`
- `repository_safety::tests::network_clone_fetch_and_unmapped_tokens_never_launch_locally`
- `repository_safety::tests::repository_inventory_cancellation_fails_before_git_launch`
- `repository_safety::tests::repository_inventory_preserves_exact_git_states_and_nonregular_hints`
- `repository_safety::tests::worktree_lifecycle_preserves_a_detached_active_checkout`
- `repository_safety::tests::worktree_lifecycle_preserves_a_dirty_active_checkout`
- `repository_safety::tests::worktree_removal_fails_closed_when_the_owned_directory_is_missing`
- `repository_safety::tests::worktree_removal_fails_closed_when_the_owned_directory_is_renamed`
- `secret_service::tests::secret_bytes_never_enter_process_arguments_or_environment`
- `secret_service::tests::substituted_secret_client_digest_fails_before_service_contact`

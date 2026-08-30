from __future__ import annotations

import copy
import unittest

from scripts.schema_evolution_plan import load_plan, validate_plan


class SchemaEvolutionPlanTests(unittest.TestCase):
    def setUp(self) -> None:
        self.plan = load_plan()

    def test_canonical_plan_passes(self) -> None:
        self.assertEqual(validate_plan(self.plan), [])

    def test_root_and_section_fields_are_closed(self) -> None:
        root = copy.deepcopy(self.plan)
        root["automatic_compatibility"] = True
        self.assertIn("plan field closure changed", validate_plan(root))
        section = copy.deepcopy(self.plan)
        section["unsupported_clients"]["best_effort_fallback"] = True
        self.assertIn("unsupported_clients field closure changed", validate_plan(section))

    def test_authority_path_and_digest_are_bound(self) -> None:
        digest = copy.deepcopy(self.plan)
        digest["authorities"][0]["sha256"] = "0" * 64
        self.assertIn("authority[0] digest differs from current authority", validate_plan(digest))
        path = copy.deepcopy(self.plan)
        path["authorities"][0]["path"] = "../outside"
        self.assertIn("authority[0] path is not repository-relative", validate_plan(path))

    def test_additive_record_cannot_be_changed_in_place(self) -> None:
        for key in (
            "new_optional_field_in_place_allowed",
            "new_required_field_in_place_allowed",
            "semantic_reinterpretation_in_place_allowed",
        ):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.plan)
                changed["additive_records"][key] = True
                self.assertIn(f"additive_records.{key} must remain false", validate_plan(changed))

    def test_migration_remains_pure_and_without_effect_authority(self) -> None:
        impure = copy.deepcopy(self.plan)
        impure["additive_records"]["migration_is_pure"] = False
        self.assertIn("additive_records.migration_is_pure must remain true", validate_plan(impure))
        effect = copy.deepcopy(self.plan)
        effect["additive_records"]["migration_effect_authority"] = "workspace-write"
        self.assertIn("record migration gained effect authority", validate_plan(effect))

    def test_unknown_events_stop_before_the_event(self) -> None:
        for key in ("unknown_event_type", "unknown_event_version"):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.plan)
                changed["event_compatibility"][key] = "skip-and-continue"
                self.assertIn(
                    f"event_compatibility.{key} must stop before the event",
                    validate_plan(changed),
                )

    def test_events_cannot_replay_effects_or_reinterpret_terminal_state(self) -> None:
        for key in ("effect_replay_allowed", "terminal_reinterpretation_allowed"):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.plan)
                changed["event_compatibility"][key] = True
                self.assertIn(f"event_compatibility.{key} must remain false", validate_plan(changed))

    def test_cache_key_and_invalidation_inputs_are_complete(self) -> None:
        key = copy.deepcopy(self.plan)
        key["cache_invalidation"]["key_inputs"].remove("policy-digest")
        self.assertIn("cache key inputs are incomplete", validate_plan(key))
        trigger = copy.deepcopy(self.plan)
        trigger["cache_invalidation"]["invalidation_triggers"].remove("component-revocation")
        self.assertIn("cache invalidation triggers are incomplete", validate_plan(trigger))

    def test_stale_cache_fallback_is_impossible(self) -> None:
        stale = copy.deepcopy(self.plan)
        stale["cache_invalidation"]["stale_read_policy"] = "serve-while-revalidate"
        self.assertIn("stale cache content could be served", validate_plan(stale))
        rebuild = copy.deepcopy(self.plan)
        rebuild["cache_invalidation"]["rebuild_failure"] = "serve-prior-generation"
        self.assertIn("cache rebuild failure gained stale fallback", validate_plan(rebuild))

    def test_unsupported_client_refusal_precedes_writes_and_effects(self) -> None:
        for key in ("before_effect_required", "before_persistent_session_write_required"):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.plan)
                changed["unsupported_clients"][key] = False
                self.assertIn(f"unsupported_clients.{key} must remain true", validate_plan(changed))

    def test_unsupported_client_has_no_silent_or_legacy_fallback(self) -> None:
        for key in (
            "field_dropping_allowed",
            "silent_downgrade_allowed",
            "legacy_fallback_allowed",
            "read_only_fallback_allowed",
        ):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.plan)
                changed["unsupported_clients"][key] = True
                self.assertIn(f"unsupported_clients.{key} must remain false", validate_plan(changed))

    def test_store_migration_cannot_add_a_second_store_or_overwrite_user_data(self) -> None:
        for key in ("second_store_allowed", "later_user_data_overwrite_allowed"):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.plan)
                changed["store_migration"][key] = True
                self.assertIn(f"store_migration.{key} must remain false", validate_plan(changed))

    def test_store_migration_retains_recovery_states(self) -> None:
        for state in ("backup-verified", "candidate-verified", "activation-committed", "recovery-required"):
            with self.subTest(state=state):
                changed = copy.deepcopy(self.plan)
                changed["store_migration"]["states"].remove(state)
                self.assertIn(f"store migration state missing: {state}", validate_plan(changed))

    def test_unreadable_downgrade_blocks_and_preserves_current_store(self) -> None:
        changed = copy.deepcopy(self.plan)
        changed["downgrade_behavior"]["unreadable_store_result"] = "restore-old-backup"
        self.assertIn(
            "unreadable downgrade target no longer preserves current store",
            validate_plan(changed),
        )

    def test_downgrade_cannot_reverse_restore_overwrite_or_trigger_remotely(self) -> None:
        for key in (
            "automatic_reverse_migration_allowed",
            "blind_backup_restore_allowed",
            "later_user_data_overwrite_allowed",
            "remote_trigger_allowed",
            "silent_rollback_allowed",
        ):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.plan)
                changed["downgrade_behavior"][key] = True
                self.assertIn(f"downgrade_behavior.{key} must remain false", validate_plan(changed))

    def test_required_campaign_and_implementation_truth_cannot_be_widened(self) -> None:
        campaign = copy.deepcopy(self.plan)
        campaign["required_test_cases"].pop()
        self.assertIn(
            "required_test_cases must retain its complete ordered inventory",
            validate_plan(campaign),
        )
        truth = copy.deepcopy(self.plan)
        truth["implementation_truth"]["migration_runtime_implemented"] = True
        self.assertIn("implementation truth was widened", validate_plan(truth))


if __name__ == "__main__":
    unittest.main()

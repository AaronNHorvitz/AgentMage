from __future__ import annotations

import copy
import unittest

from scripts import artifact_evaluation_fuzz_registration as registration


def reseal(value: dict[str, object]) -> dict[str, object]:
    value["registry_sha256"] = registration.ZERO_SHA256
    return registration.sealed(value)


class ArtifactEvaluationFuzzRegistrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.registry = registration.read_json(registration.REGISTRY_PATH)

    def test_checked_extension_registry_is_current(self) -> None:
        self.assertEqual(registration.check(), [])
        self.assertTrue(registration.valid_hash(self.registry))
        self.assertEqual(self.registry, registration.build_registry())

    def test_all_eight_boundaries_map_to_existing_targets(self) -> None:
        parent = registration.read_json(registration.PARENT_REGISTRY_PATH)
        parent_ids = {item["target_id"] for item in parent["targets"]}
        self.assertEqual(self.registry["registration_count"], 8)
        self.assertEqual(
            [item["registration_id"] for item in self.registry["registrations"]],
            [item[0] for item in registration.REGISTRATION_SPECS],
        )
        self.assertTrue(
            all(
                set(item["parent_target_ids"]) <= parent_ids
                for item in self.registry["registrations"]
            )
        )

    def test_every_seed_and_policy_input_is_hash_bound(self) -> None:
        self.assertEqual(self.registry["input_count"], len(registration.INPUT_PATHS))
        self.assertTrue(all(len(item["sha256"]) == 64 for item in self.registry["inputs"]))
        self.assertTrue(
            all(
                all(len(seed["sha256"]) == 64 for seed in item["seed_inputs"])
                for item in self.registry["registrations"]
            )
        )

    def test_fixed_seed_oracles_cover_parser_provenance_and_workflow_faults(self) -> None:
        summary = self.registry["deterministic_oracle_summary"]
        self.assertEqual(summary["oracle_count"], 8)
        self.assertEqual(summary["passing_oracle_count"], 8)
        self.assertEqual(summary["seed_case_observation_count"], 153)
        self.assertFalse(summary["fixed_seed_oracles_are_real_fuzzing"])
        self.assertEqual(summary["sanitizer_execution_claim"], "none")
        self.assertEqual(summary["coverage_claim"], "none")

    def test_decision_0025_and_every_release_blocker_remain_open(self) -> None:
        manual = self.registry["manual_campaign"]
        self.assertEqual(manual, registration.MANUAL_CAMPAIGN_CONTRACT)
        self.assertFalse(manual["real_fuzz_engine_executed"])
        self.assertEqual(
            manual["blocking_gates"],
            ["affected-RV-15", "Sprint-166", "G-GA"],
        )

    def test_missing_registration_or_parent_target_mutation_blocks_when_resealed(self) -> None:
        missing = copy.deepcopy(self.registry)
        missing["registrations"].pop()
        missing["registration_count"] -= 1
        parent = copy.deepcopy(self.registry)
        parent["registrations"][0]["parent_target_ids"] = ["FT-UNKNOWN-001"]
        self.assertTrue(registration.validate_registry(reseal(missing)))
        self.assertTrue(registration.validate_registry(reseal(parent)))

    def test_false_manual_fuzz_or_sanitizer_claim_blocks_when_resealed(self) -> None:
        executed = copy.deepcopy(self.registry)
        executed["manual_campaign"]["real_fuzz_engine_executed"] = True
        sanitizer = copy.deepcopy(self.registry)
        sanitizer["deterministic_oracle_summary"]["sanitizer_execution_claim"] = "pass"
        self.assertTrue(registration.validate_registry(reseal(executed)))
        self.assertTrue(registration.validate_registry(reseal(sanitizer)))

    def test_fault_oracle_and_side_effect_mutations_block_when_resealed(self) -> None:
        oracle = copy.deepcopy(self.registry)
        oracle["registrations"][0]["deterministic_fault_oracle"][
            "non_success_case_count"
        ] = 0
        effect = copy.deepcopy(self.registry)
        effect["registrations"][0]["network_calls"] = 1
        self.assertTrue(registration.validate_registry(reseal(oracle)))
        self.assertTrue(registration.validate_registry(reseal(effect)))


if __name__ == "__main__":
    unittest.main()

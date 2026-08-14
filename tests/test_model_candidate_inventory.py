from __future__ import annotations

import copy
import unittest

from scripts import model_candidate_inventory as inventory


class ModelCandidateInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.snapshot = inventory.read_json(inventory.SOURCE_SNAPSHOT)
        cls.normalized = inventory.read_json(inventory.INVENTORY)
        cls.matrix = inventory.read_json(inventory.ROLE_MATRIX)

    def test_frozen_sources_reconcile_without_omission_or_duplicate(self) -> None:
        self.assertEqual(
            inventory.validate(self.snapshot, self.normalized, self.matrix), []
        )
        self.assertGreaterEqual(self.snapshot["counts"]["google"], 300)
        self.assertEqual(self.snapshot["counts"]["meta"], 4)
        self.assertEqual(
            self.snapshot["counts"]["total"], len(self.normalized["entries"])
        )
        excluded = [
            item
            for item in self.normalized["entries"]
            if item["disposition"] == "INELIGIBLE"
        ]
        self.assertEqual(len(excluded), 1)
        self.assertIn("collection-adjacent-non-gemma-model", excluded[0]["blockers"])

    def test_omission_duplicate_and_matrix_drift_fail(self) -> None:
        for target, mutate in (
            ("inventory", lambda value: value["entries"].pop()),
            ("inventory", lambda value: value["entries"].append(value["entries"][0])),
            ("matrix", lambda value: value["entries"].pop()),
        ):
            normalized = copy.deepcopy(self.normalized)
            matrix = copy.deepcopy(self.matrix)
            mutate(normalized if target == "inventory" else matrix)
            self.assertTrue(inventory.validate(self.snapshot, normalized, matrix))

    def test_specialists_cannot_escalate_into_coding_planner(self) -> None:
        for marker in inventory.PROTECTED_SPECIALIST_MARKERS:
            matching = [
                item
                for item in self.normalized["entries"]
                if marker in item["repository"].lower()
            ]
            if not matching:
                continue
            for item in matching:
                if "coding_planner" in item["prohibited_roles"]:
                    self.assertNotIn("coding_planner", item["roles"])
        changed = copy.deepcopy(self.normalized)
        specialist = next(
            item for item in changed["entries"] if item["prohibited_roles"]
        )
        specialist["roles"].append("coding_planner")
        self.assertTrue(inventory.validate(self.snapshot, changed, self.matrix))

    def test_hardware_boundaries_are_exact_and_non_acquiring(self) -> None:
        required = {
            "required_disk": 20,
            "required_memory": 32,
            "required_accelerator": 16,
        }
        self.assertEqual(
            inventory.hardware_fit(
                **required,
                available_disk=19,
                available_memory=32,
                available_accelerator=16,
            ),
            "BLOCKED-HARDWARE",
        )
        self.assertEqual(
            inventory.hardware_fit(
                **required,
                available_disk=20,
                available_memory=32,
                available_accelerator=16,
            ),
            "CANDIDATE",
        )
        self.assertEqual(
            inventory.hardware_fit(
                **required,
                available_disk=21,
                available_memory=33,
                available_accelerator=17,
            ),
            "CANDIDATE",
        )
        self.assertTrue(
            all(
                item["hardware_preflight"]["acquisition_started"] is False
                for item in self.normalized["entries"]
            )
        )

    def test_intake_routes_first_party_and_prohibited_sources_exactly(self) -> None:
        self.assertEqual(
            inventory.intake_disposition(
                owner="google",
                revision="a" * 40,
                private=False,
                first_party=True,
            ),
            ("CANDIDATE", []),
        )
        cases = (
            {"owner": "mirror", "revision": "a" * 40, "private": False, "first_party": False},
            {"owner": "google", "revision": "main", "private": False, "first_party": True},
            {
                "owner": "google",
                "revision": "a" * 40,
                "private": False,
                "first_party": True,
                "mirrored": True,
            },
            {
                "owner": "google",
                "revision": "a" * 40,
                "private": False,
                "first_party": True,
                "community_converted": True,
            },
        )
        for case in cases:
            disposition, reasons = inventory.intake_disposition(**case)
            self.assertEqual(disposition, "INELIGIBLE")
            self.assertTrue(reasons)

    def test_activation_fallback_and_acquisition_mutations_fail(self) -> None:
        for field in ("enabled", "automatic_fallback", "acquisition_allowed"):
            changed = copy.deepcopy(self.normalized)
            changed["entries"][0][field] = True
            self.assertTrue(inventory.validate(self.snapshot, changed, self.matrix))


if __name__ == "__main__":
    unittest.main()

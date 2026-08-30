from __future__ import annotations

import copy
import io
import unittest
import zipfile

from scripts.engineering_artifact_admission_fixtures import sha256_bytes, validate_archive
from scripts.engineering_context_delivery_fixtures import (
    BLOCKED_SCENARIOS,
    build_suite,
    model_visible_set_sha256,
    validate_suite,
)


class EngineeringContextDeliveryFixtureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.archive, self.entries, self.suite = build_suite()

    def test_every_model_visible_item_reconstructs_exactly_from_receipt_and_archive(self) -> None:
        self.assertEqual(validate_archive(self.archive, self.entries), [])
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            for scenario in self.suite["scenarios"]:
                self.assertEqual(len(scenario["delivery_map"]), len(scenario["receipt"]["delivered"]))
                for mapped, delivered in zip(scenario["delivery_map"], scenario["receipt"]["delivered"], strict=True):
                    content = archive.read(mapped["archive_entry"])
                    self.assertEqual(delivered["sha256"], sha256_bytes(content))
                    self.assertEqual(delivered["range"], {"start_byte": 0, "end_byte_exclusive": len(content)})
                self.assertEqual(
                    scenario["model_visible_set_sha256"],
                    model_visible_set_sha256(scenario["delivery_map"], self.entries),
                )

    def test_required_authoritative_omission_blocks_completion(self) -> None:
        for scenario in self.suite["scenarios"]:
            blocked = scenario["scenario_id"] in BLOCKED_SCENARIOS
            self.assertEqual(scenario["receipt"]["outcome"], "blocked" if blocked else "delivered")
            self.assertEqual(scenario["completion_allowed"], not blocked)
            self.assertEqual(bool(scenario["required_authoritative_artifact_ids"]), blocked)
            self.assertEqual(
                scenario["receipt"]["required_unseen_artifact_ids"],
                scenario["required_authoritative_artifact_ids"],
            )

    def test_summary_and_source_ranges_are_distinct_and_explicit(self) -> None:
        summary = next(item for item in self.suite["scenarios"] if item["scenario_id"] == "summary")
        mapped = summary["delivery_map"][0]
        self.assertEqual(mapped["manifest_disposition"], "summarized")
        self.assertTrue(mapped["source_ranges"])
        self.assertNotEqual(mapped["range"], mapped["source_ranges"][0])
        self.assertTrue(self.entries[mapped["archive_entry"]].startswith(b"Synthetic bounded summary"))

    def test_empty_visible_sets_are_still_exact_reconstructible_sets(self) -> None:
        empty = [scenario for scenario in self.suite["scenarios"] if not scenario["delivery_map"]]
        self.assertGreaterEqual(len(empty), 4)
        for scenario in empty:
            self.assertEqual(scenario["model_visible_set_sha256"], sha256_bytes(b""))
            self.assertEqual(scenario["receipt"]["delivered"], [])

    def test_suite_rejects_byte_receipt_authority_and_completion_mutations(self) -> None:
        self.assertEqual(validate_suite(self.suite, self.archive, self.entries), [])
        mutations = (
            lambda value: value["scenarios"][1]["receipt"]["delivered"][0].update({"sha256": "f" * 64}),
            lambda value: value["scenarios"][0].update({"completion_allowed": True}),
            lambda value: value["scenarios"][4]["receipt"].update({"required_unseen_artifact_ids": []}),
            lambda value: value.update({"product_delivery_claim": "delivered"}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(self.suite)
            mutate(changed)
            self.assertTrue(validate_suite(changed, self.archive, self.entries))

    def test_corrupt_payload_archive_fails_before_reconstruction(self) -> None:
        corrupted = bytearray(self.archive)
        corrupted[len(corrupted) // 2] ^= 0xFF
        self.assertTrue(validate_archive(bytes(corrupted), self.entries))


if __name__ == "__main__":
    unittest.main()

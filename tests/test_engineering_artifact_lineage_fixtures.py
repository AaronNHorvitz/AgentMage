from __future__ import annotations

import copy
import unittest

from scripts.engineering_artifact_lineage_fixtures import build_manifest, validate_manifest


class EngineeringArtifactLineageFixtureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = build_manifest()

    def test_all_sources_and_ranges_are_terminally_accounted(self) -> None:
        self.assertEqual(self.manifest["source_count"], 26)
        self.assertEqual(self.manifest["range_count"], 21)
        self.assertEqual(self.manifest["range_count"], sum(len(case["ranges"]) for case in self.manifest["cases"]))
        self.assertTrue(all(case["accounting_terminal"] for case in self.manifest["cases"]))

    def test_every_range_has_parser_transformation_warning_structure_provenance_retention_and_disposition(self) -> None:
        required = {
            "parser_id",
            "transformation_id",
            "warning_codes",
            "section_id",
            "provenance_id",
            "retention_id",
            "disposition_id",
        }
        for case in self.manifest["cases"]:
            for range_record in case["ranges"]:
                self.assertTrue(required.issubset(range_record), case["fixture_id"])
                self.assertTrue(range_record["warning_codes"])
                self.assertEqual(range_record["source_range"], range_record["derivative_range"])

    def test_captured_identity_projection_is_exact_but_never_claims_product_parsing(self) -> None:
        for case in self.manifest["cases"]:
            if case["capture_state"] != "captured":
                continue
            identity = case["source_byte_identity"]
            records = case["records"]
            self.assertFalse(case["parser_link"]["product_parser_executed"])
            self.assertEqual(records["transformation"]["input_sha256"], identity["sha256"])
            self.assertEqual(records["transformation"]["output_sha256"], identity["sha256"])
            self.assertEqual(records["sections"][0]["content_sha256"], identity["sha256"])

    def test_uncaptured_sources_have_typed_absence_and_no_derivative_range(self) -> None:
        uncaptured = [case for case in self.manifest["cases"] if case["capture_state"] != "captured"]
        self.assertEqual(len(uncaptured), 5)
        for case in uncaptured:
            self.assertEqual(case["parser_link"]["state"], "not_created")
            self.assertEqual(case["transformation_link"]["state"], "not_created")
            self.assertEqual(case["ranges"], [])
            self.assertIsNone(case["records"]["extraction"])
            self.assertIsNone(case["records"]["transformation"])

    def test_retention_is_memory_only_and_hostile_sources_are_quarantined(self) -> None:
        for case in self.manifest["cases"]:
            retention = case["source_records"]["source_retention"]
            self.assertEqual(retention["retention_class"], "memory_only")
            self.assertIsNone(retention["physical_binding"])
            self.assertEqual(retention["encryption_state"], "not_persisted")
            if case["fixture_id"].startswith("adversarial-"):
                self.assertEqual(retention["lifecycle_state"], "quarantined")

    def test_every_context_disposition_is_terminal_and_model_inert(self) -> None:
        for case in self.manifest["cases"]:
            disposition = case["source_records"]["context_disposition"]
            self.assertTrue(disposition["terminal"])
            self.assertEqual(disposition["ranges"], [])
            self.assertEqual(disposition["token_count"], 0)
            self.assertNotEqual(disposition["disposition"], "included")

    def test_manifest_rejects_each_link_family_or_false_support_claim(self) -> None:
        mutations = (
            lambda value: value["cases"][0].pop("provenance_link"),
            lambda value: value["cases"][0]["ranges"][0].pop("transformation_id"),
            lambda value: value["cases"][0].update({"accounting_terminal": False}),
            lambda value: value.update({"product_parser_support_claim": "supported"}),
            lambda value: value.update({"model_delivery_claim": "delivered"}),
        )
        self.assertEqual(validate_manifest(self.manifest), [])
        for mutate in mutations:
            changed = copy.deepcopy(self.manifest)
            mutate(changed)
            self.assertTrue(validate_manifest(changed))


if __name__ == "__main__":
    unittest.main()

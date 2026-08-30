from __future__ import annotations

import copy
import io
import unittest
import zipfile

from scripts.engineering_artifact_admission_fixtures import sha256_bytes, validate_archive
from scripts.engineering_artifact_adversarial_fixtures import (
    REQUIRED_CATEGORIES,
    expected,
    validate_manifest,
)


class EngineeringArtifactAdversarialFixtureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.archive, self.manifest, self.entries = expected()
        self.cases = {case["category"]: case for case in self.manifest["cases"]}

    def test_every_required_hostile_category_has_one_noncompletion_case(self) -> None:
        self.assertEqual([case["category"] for case in self.manifest["cases"]], list(REQUIRED_CATEGORIES))
        self.assertEqual(len({case["fixture_id"] for case in self.manifest["cases"]}), 10)
        for case in self.manifest["cases"]:
            result = case["expected_result"]
            self.assertTrue(result["terminal"])
            self.assertFalse(result["may_claim_complete"])

    def test_outer_container_is_inert_safe_and_binds_every_hostile_payload(self) -> None:
        self.assertEqual(validate_archive(self.archive, self.entries), [])
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            for case in self.manifest["cases"]:
                content = archive.read(case["archive_entry"])
                self.assertEqual(
                    case["offered_byte_identity"],
                    {"byte_length": len(content), "sha256": sha256_bytes(content)},
                )

    def test_nested_active_relationship_and_traversal_inputs_remain_inert(self) -> None:
        with zipfile.ZipFile(io.BytesIO(self.archive)) as outer:
            active = zipfile.ZipFile(io.BytesIO(outer.read(self.cases["active_content"]["archive_entry"])))
            external = zipfile.ZipFile(io.BytesIO(outer.read(self.cases["external_relationship"]["archive_entry"])))
            traversal = zipfile.ZipFile(io.BytesIO(outer.read(self.cases["archive_traversal"]["archive_entry"])))
            self.assertIn("word/vbaProject.bin", active.namelist())
            self.assertIn(b'TargetMode="External"', external.read("word/_rels/document.xml.rels"))
            self.assertIn("../outside.txt", traversal.namelist())
        for category in ("active_content", "external_relationship", "archive_traversal"):
            result = self.cases[category]["expected_result"]
            self.assertFalse(result["active_content_executed"])
            self.assertFalse(result["external_relationship_fetched"])
            self.assertFalse(result["path_materialized"])

    def test_bomb_exceeds_declared_limits_without_large_outer_corpus(self) -> None:
        case = self.cases["decompression_bomb"]
        with zipfile.ZipFile(io.BytesIO(self.archive)) as outer:
            nested_bytes = outer.read(case["archive_entry"])
        with zipfile.ZipFile(io.BytesIO(nested_bytes)) as nested:
            info = nested.infolist()[0]
            self.assertGreater(info.file_size, case["resource_limit"]["maximum_decompressed_bytes"])
            self.assertGreater(info.file_size / info.compress_size, case["resource_limit"]["maximum_ratio"])
        self.assertLess(len(nested_bytes), 4096)

    def test_mixed_encoding_parser_failure_and_malformed_package_are_real_bytes(self) -> None:
        with zipfile.ZipFile(io.BytesIO(self.archive)) as outer:
            mixed = outer.read(self.cases["mixed_encoding"]["archive_entry"])
            malformed = outer.read(self.cases["malformed_package"]["archive_entry"])
            broken_pdf = outer.read(self.cases["parser_failure"]["archive_entry"])
        self.assertIn(b"\xef\xbb\xbf", mixed)
        self.assertIn(b"\xff\xfe", mixed)
        with self.assertRaises(zipfile.BadZipFile):
            zipfile.ZipFile(io.BytesIO(malformed)).infolist()
        self.assertIn(b"/Pages 999 0 R", broken_pdf)

    def test_replacement_truncation_and_cancellation_keep_exact_bounds_visible(self) -> None:
        replacement = self.cases["concurrent_replacement"]
        self.assertNotEqual(replacement["offered_byte_identity"], replacement["current_byte_identity"])
        self.assertEqual(replacement["records"]["source_artifact"]["capture_state"], "failed")
        self.assertEqual(self.cases["truncation"]["resource_limit"]["maximum_included_bytes"], 128)
        self.assertEqual(self.cases["cancellation"]["resource_limit"]["cancel_after_bytes"], 128)
        self.assertFalse(self.cases["cancellation"]["expected_result"]["may_publish_partial_derivative"])

    def test_manifest_rejects_false_completion_effects_and_security_claims(self) -> None:
        self.assertEqual(validate_manifest(self.manifest, self.archive, self.entries), [])
        for mutation in (
            lambda value: value["cases"][0]["expected_result"].update({"may_claim_complete": True}),
            lambda value: value["cases"][1]["expected_result"].update({"active_content_executed": True}),
            lambda value: value.update({"product_security_claim": "passed"}),
            lambda value: value["required_categories"].pop(),
        ):
            changed = copy.deepcopy(self.manifest)
            mutation(changed)
            self.assertTrue(validate_manifest(changed, self.archive, self.entries))

    def test_corpus_is_deterministic_synthetic_and_network_free(self) -> None:
        self.assertEqual(expected()[0], self.archive)
        self.assertTrue(self.manifest["synthetic_only"])
        self.assertFalse(self.manifest["network_access"])
        self.assertFalse(self.manifest["active_content_execution"])


if __name__ == "__main__":
    unittest.main()

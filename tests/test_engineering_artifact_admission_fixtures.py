from __future__ import annotations

import copy
import io
import json
import unittest
import zipfile

from scripts.engineering_artifact_admission_fixtures import (
    REQUIRED_CATEGORIES,
    build_manifest,
    canonical_json,
    expected,
    materialize_specs,
    sha256_bytes,
    validate_archive,
    validate_manifest,
)


class EngineeringArtifactAdmissionFixtureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.archive, self.manifest, self.entries = expected()

    def test_every_required_category_has_one_identity_bound_case(self) -> None:
        self.assertEqual(
            [case["category"] for case in self.manifest["cases"]],
            list(REQUIRED_CATEGORIES),
        )
        self.assertEqual(len({case["fixture_id"] for case in self.manifest["cases"]}), 16)
        for case in self.manifest["cases"]:
            records = case["records"]
            artifact = records["source_artifact"]
            self.assertEqual(artifact["reference_id"], records["source_reference"]["reference_id"])
            self.assertEqual(artifact["origin_id"], records["origin"]["origin_id"])
            self.assertEqual(artifact["provenance_sha256"], records["source_provenance"]["provenance_sha256"])

    def test_every_available_payload_has_exact_archive_byte_identity(self) -> None:
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            for case in self.manifest["cases"]:
                identity = case["offered_byte_identity"]
                entry = case["archive_entry"]
                if identity is None:
                    self.assertIsNone(entry)
                    continue
                content = archive.read(entry)
                self.assertEqual(identity, {"byte_length": len(content), "sha256": sha256_bytes(content)})
                artifact = case["records"]["source_artifact"]
                if artifact["capture_state"] == "captured":
                    self.assertEqual(artifact["byte_length"], identity["byte_length"])
                    self.assertEqual(artifact["sha256"], identity["sha256"])

    def test_duplicate_and_stale_relationships_are_explicit_without_false_capture(self) -> None:
        cases = {case["category"]: case for case in self.manifest["cases"]}
        self.assertEqual(cases["duplicate"]["offered_byte_identity"], cases["local_file"]["offered_byte_identity"])
        self.assertNotEqual(cases["duplicate"]["fixture_id"], cases["local_file"]["fixture_id"])
        stale = cases["stale"]
        self.assertNotEqual(stale["offered_byte_identity"], stale["current_byte_identity"])
        self.assertEqual(stale["records"]["source_artifact"]["capture_state"], "failed")
        self.assertIsNone(stale["records"]["source_artifact"]["sha256"])

    def test_inaccessible_and_unsupported_never_invent_source_bytes(self) -> None:
        cases = {case["category"]: case for case in self.manifest["cases"]}
        for category, state in (("inaccessible", "unavailable"), ("unsupported", "unsupported")):
            case = cases[category]
            self.assertIsNone(case["archive_entry"])
            self.assertIsNone(case["offered_byte_identity"])
            self.assertEqual(case["records"]["source_artifact"]["capture_state"], state)
            self.assertIsNone(case["records"]["source_artifact"]["byte_length"])
            self.assertIsNone(case["records"]["source_artifact"]["sha256"])

    def test_archive_is_deterministic_inert_stored_and_path_safe(self) -> None:
        self.assertEqual(validate_archive(self.archive, self.entries), [])
        self.assertEqual(expected()[0], self.archive)
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            self.assertIsNone(archive.testzip())
            self.assertTrue(all(item.compress_type == zipfile.ZIP_STORED for item in archive.infolist()))
            self.assertTrue(all((item.external_attr >> 16) & 0o111 == 0 for item in archive.infolist()))

    def test_manifest_is_self_hashed_and_rejects_identity_or_support_mutation(self) -> None:
        self.assertEqual(validate_manifest(self.manifest, self.archive, self.entries), [])
        changed = copy.deepcopy(self.manifest)
        changed["cases"][0]["offered_byte_identity"]["sha256"] = "f" * 64
        self.assertTrue(validate_manifest(changed, self.archive, self.entries))
        claimed = copy.deepcopy(self.manifest)
        claimed["product_support_claim"] = "all-formats-supported"
        self.assertTrue(validate_manifest(claimed, self.archive, self.entries))

    def test_manifest_contains_only_synthetic_protected_locator_hashes(self) -> None:
        content = canonical_json(self.manifest)
        for prohibited in (b"/home/", b"/Users/", b"file://", b"https://", b"AKIA", b"BEGIN PRIVATE KEY"):
            self.assertNotIn(prohibited, content)
        self.assertTrue(self.manifest["synthetic_only"])
        self.assertFalse(self.manifest["network_access"])

    def test_generator_and_document_inputs_are_hash_bound(self) -> None:
        rebuilt = build_manifest(materialize_specs(), self.archive, self.entries)
        self.assertEqual(rebuilt, self.manifest)
        self.assertEqual(len(self.manifest["generator"]["sha256"]), 64)
        self.assertEqual(len(self.manifest["document_fixture_source"]["profile_sha256"]), 64)


if __name__ == "__main__":
    unittest.main()

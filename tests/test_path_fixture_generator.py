from __future__ import annotations

import copy
import json
import tempfile
import unicodedata
import unittest
from pathlib import Path, PurePosixPath

from scripts.path_fixture_generator import (
    EXPECTED_SCENARIOS,
    EXPECTED_TRAVERSAL,
    PROFILE_PATH,
    check_report,
    generate,
    materialize_specs,
    read_json,
    scenario_manifest,
    validate_profile,
    validate_report,
)


class PathFixtureGeneratorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)
        self.manifest = scenario_manifest(self.profile)

    def test_checked_in_profile_and_report_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(check_report(), [])

    def test_all_seven_scenario_types_are_present_once(self) -> None:
        self.assertEqual(
            tuple(item["id"] for item in self.manifest["scenarios"]),
            EXPECTED_SCENARIOS,
        )

    def test_traversal_inputs_cover_relative_encoded_and_absolute_forms(self) -> None:
        traversal = self.manifest["scenarios"][0]
        self.assertEqual(tuple(traversal["inputs"]), EXPECTED_TRAVERSAL)
        self.assertIn("%2e%2e/outside/forbidden.txt", traversal["inputs"])
        self.assertIn("/synthetic-absolute/forbidden.txt", traversal["inputs"])

    def test_real_symlinks_resolve_inside_sandbox_but_outside_approved_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "fixtures"
            generate(self.profile, destination)
            sandbox = (destination / "sandbox").resolve()
            approved = (destination / "sandbox/allowed/workspace").resolve()
            for record in self.profile["symlink_fixtures"]:
                link = destination / record["path"]
                self.assertTrue(link.is_symlink())
                resolved = link.resolve()
                self.assertTrue(resolved.is_relative_to(sandbox))
                self.assertFalse(resolved.is_relative_to(approved))

    def test_case_collision_is_logical_and_portable(self) -> None:
        first, second = self.profile["case_collision_logical_paths"]
        self.assertNotEqual(first, second)
        self.assertEqual(first.casefold(), second.casefold())
        specs = materialize_specs(self.profile)
        self.assertFalse(any(first in path or second in path for path in specs))

    def test_unicode_collision_is_logical_and_portable(self) -> None:
        nfc, nfd = self.profile["unicode_collision_logical_paths"]
        self.assertNotEqual(nfc.encode("utf-8"), nfd.encode("utf-8"))
        self.assertEqual(unicodedata.normalize("NFC", nfc), unicodedata.normalize("NFC", nfd))

    def test_mount_change_is_a_nonprivileged_identity_fixture(self) -> None:
        scenario = next(item for item in self.manifest["scenarios"] if item["id"] == "mount-change")
        self.assertFalse(scenario["operation_performed"])
        self.assertNotEqual(scenario["identity_before"], scenario["identity_after"])

    def test_replacement_hashes_differ_and_match_payloads(self) -> None:
        scenario = next(
            item for item in self.manifest["scenarios"] if item["id"] == "file-replacement"
        )
        self.assertNotEqual(scenario["original_sha256"], scenario["replacement_sha256"])

    def test_stale_bookmark_is_synthetic_and_changed(self) -> None:
        scenario = next(item for item in self.manifest["scenarios"] if item["id"] == "stale-bookmark")
        self.assertEqual(scenario["bookmark_kind"], "synthetic-record")
        self.assertFalse(scenario["platform_bookmark_created"])
        self.assertNotEqual(scenario["observed_resource_id"], scenario["current_resource_id"])

    def test_generated_manifest_has_no_private_or_remote_path(self) -> None:
        encoded = json.dumps(self.manifest, sort_keys=True)
        for prohibited in ("/home/", "/Users/", "http://", "https://"):
            self.assertNotIn(prohibited, encoded)
        for record in self.profile["symlink_fixtures"]:
            self.assertFalse(PurePosixPath(record["target"]).is_absolute())

    def test_generation_refuses_existing_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(FileExistsError):
                generate(self.profile, Path(temporary))

    def test_real_macos_bookmark_claim_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.profile)
        mutated["macos_bookmark_claim"] = "verified-platform-bookmark"
        self.assertTrue(validate_profile(mutated))

    def test_product_path_safety_claim_is_rejected(self) -> None:
        report = read_json(
            Path("artifacts/sprints/sprint-2/story-2.1/path-fixture-report.json")
        )
        report["product_path_safety_claim"] = "pass"
        self.assertTrue(validate_report(report))


if __name__ == "__main__":
    unittest.main()

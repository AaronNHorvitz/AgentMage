from __future__ import annotations

import copy
import unittest

from scripts.component_inventory import (
    EXPECTED_EXECUTABLES,
    EXPECTED_PLATFORMS,
    EXPECTED_RUNTIME_CANDIDATES,
    EXPECTED_UNAPPROVED_COMPONENTS,
    REPORT_PATH,
    build_report,
    check_artifact,
    read_json,
    validate_policy,
    validate_report,
)


class ComponentInventoryTests(unittest.TestCase):
    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_every_approved_record_has_a_version_and_sha256(self) -> None:
        report = build_report()
        approved = report["approved_inventory"]
        for record in [
            *approved["build_executables"],
            *approved["build_environments"],
        ]:
            self.assertTrue(record["version"])
            self.assertRegex(record["sha256"], r"^[0-9a-f]{64}$")
        self.assertTrue(approved["packages"]["all_versions_present"])
        self.assertTrue(approved["packages"]["all_cryptographic_hashes_present"])

    def test_build_executable_and_platform_closures_are_exact(self) -> None:
        report = build_report()
        records = report["approved_inventory"]["build_executables"]
        self.assertEqual(len(records), len(EXPECTED_EXECUTABLES) * len(EXPECTED_PLATFORMS))
        self.assertEqual(
            {(item["platform"], item["executable_id"]) for item in records},
            {
                (platform, executable)
                for platform in EXPECTED_PLATFORMS
                for executable in EXPECTED_EXECUTABLES
            },
        )

    def test_package_inventory_binds_all_locked_components(self) -> None:
        packages = build_report()["approved_inventory"]["packages"]
        self.assertEqual(packages["component_count"], 421)
        self.assertEqual(packages["classifications"], {"development": 371, "production": 50})
        self.assertRegex(packages["identity_set_sha256"], r"^[0-9a-f]{64}$")

    def test_runtime_candidates_and_presence_only_components_are_not_approved(self) -> None:
        report = build_report()
        non_approved = report["non_approved_inventory"]
        self.assertEqual(
            tuple(item["component_id"] for item in non_approved["runtime_candidates"]),
            EXPECTED_RUNTIME_CANDIDATES,
        )
        self.assertEqual(
            tuple(
                item["component_id"]
                for item in non_approved["presence_only_components"]
            ),
            EXPECTED_UNAPPROVED_COMPONENTS,
        )
        self.assertTrue(
            all(
                item["approval_status"] == "candidate-not-approved"
                for item in non_approved["runtime_candidates"]
            )
        )
        self.assertTrue(
            all(
                item["version"] is None
                and item["sha256"] is None
                and item["approval_status"] == "presence-only-not-approved"
                for item in non_approved["presence_only_components"]
            )
        )

    def test_policy_rejects_early_runtime_optional_or_macos_approval(self) -> None:
        policy = read_json(REPORT_PATH.parents[4] / "architecture/component-inventory-policy.json")
        runtime = copy.deepcopy(policy)
        runtime["approved_product_runtime_ids"] = [EXPECTED_RUNTIME_CANDIDATES[0]]
        optional = copy.deepcopy(policy)
        optional["approved_optional_dependency_ids"] = ["docker-compatibility"]
        macos = copy.deepcopy(policy)
        macos["platform_status"]["macos"] = "verified"
        for changed in (runtime, optional, macos):
            self.assertTrue(validate_policy(changed))

    def test_stale_source_and_product_or_macos_overclaim_fail_closed(self) -> None:
        report = build_report()
        stale = copy.deepcopy(report)
        stale["source_artifacts"][0]["sha256"] = "0" * 64
        product = copy.deepcopy(report)
        product["product_runtime_activation_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (stale, product, macos):
            self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()

import copy
import unittest

from scripts import path_contract_artifact as artifact


class PathContractArtifactTests(unittest.TestCase):
    def test_sources_expose_exact_bounded_path_interface(self) -> None:
        self.assertEqual(
            artifact.validate_sources(),
            {
                "adapter_error_class_count": 15,
                "adapter_method_count": 3,
                "authorized_handle_method_count": 4,
                "held_object_method_count": 7,
                "path_platform_count": 3,
                "resolution_intent_count": 4,
                "strict_openat2_flag_count": 4,
            },
        )

    def test_report_rejects_platform_and_release_overclaims(self) -> None:
        value = {
            "schema_version": 1,
            "task_id": "6.1.2.1",
            "artifact_id": "path-contract-platform-adapter-reference",
            "status": "pass-shared-fedora-scope",
            "coverage": artifact.validate_sources(),
            "verification": {f"check-{index}": "pass" for index in range(7)},
            "platform_status": {
                "fedora": "verified-local",
                "ubuntu": "not-executed",
                "macos": "blocked-macos",
            },
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": ["one", "two", "three", "four"],
        }
        self.assertEqual(artifact.validate_report(value), [])
        for field, changed_value in (
            ("release_claim", "approved"),
            ("macos_evidence_substituted", True),
        ):
            changed = copy.deepcopy(value)
            changed[field] = changed_value
            with self.subTest(field=field):
                self.assertTrue(artifact.validate_report(changed))
        promoted = copy.deepcopy(value)
        promoted["platform_status"]["macos"] = "verified-local"
        self.assertTrue(artifact.validate_report(promoted))

    def test_report_rejects_missing_verification_and_coverage_drift(self) -> None:
        value = {
            "schema_version": 1,
            "task_id": "6.1.2.1",
            "artifact_id": "path-contract-platform-adapter-reference",
            "status": "pass-shared-fedora-scope",
            "coverage": artifact.validate_sources(),
            "verification": {f"check-{index}": "pass" for index in range(7)},
            "platform_status": {
                "fedora": "verified-local",
                "ubuntu": "not-executed",
                "macos": "blocked-macos",
            },
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": ["one", "two", "three", "four"],
        }
        missing = copy.deepcopy(value)
        missing["verification"].pop("check-0")
        drifted = copy.deepcopy(value)
        drifted["coverage"]["strict_openat2_flag_count"] = 3
        self.assertTrue(artifact.validate_report(missing))
        self.assertTrue(artifact.validate_report(drifted))


if __name__ == "__main__":
    unittest.main()

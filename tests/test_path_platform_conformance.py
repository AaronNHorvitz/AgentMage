import copy
import unittest

from scripts import path_platform_conformance as conformance


class PathPlatformConformanceTests(unittest.TestCase):
    def report(self):
        return {
            "schema_version": 1,
            "task_id": "6.1.3.4",
            "artifact_id": "shared-logical-path-platform-conformance",
            "status": "pass-all-available-non-macos-platforms",
            "reference_revision": "a" * 40,
            "coverage": {
                "available_adapter_result_count": 3,
                "equivalent_policy_decision_count": 3,
                "networked_test_count": 0,
                "out_of_workspace_access_count": 0,
            },
            "results": [
                {"status": "pass", "policy_decision": "admit-canonical-read"},
                {"status": "pass", "policy_decision": "admit-canonical-read"},
                {
                    "status": "pass", "policy_decision": "admit-canonical-read",
                    "container_network": "none",
                },
            ],
            "platform_status": {
                "deterministic_fake": "verified",
                "fedora_44": "verified-local",
                "ubuntu_26_04": "verified-no-network-container",
                "macos": "blocked-macos",
            },
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": ["macOS blocked"],
        }

    def test_bounded_non_macos_report_is_accepted(self) -> None:
        self.assertEqual(conformance.validate_report(self.report()), [])

    def test_failed_missing_or_networked_result_fails_closed(self) -> None:
        failed = copy.deepcopy(self.report())
        failed["results"][0]["status"] = "fail"
        missing = copy.deepcopy(self.report())
        missing["results"].pop()
        networked = copy.deepcopy(self.report())
        networked["results"][2]["container_network"] = "bridge"
        for changed in (failed, missing, networked):
            with self.subTest():
                self.assertTrue(conformance.validate_report(changed))

    def test_macos_or_release_promotion_fails_closed(self) -> None:
        for field, replacement in (
            ("macos_evidence_substituted", True),
            ("release_claim", "approved"),
        ):
            changed = copy.deepcopy(self.report())
            changed[field] = replacement
            with self.subTest(field=field):
                self.assertTrue(conformance.validate_report(changed))
        promoted = copy.deepcopy(self.report())
        promoted["platform_status"]["macos"] = "verified"
        self.assertTrue(conformance.validate_report(promoted))


if __name__ == "__main__":
    unittest.main()

import copy
import unittest

from scripts import path_race_artifact as artifact


class PathRaceArtifactTests(unittest.TestCase):
    def report(self):
        return {
            "schema_version": 1,
            "task_id": "6.1.2.3",
            "related_verification_task": "6.1.3.2",
            "artifact_id": "linux-file-identity-race-harness",
            "status": "pass-fedora-unprivileged-scope",
            "reference_revision": "a" * 40,
            "coverage": {
                "attack_class_count": 8,
                "executed_scenario_count": 5,
                "out_of_root_access_count": 0,
                "concurrent_minimum_resolution_attempts": 512,
            },
            "traces": [
                {
                    "scenario_id": scenario,
                    "test_name": test,
                    "execution_status": "pass-fedora-local",
                    "out_of_root_access_count": 0,
                }
                for scenario, test in artifact.TESTS
            ],
            "attack_status": {
                "symlink": "pass-fedora-local",
                "hard_link": "pass-fedora-local",
                "rename": "pass-fedora-local",
                "replacement": "pass-fedora-local",
                "content_mutation": "pass-fedora-local",
                "concurrent_toctou": "pass-fedora-local",
                "mount_identity_drift": "pass-synthetic-unit",
                "privileged_mount_swap": "not-executed-requires-isolated-privilege",
                "macos_alias": "blocked-macos",
            },
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": ["one", "two", "three"],
        }

    def test_valid_bounded_report_is_accepted(self) -> None:
        self.assertEqual(artifact.validate_report(self.report()), [])

    def test_trace_failure_or_escape_fails_closed(self) -> None:
        failed = copy.deepcopy(self.report())
        failed["traces"][0]["execution_status"] = "fail"
        escaped = copy.deepcopy(self.report())
        escaped["coverage"]["out_of_root_access_count"] = 1
        self.assertTrue(artifact.validate_report(failed))
        self.assertTrue(artifact.validate_report(escaped))

    def test_platform_or_release_overclaim_fails_closed(self) -> None:
        for field, replacement in (
            ("macos_evidence_substituted", True),
            ("release_claim", "approved"),
        ):
            changed = copy.deepcopy(self.report())
            changed[field] = replacement
            with self.subTest(field=field):
                self.assertTrue(artifact.validate_report(changed))
        promoted = copy.deepcopy(self.report())
        promoted["attack_status"]["macos_alias"] = "pass"
        self.assertTrue(artifact.validate_report(promoted))


if __name__ == "__main__":
    unittest.main()

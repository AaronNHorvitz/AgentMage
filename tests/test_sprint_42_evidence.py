import copy
import importlib.util
import json
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "sprint_42_evidence.py"
SPEC = importlib.util.spec_from_file_location("sprint_42_evidence", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class Sprint42EvidenceTests(unittest.TestCase):
    def valid_report(self):
        commands = []
        for identifier, argv in MODULE.COMMANDS:
            commands.append({
                "id": identifier,
                "argv": list(argv),
                "exit_code": 0,
                "output_sha256": "1" * 64,
                "blocking_skip_count": 0 if identifier in MODULE.FOCUSED_COMMANDS else None,
            })
        artifact = {
            "id": "git",
            "name": "git",
            "size": 1,
            "sha256": "2" * 64,
            "root_owned": True,
            "group_or_world_writable": False,
        }
        with patch.object(MODULE, "git_file", return_value=b"source"):
            return MODULE.build_report("a" * 40, commands, [artifact])

    def validate(self, value):
        with patch.object(MODULE, "git_file", return_value=b"source"):
            return MODULE.validate_report(value, verify_current=False)

    def test_valid_synthetic_report_passes(self):
        self.assertEqual(self.validate(self.valid_report()), [])

    def test_report_policy_collections_do_not_alias_module_constants(self):
        report = self.valid_report()
        report["security_requirement_ids"].clear()
        report["implemented_contracts"]["network_clone_or_fetch_execution"] = True
        report["blockers"].clear()

        self.assertTrue(MODULE.SECURITY_REQUIREMENTS)
        self.assertFalse(MODULE.IMPLEMENTED["network_clone_or_fetch_execution"])
        self.assertTrue(MODULE.BLOCKERS)

    def test_release_and_network_overclaims_fail(self):
        for mutate in (
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["summary"].update({"network_git_enabled": True}),
            lambda value: value["verification_evidence"].update(
                {"network_clone_or_fetch_execution": True}
            ),
            lambda value: value["implemented_contracts"].update(
                {"native_cross_platform_acceptance": True}
            ),
        ):
            report = self.valid_report()
            mutate(report)
            self.assertTrue(self.validate(report))

    def test_failed_skipped_or_tampered_commands_fail(self):
        report = self.valid_report()
        report["commands"][0]["exit_code"] = 1
        self.assertIn("command result invalid", self.validate(report))

        report = self.valid_report()
        report["commands"][0]["blocking_skip_count"] = 1
        self.assertTrue(
            any("focused skipped" in item for item in self.validate(report))
        )

        report = self.valid_report()
        report["commands"].pop()
        self.assertIn("command inventory drift", self.validate(report))

    def test_artifact_or_source_drift_fails(self):
        report = self.valid_report()
        report["native_fixture_artifacts"][0]["root_owned"] = False
        self.assertIn("native artifact claim invalid", self.validate(report))

        report = self.valid_report()
        first = MODULE.SOURCE_PATHS[0]
        report["source_sha256"][first] = "f" * 64
        self.assertTrue(any("source hash drift" in item for item in self.validate(report)))

    def test_report_shape_contains_no_raw_repository_or_credentials(self):
        encoded = json.dumps(self.valid_report(), sort_keys=True)
        for prohibited in (
            "canonical_url",
            "repository_path",
            "remote_url",
            "credential_value",
            "command_output",
            "file_content",
        ):
            self.assertNotIn(prohibited, encoded)

    def test_blocker_removal_fails(self):
        report = self.valid_report()
        report["blockers"] = copy.deepcopy(report["blockers"][:-1])
        self.assertIn("blocker drift", self.validate(report))


if __name__ == "__main__":
    unittest.main()

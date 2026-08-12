from __future__ import annotations

import copy
import unittest

from scripts.platform_gates import (
    POLICY_PATH,
    ROOT,
    STATUS_PATH,
    build_report,
    compose_milestone,
    validate_policy,
    validate_status,
)
from scripts.evidence_core import read_json_object


class PlatformGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = read_json_object(ROOT, POLICY_PATH.relative_to(ROOT).as_posix())
        self.status = read_json_object(ROOT, STATUS_PATH.relative_to(ROOT).as_posix())

    def test_current_lane_policy_and_status_are_exact(self) -> None:
        self.assertEqual(validate_policy(self.policy), [])
        self.assertEqual(validate_status(self.status, self.policy, ROOT), [])

    def test_linux_progress_is_not_erased_by_retained_macos_block(self) -> None:
        report = build_report()
        milestones = {item["milestone_id"]: item for item in report["milestones"]}
        self.assertEqual(milestones["linux-source-candidate"]["status"], "pass")
        self.assertEqual(milestones["macos-retained-milestone"]["status"], "block")
        self.assertNotIn(
            "macos-arm64-retained",
            milestones["linux-source-candidate"]["required_lanes"],
        )

    def test_missing_windows_and_ubuntu_block_first_ga_only(self) -> None:
        report = build_report()
        milestones = {item["milestone_id"]: item for item in report["milestones"]}
        self.assertEqual(
            milestones["v1.0-first-ga"]["blocking_lanes"],
            ["ubuntu-x86_64", "windows-x86_64"],
        )
        self.assertEqual(milestones["linux-source-candidate"]["blocking_lanes"], [])

    def test_unsupported_required_lane_blocks_its_milestone(self) -> None:
        lanes = {item["lane_id"]: copy.deepcopy(item) for item in self.status["lanes"]}
        lanes["windows-x86_64"]["state"] = "unsupported"
        milestone = next(
            item for item in self.policy["milestones"] if item["milestone_id"] == "v1.0-first-ga"
        )
        result = compose_milestone(milestone, lanes)
        self.assertEqual(result["status"], "block")
        self.assertIn("windows-x86_64", result["blocking_lanes"])

    def test_linux_evidence_cannot_substitute_for_macos(self) -> None:
        changed = copy.deepcopy(self.status)
        mac = next(item for item in changed["lanes"] if item["lane_id"] == "macos-arm64-retained")
        mac["native_lane"] = "fedora-x86_64"
        failures = validate_status(changed, self.policy, ROOT)
        self.assertIn("platform.macos-arm64-retained.evidence_substitution", failures)

    def test_unknown_or_reordered_milestone_lane_fails_closed(self) -> None:
        changed = copy.deepcopy(self.policy)
        changed["milestones"][0]["required_lanes"] = ["shared", "unknown"]
        self.assertTrue(validate_policy(changed))


if __name__ == "__main__":
    unittest.main()

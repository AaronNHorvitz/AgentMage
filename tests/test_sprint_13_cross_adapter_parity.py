from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_13_cross_adapter_parity as parity


class Sprint13CrossAdapterParityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        with (
            patch.object(parity, "resolve_revision", return_value="a" * 40),
            patch.object(parity, "git_sha256", return_value="b" * 64),
        ):
            cls.report = parity.build_report("HEAD")

    def test_retained_trials_and_each_one_field_mutation_are_bounded(self) -> None:
        self.assertEqual(parity.validate_report(self.report, verify_current=False), [])
        self.assertEqual(
            [item["id"] for item in self.report["mutation_results"]],
            [item[0] for item in parity.MUTATIONS],
        )
        self.assertTrue(
            all(item["changed_field_count"] == 1 for item in self.report["mutation_results"])
        )

    def test_quality_failure_cannot_be_relabelled_as_eligible(self) -> None:
        changed = copy.deepcopy(self.report)
        changed["disposition"]["eligible_for_merge"] = True
        self.assertTrue(parity.validate_report(changed, verify_current=False))

    def test_mutation_cannot_merge_or_enable(self) -> None:
        for field in ("merged", "enabled"):
            changed = copy.deepcopy(self.report)
            changed["mutation_results"][0][field] = True
            self.assertTrue(parity.validate_report(changed, verify_current=False))

    def test_input_and_source_membership_are_closed(self) -> None:
        changed = copy.deepcopy(self.report)
        changed["input_sha256"]["unexpected"] = "c" * 64
        self.assertTrue(parity.validate_report(changed, verify_current=False))

    def test_review_and_security_mappings_are_closed(self) -> None:
        changed = copy.deepcopy(self.report)
        changed["review_protocol_ids"].pop()
        self.assertTrue(parity.validate_report(changed, verify_current=False))

        changed = copy.deepcopy(self.report)
        changed["security_requirement_ids"].append("SR-UNKNOWN")
        self.assertTrue(parity.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()

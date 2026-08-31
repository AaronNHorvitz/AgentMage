from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts import story_8_1_security_evidence as evidence


class Story81SecurityEvidenceTests(unittest.TestCase):
    """Five closed mapping tests for Task 8.1.3.5."""

    def setUp(self) -> None:
        self.value = evidence.build_map("a" * 40)

    def test_exact_requirement_and_protocol_closure(self) -> None:
        self.assertEqual(
            [item["requirement_id"] for item in self.value["requirements"]],
            list(evidence.REQUIREMENTS),
        )
        self.assertEqual(
            [item["protocol_id"] for item in self.value["reviewer_protocols"]],
            list(evidence.PROTOCOLS),
        )
        self.assertEqual(self.value["summary"]["mapped_requirement_count"], 12)
        self.assertEqual(self.value["summary"]["mapped_reviewer_protocol_count"], 5)

    def test_all_native_classes_remain_external_and_unretained(self) -> None:
        self.assertEqual(
            [item["evidence_class"] for item in self.value["native_evidence_inventory"]],
            list(evidence.NATIVE_CLASSES),
        )
        self.assertTrue(
            all(
                item["status"] == "missing-external" and item["retained"] is False
                for item in self.value["native_evidence_inventory"]
            )
        )

    def test_every_source_report_is_blocked_and_nonpromoting(self) -> None:
        self.assertEqual(evidence.validate_inputs(), [])
        for name in evidence.SOURCE_REPORTS:
            value = json.loads((evidence.ROOT / evidence.REPORT_ROOT / name).read_text())
            self.assertIn(
                value["status"],
                {"partial-source-only-blocked-macos", "prepared-source-only-blocked-macos"},
            )
            self.assertTrue(all(claim is False for claim in value["claims"].values()))

    def test_completion_release_execution_and_substitution_are_false(self) -> None:
        fields = (
            "private_user_data_used",
            "network_used",
            "macos_execution_performed",
            "independent_review_performed",
            "macos_evidence_substituted",
            "macos_support_claim",
        )
        self.assertTrue(all(self.value[field] is False for field in fields))
        self.assertEqual(self.value["release_claim"], "none")
        self.assertFalse(self.value["summary"]["story_gate_complete"])

    def test_retained_map_mutations_fail_currentness(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for relative in (*evidence.EVIDENCE_PATHS, "TASKS.md"):
                target = root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes((evidence.ROOT / relative).read_bytes())
            report = copy.deepcopy(self.value)
            report["macos_support_claim"] = True
            target = root / evidence.REPORT_PATH.relative_to(evidence.ROOT)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(json.dumps(report), encoding="utf-8")
            self.assertTrue(evidence.check_map(root))


if __name__ == "__main__":
    unittest.main()

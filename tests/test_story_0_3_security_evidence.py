from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.story_0_3_security_evidence import (
    Story03SecurityEvidenceError,
    build_bundle,
    protocol_map,
    reviewer_disposition,
    validate_inputs,
    write_bundle,
)


class Story03SecurityEvidenceTests(unittest.TestCase):
    def test_all_security_evidence_inputs_are_current(self) -> None:
        self.assertEqual(validate_inputs(), [])

    def test_declared_feasibility_protocols_are_complete_without_full_claim(self) -> None:
        controls = protocol_map()["protocols"]
        self.assertEqual(set(controls), {"RV-06", "RV-13", "RV-14", "RV-16"})
        self.assertTrue(
            all(item["feasibility_execution_status"] == "COMPLETE" for item in controls.values())
        )
        self.assertTrue(
            all(item["full_protocol_status"] == "NOT_COMPLETE" for item in controls.values())
        )

    def test_disposition_preserves_rejections_and_missing_mac(self) -> None:
        decision = reviewer_disposition("0" * 40)
        self.assertEqual(decision["e4b_status"], "REJECTED_DISABLED")
        self.assertEqual(decision["fallback_status"], "REJECTED_DISABLED")
        self.assertEqual(decision["story_0_3_status"], "BLOCKED_REQUIRED_MAC_HARDWARE")
        self.assertFalse(decision["macos_evidence_substituted"])
        self.assertFalse(decision["release_approval"])

    def test_generated_bundle_contains_no_full_protocol_or_release_claim(self) -> None:
        bundle = build_bundle("HEAD")
        self.assertIn(b'"full_protocols_complete": false', bundle["raw-checker-output.json"])
        self.assertIn(b'"release_approval": false', bundle["reviewer-disposition.json"])

    def test_writer_refuses_to_overwrite_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            with self.assertRaises(Story03SecurityEvidenceError):
                write_bundle(output, "HEAD")


if __name__ == "__main__":
    unittest.main()

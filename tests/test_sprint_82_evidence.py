import copy
import unittest
from unittest.mock import patch

from scripts import sprint_82_evidence as evidence
from scripts.sprint_evidence_recorder import build_report, validate_report


def commands() -> list[dict[str, object]]:
    return [{"id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64,
             "blocking_skip_count": 0 if identifier in evidence.FOCUSED_COMMANDS else None}
            for identifier, argv in evidence.COMMANDS]


def report() -> dict[str, object]:
    with patch("scripts.sprint_evidence_recorder.git_file", return_value=b"source"):
        return build_report(evidence.DEFINITION, "c" * 40, commands(), environment={})


class Sprint82EvidenceTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        with patch("scripts.sprint_evidence_recorder.git_file", return_value=b"source"):
            return validate_report(evidence.DEFINITION, value, verify_ancestry=False)

    def test_valid(self) -> None:
        self.assertEqual(self.validate(report()), [])

    def test_external_overclaims_are_rejected(self) -> None:
        for field in ("upstream_sprint_81_closed", "live_research_complete",
                      "native_security_evidence_complete", "privacy_review_present", "release_approval"):
            value = copy.deepcopy(report())
            value["summary"][field] = True
            self.assertTrue(self.validate(value))

    def test_runtime_overclaims_are_rejected(self) -> None:
        for field in ("live_public_search_executed", "browser_campaign_complete", "privacy_review"):
            value = copy.deepcopy(report())
            value["verification_evidence"][field] = True
            self.assertTrue(self.validate(value))


if __name__ == "__main__":
    unittest.main()

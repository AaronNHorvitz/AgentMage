from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import story_17_1_git_fuzz_evidence as evidence


class Story171GitFuzzEvidenceTests(unittest.TestCase):
    def report(self) -> dict[str, object]:
        with patch.object(
            evidence,
            "source_records",
            return_value=[
                {"path": path, "bytes": 1, "sha256": "a" * 64}
                for path in evidence.SOURCE_PATHS
            ],
        ):
            return evidence.build_report(
                "b" * 40,
                {
                    "argv": ["cargo", "fuzz"],
                    "exit_code": 0,
                    "output_sha256": "c" * 64,
                    "completion_marker_present": True,
                    "crash_artifact_count": 0,
                },
            )

    def test_clean_campaign_is_admitted_without_product_claim(self) -> None:
        report = self.report()
        self.assertEqual(evidence.validate_report(report, verify_current=False), [])
        self.assertTrue(report["manual_parser_fuzzing_complete"])
        self.assertEqual(report["product_support_claim"], "none")

    def test_failure_crash_limit_and_claim_mutations_fail_closed(self) -> None:
        mutations = (
            lambda value: value["campaign"].update({"exit_code": 1}),
            lambda value: value["campaign"].update({"crash_artifact_count": 1}),
            lambda value: value["limits"].update({"duration_seconds": 59}),
            lambda value: value.update({"network_used": True}),
            lambda value: value.update({"product_support_claim": "supported"}),
            lambda value: value.update({"release_claim": True}),
            lambda value: value["sources"].pop(),
        )
        for mutate in mutations:
            report = copy.deepcopy(self.report())
            mutate(report)
            self.assertTrue(evidence.validate_report(report, verify_current=False))


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import copy
import unittest

from scripts.evidence_core import read_json_object
from scripts.review_provenance import (
    REGISTRY_PATH,
    ROOT,
    satisfies_required_class,
    validate_registry,
)


class ReviewProvenanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.registry = read_json_object(ROOT, REGISTRY_PATH.relative_to(ROOT).as_posix())

    def test_historical_automation_is_valid_aggregation_not_human_review(self) -> None:
        self.assertEqual(validate_registry(self.registry), [])
        subject = self.registry["records"][0]["subject"]["path"]
        self.assertTrue(
            satisfies_required_class(self.registry, subject, "automated-aggregation")
        )
        self.assertFalse(
            satisfies_required_class(self.registry, subject, "independent-human-review")
        )

    def test_self_or_aggregation_cannot_be_mislabeled_independent(self) -> None:
        changed = copy.deepcopy(self.registry)
        changed["records"][0]["review_class"] = "independent-automated-analysis"
        self.assertIn(
            "review.review-grant-boundary-historical.independence",
            validate_registry(changed),
        )

    def test_missing_reviewer_and_reused_process_fail_closed(self) -> None:
        missing = copy.deepcopy(self.registry)
        missing["records"][0]["performer"]["identity"] = ""
        self.assertTrue(validate_registry(missing))
        reused = copy.deepcopy(self.registry)
        reused["records"][0]["review_class"] = "independent-automated-analysis"
        reused["records"][0]["performer"]["distinct_implementation"] = True
        self.assertTrue(validate_registry(reused))

    def test_blocked_review_retains_owned_open_finding(self) -> None:
        blocked = copy.deepcopy(self.registry)
        record = blocked["records"][0]
        record["status"] = "blocked"
        record["findings"] = [
            {
                "finding_id": "finding-001",
                "owner": "kernel-boundary",
                "severity": "high",
                "state": "open",
            }
        ]
        self.assertEqual(validate_registry(blocked), [])
        self.assertFalse(
            satisfies_required_class(
                blocked, record["subject"]["path"], "automated-aggregation"
            )
        )

    def test_completed_review_cannot_hide_open_finding(self) -> None:
        changed = copy.deepcopy(self.registry)
        changed["records"][0]["findings"] = [
            {
                "finding_id": "finding-001",
                "owner": "kernel-boundary",
                "severity": "high",
                "state": "open",
            }
        ]
        self.assertTrue(validate_registry(changed))

    def test_rereview_must_resolve_to_an_acyclic_existing_record(self) -> None:
        changed = copy.deepcopy(self.registry)
        changed["records"][0]["rereview_of"] = changed["records"][1]["review_id"]
        changed["records"][1]["rereview_of"] = changed["records"][0]["review_id"]
        self.assertIn("review.rereview_cycle", validate_registry(changed))


if __name__ == "__main__":
    unittest.main()

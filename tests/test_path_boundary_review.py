import copy
import unittest

from scripts import path_boundary_review as review


class PathBoundaryReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.subjects = review.load_subjects()

    def test_review_reconciles_current_subjects(self) -> None:
        self.assertEqual(
            review.independent_review(),
            {
                "reviewed_subject_count": 4,
                "generated_path_case_count": 640,
                "admitted_escape_count": 0,
                "display_link_rejection_count": 1280,
                "race_executed_scenario_count": 6,
                "out_of_root_access_count": 0,
                "open_boundary_count": 5,
            },
        )

    def test_subject_omission_or_escape_fails_closed(self) -> None:
        omitted = copy.deepcopy(self.subjects)
        omitted.pop("path-race-report.json")
        escaped = copy.deepcopy(self.subjects)
        escaped["path-corpus-report.json"]["coverage"]["admitted_escape_count"] = 1
        self.assertTrue(review.validate_subjects(omitted))
        self.assertTrue(review.validate_subjects(escaped))

    def test_review_rejects_completion_fuzzing_and_macos_overclaims(self) -> None:
        value = {
            "schema_version": 1,
            "task_id": "6.1.3.5",
            "artifact_id": "independent-automated-secure-path-review",
            "status": "pass-shared-fedora-review-scope",
            "reference_revision": "a" * 40,
            "review_protocol": "RV-04",
            "review_protocol_status": "partial-blocked-platform-and-privileged-tests",
            "reviewer_identity": "agentmage-path-boundary-review-v1",
            "external_human_review_status": "not-performed",
            "coverage": review.independent_review(),
            "verification": {f"check-{index}": "pass" for index in range(7)},
            "findings": [],
            "open_product_boundaries": list(review.OPEN_BOUNDARIES),
            "product_requirement_completion_claim": "none",
            "fuzzing_claim": "generated-fixed-corpus-not-fuzzing",
            "release_claim": "none",
            "macos_execution_status": "blocked-macos",
            "macos_evidence_substituted": False,
        }
        self.assertEqual(review.validate_report(value), [])
        for field, replacement in (
            ("product_requirement_completion_claim", "complete"),
            ("fuzzing_claim", "coverage-guided-pass"),
            ("release_claim", "approved"),
            ("macos_execution_status", "pass"),
            ("macos_evidence_substituted", True),
        ):
            changed = copy.deepcopy(value)
            changed[field] = replacement
            with self.subTest(field=field):
                self.assertTrue(review.validate_report(changed))


if __name__ == "__main__":
    unittest.main()

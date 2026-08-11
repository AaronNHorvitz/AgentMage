import copy
import unittest

from scripts import grant_boundary_review as review


class GrantBoundaryReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.subjects = review.load_subjects()

    def test_independent_review_reconciles_all_story_5_subjects(self) -> None:
        coverage = review.independent_review()
        self.assertEqual(coverage["reviewed_subject_count"], 7)
        self.assertEqual(coverage["mutation_seed_count"], 560)
        self.assertEqual(coverage["race_scenario_count"], 5)
        self.assertEqual(coverage["state_transition_count"], 8)
        self.assertEqual(coverage["admitted_mutation_count"], 0)
        self.assertEqual(coverage["admitted_escalation_count"], 0)
        self.assertEqual(coverage["replay_success_count"], 0)

    def test_missing_subject_and_changed_security_result_fail_closed(self) -> None:
        omitted = copy.deepcopy(self.subjects)
        omitted.pop("grant-state-report.json")
        changed = copy.deepcopy(self.subjects)
        changed["adversarial-grant-corpus-report.json"]["coverage"][
            "admitted_attempt_count"
        ] = 1
        self.assertTrue(review.validate_subjects(omitted))
        self.assertTrue(review.validate_subjects(changed))

    def test_failed_verification_and_macos_promotion_fail_closed(self) -> None:
        failed = copy.deepcopy(self.subjects)
        failed["grant-race-replay-report.json"]["verification"][
            "terminal_replay_denial"
        ] = "fail"
        promoted = copy.deepcopy(self.subjects)
        promoted["grant-policy-reference-report.json"]["platform_status"][
            "macos"
        ] = "verified-local"
        self.assertTrue(review.validate_subjects(failed))
        self.assertTrue(review.validate_subjects(promoted))

    def test_review_discloses_open_boundaries_and_rejects_overclaims(self) -> None:
        value = {
            "schema_version": 1,
            "task_id": "5.1.3.5",
            "artifact_id": "grant-boundary-independent-automated-review",
            "status": "pass-shared-linux-story-scope",
            "review_type": "independent-automated-boundary-review",
            "reviewer_identity": "agentmage-grant-boundary-review-v1",
            "external_human_review_status": "not-performed",
            "coverage": {
                "reviewed_subject_count": 7,
                "mutation_seed_count": 560,
                "race_scenario_count": 5,
                "state_transition_count": 8,
                "authority_escalation_case_count": 28,
                "stale_dispatch_case_count": 4,
                "admitted_mutation_count": 0,
                "admitted_escalation_count": 0,
                "replay_success_count": 0,
                "open_product_boundary_count": 8,
            },
            "verification": {f"check-{index}": "pass" for index in range(9)},
            "findings": [],
            "open_product_boundaries": list(review.OPEN_BOUNDARIES),
            "product_requirement_completion_claim": "none",
            "release_claim": "none",
            "macos_execution_status": "blocked-macos",
            "macos_evidence_substituted": False,
        }
        self.assertEqual(review.validate_report(value), [])
        for field, changed_value in (
            ("external_human_review_status", "pass"),
            ("product_requirement_completion_claim", "complete"),
            ("release_claim", "pass"),
            ("macos_execution_status", "pass"),
            ("macos_evidence_substituted", True),
        ):
            changed = copy.deepcopy(value)
            changed[field] = changed_value
            with self.subTest(field=field):
                self.assertTrue(review.validate_report(changed))


if __name__ == "__main__":
    unittest.main()

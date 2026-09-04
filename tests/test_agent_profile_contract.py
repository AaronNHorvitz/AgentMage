import copy
import unittest

from scripts.agent_profile_contract import (
    AgentProfileContractError,
    FIXTURE_CLASSES,
    INDEPENDENT_REVIEW,
    build_artifacts,
    parse_profiles,
)


class AgentProfileContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.artifacts = build_artifacts()

    def test_exact_catalog_and_generated_artifact_set(self):
        self.assertEqual(set(self.artifacts), {"catalog", "reference", "matrix", "fixtures", "evaluation"})
        catalog = __import__("json").loads(self.artifacts["catalog"])
        self.assertEqual([row["profile_id"] for row in catalog["profiles"]], [f"AG-{number:02d}" for number in range(1, 50)])
        self.assertEqual(catalog["enabled_profile_count"], 0)
        self.assertEqual(catalog["runtime_implementation_count"], 1)

    def test_every_profile_has_closed_contract_and_no_registration_authority(self):
        catalog = __import__("json").loads(self.artifacts["catalog"])
        required = {"version", "owner", "source_sha256", "signature", "lifecycle", "compatible_model_profiles", "compatible_codec_profiles", "requested_tools", "requested_roots", "provider_object_classes", "input_schema", "output_schema", "evidence", "citations", "budgets", "approval", "completion", "cancellation", "stop_conditions"}
        for profile in catalog["profiles"]:
            self.assertFalse(required - profile.keys())
            self.assertEqual(profile["lifecycle"], "disabled")
            self.assertFalse(profile["enabled"])
            self.assertFalse(profile["user_reviewed"])
            self.assertEqual(profile["provider_object_classes"], ["none-by-registration"])

    def test_independent_review_profiles_are_exact_and_machine_testable(self):
        catalog = __import__("json").loads(self.artifacts["catalog"])
        actual = {row["profile_id"] for row in catalog["profiles"] if row["independent_review_required"]}
        self.assertEqual(actual, INDEPENDENT_REVIEW)
        for row in catalog["profiles"]:
            if row["profile_id"] in actual:
                self.assertTrue(all(row["independent_review"].values()))

    def test_fixture_cross_product_and_synthetic_no_effect_decision(self):
        json = __import__("json")
        fixtures = json.loads(self.artifacts["fixtures"])
        evaluation = json.loads(self.artifacts["evaluation"])
        self.assertEqual(fixtures["case_count"], 49 * len(FIXTURE_CLASSES))
        self.assertEqual({case["class"] for case in fixtures["cases"]}, set(FIXTURE_CLASSES))
        self.assertTrue(all(case["real_effect_count"] == 0 for case in fixtures["cases"]))
        self.assertEqual(evaluation["enabled_profile_count"], 0)
        self.assertEqual(evaluation["real_data_read_count"], 0)
        self.assertEqual(evaluation["real_external_effect_count"], 0)
        self.assertFalse(evaluation["all_signatures_verified"])
        self.assertFalse(evaluation["all_user_reviews_present"])

    def test_source_mutations_fail_exact_identity_and_uniqueness(self):
        source = __import__("pathlib").Path("docs/architecture/planning-review-and-delivery-agent-profiles.md").read_text()
        with self.assertRaises(AgentProfileContractError):
            parse_profiles(source.replace("`AG-49`", "`AG-50`"))
        with self.assertRaises(AgentProfileContractError):
            parse_profiles(source.replace("License and Provenance Reviewer", "Cost and Capacity Analyst"))


if __name__ == "__main__":
    unittest.main()

import copy
import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts/story_7_1_security_evidence.py"
SPEC = importlib.util.spec_from_file_location("story_7_1_security_evidence", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class Story71SecurityEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline = MODULE.build_map("a" * 40)

    def test_baseline_map_is_complete_bounded_and_current(self):
        self.assertEqual(MODULE.validate_map(self.baseline), [])
        self.assertEqual(
            [item["requirement_id"] for item in self.baseline["requirements"]],
            list(MODULE.EXPECTED_REQUIREMENTS),
        )
        self.assertTrue(
            all(
                item["product_requirement_status"] == "not-complete"
                for item in self.baseline["requirements"]
            )
        )

    def test_requirement_protocol_or_artifact_removal_fails(self):
        for field in ("requirements", "reviewer_protocols", "artifacts"):
            candidate = copy.deepcopy(self.baseline)
            candidate[field].pop()
            self.assertTrue(MODULE.validate_map(candidate), field)

    def test_product_release_or_macos_promotion_fails(self):
        mutations = (
            ("product_requirement_completion_claim", "complete"),
            ("release_claim", "supported"),
            ("macos_execution_status", "pass"),
            ("macos_evidence_substituted", True),
            ("private_user_data_used", True),
            ("network_used", True),
        )
        for field, value in mutations:
            candidate = copy.deepcopy(self.baseline)
            candidate[field] = value
            self.assertTrue(MODULE.validate_map(candidate), field)

    def test_mapping_text_status_and_paths_are_exact(self):
        candidate = copy.deepcopy(self.baseline)
        candidate["requirements"][0]["remaining"] = "nothing remains"
        self.assertTrue(MODULE.validate_map(candidate))
        candidate = copy.deepcopy(self.baseline)
        candidate["requirements"][0]["evidence"][0] = "../private"
        self.assertTrue(MODULE.validate_map(candidate))
        candidate = copy.deepcopy(self.baseline)
        candidate["reviewer_protocols"][0]["status"] = "pass"
        self.assertTrue(MODULE.validate_map(candidate))


if __name__ == "__main__":
    unittest.main()

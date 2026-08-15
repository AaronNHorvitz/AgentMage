from __future__ import annotations

import json
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "docs/verification/sprint-46-failure-corpus.json"
SHA256 = re.compile(r"^[0-9a-f]{64}$")
EXPECTED_CLASSIFICATIONS = {
    "baseline",
    "change_caused",
    "dependency",
    "environment",
    "flaky",
    "permission",
    "unclassified",
    "unrelated",
}


def classify(case: dict[str, object]) -> str:
    status = case["status"]
    platform = str(case["platform_code"])
    if status == "flaky":
        return "flaky"
    if "permission" in platform or "denied" in platform:
        return "permission"
    if "dependency" in platform:
        return "dependency"
    if status in {
        "infrastructure_failed",
        "timed_out",
        "cancelled",
        "crashed",
        "truncated",
        "sensitive_output",
    } or "environment" in platform:
        return "environment"
    if case["baseline_failure_match"] is True:
        return "baseline"
    if case["affected_path_intersects_change"] is True:
        return "change_caused"
    if case["baseline_known_clean"] is True:
        return "unrelated"
    return "unclassified"


class Sprint46ValidationCorpusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.corpus = json.loads(CORPUS.read_text(encoding="utf-8"))

    def test_corpus_is_complete_sorted_and_content_free(self) -> None:
        self.assertEqual(self.corpus["schema_version"], 1)
        self.assertEqual(
            self.corpus["classification_precedence"],
            [
                "flaky",
                "permission",
                "dependency",
                "environment",
                "baseline",
                "change_caused",
                "unrelated",
                "unclassified",
            ],
        )
        cases = self.corpus["cases"]
        self.assertEqual(
            [case["case_id"] for case in cases],
            sorted(case["case_id"] for case in cases),
        )
        self.assertEqual(
            {case["expected_classification"] for case in cases},
            EXPECTED_CLASSIFICATIONS,
        )
        policy = self.corpus["content_policy"]
        self.assertTrue(policy["synthetic_only"])
        self.assertFalse(policy["raw_process_bytes_retained"])
        self.assertFalse(policy["secret_values_retained"])
        self.assertFalse(policy["model_narration_used"])
        encoded = CORPUS.read_text(encoding="utf-8")
        for prohibited in (
            '"stdout"',
            '"stderr"',
            '"raw_output"',
            '"secret_value"',
            '"credential"',
            '"model_result"',
        ):
            self.assertNotIn(prohibited, encoded)

    def test_every_case_matches_closed_precedence_and_safe_shape(self) -> None:
        exact_keys = {
            "affected_path_intersects_change",
            "baseline_failure_match",
            "baseline_known_clean",
            "case_id",
            "expected_classification",
            "failure_sha256",
            "platform_code",
            "rationale_code",
            "retry_count",
            "status",
        }
        for case in self.corpus["cases"]:
            self.assertEqual(set(case), exact_keys, case["case_id"])
            self.assertRegex(case["failure_sha256"], SHA256)
            self.assertEqual(classify(case), case["expected_classification"])
            self.assertEqual(
                case["retry_count"] > 0,
                case["expected_classification"] == "flaky",
            )

    def test_mutated_precedence_cannot_false_classify(self) -> None:
        baseline = next(
            case
            for case in self.corpus["cases"]
            if case["expected_classification"] == "baseline"
        )
        self.assertTrue(baseline["affected_path_intersects_change"])
        self.assertEqual(classify(baseline), "baseline")
        changed = dict(baseline)
        changed["platform_code"] = "fixture.permission-denied"
        self.assertEqual(classify(changed), "permission")


if __name__ == "__main__":
    unittest.main()

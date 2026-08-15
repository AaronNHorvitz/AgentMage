from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "docs/verification/sprint-49-routing-corpus.json"
EXPECTED_BOUNDARIES = {
    "exact_profile",
    "manual_selection",
    "remote_path",
    "role_evidence",
    "routing",
    "second_verifier",
}


class Sprint49RoutingCorpusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.corpus = json.loads(CORPUS.read_text(encoding="utf-8"))

    def test_corpus_is_closed_unique_and_fail_closed(self) -> None:
        self.assertEqual(
            set(self.corpus), {"schema_version", "record_type", "cases"}
        )
        self.assertEqual(self.corpus["schema_version"], 1)
        self.assertEqual(
            self.corpus["record_type"],
            "sprint_49_measured_routing_security_corpus",
        )
        cases = self.corpus["cases"]
        self.assertGreaterEqual(len(cases), 48)
        identities = [case["id"] for case in cases]
        self.assertEqual(len(identities), len(set(identities)))
        self.assertEqual({case["boundary"] for case in cases}, EXPECTED_BOUNDARIES)
        self.assertEqual({case["expected"] for case in cases}, {"denied"})
        for case in cases:
            self.assertEqual(
                set(case), {"id", "boundary", "mutation", "expected"}
            )
            for field in ("id", "boundary", "mutation", "expected"):
                self.assertRegex(case[field], r"^[a-z][a-z0-9_-]{2,127}$")

    def test_each_security_boundary_has_multiple_mutations(self) -> None:
        counts = {
            boundary: sum(
                case["boundary"] == boundary for case in self.corpus["cases"]
            )
            for boundary in EXPECTED_BOUNDARIES
        }
        self.assertGreaterEqual(counts["exact_profile"], 16)
        self.assertGreaterEqual(counts["role_evidence"], 8)
        self.assertGreaterEqual(counts["routing"], 11)
        self.assertGreaterEqual(counts["manual_selection"], 3)
        self.assertGreaterEqual(counts["second_verifier"], 4)
        self.assertGreaterEqual(counts["remote_path"], 6)

    def test_corpus_contains_no_secrets_raw_results_prompts_or_locations(self) -> None:
        encoded = json.dumps(self.corpus, sort_keys=True)
        for prohibited in (
            "private_key",
            "secret_value",
            "credential_value",
            "raw_output",
            "raw_result",
            "prompt_text",
            "repository_path",
            "remote_url",
            "access_token",
        ):
            self.assertNotIn(prohibited, encoded)


if __name__ == "__main__":
    unittest.main()

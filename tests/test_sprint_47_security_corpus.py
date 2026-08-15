from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "docs/verification/sprint-47-security-corpus.json"
EXPECTED_BOUNDARIES = {
    "candidate_tree",
    "local_commit",
    "local_commit_plan",
    "logical_commit_plan",
    "manual_approval",
    "review_packet",
}


class Sprint47SecurityCorpusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.corpus = json.loads(CORPUS.read_text(encoding="utf-8"))

    def test_corpus_is_closed_unique_and_fail_closed(self) -> None:
        self.assertEqual(self.corpus["schema_version"], 1)
        self.assertEqual(
            self.corpus["record_type"], "sprint_47_local_security_corpus"
        )
        cases = self.corpus["cases"]
        self.assertGreaterEqual(len(cases), 28)
        identities = [case["id"] for case in cases]
        self.assertEqual(len(identities), len(set(identities)))
        self.assertEqual({case["boundary"] for case in cases}, EXPECTED_BOUNDARIES)
        self.assertEqual({case["expected"] for case in cases}, {"denied"})
        for case in cases:
            self.assertEqual(
                set(case), {"id", "boundary", "mutation", "expected"}
            )
            for field in ("id", "boundary", "mutation"):
                self.assertRegex(case[field], r"^[a-z][a-z0-9_-]{2,127}$")

    def test_corpus_contains_no_secrets_commands_or_raw_content(self) -> None:
        encoded = json.dumps(self.corpus, sort_keys=True)
        for prohibited in (
            "private_key",
            "secret_value",
            "credential_value",
            "raw_output",
            "command_argv",
            "repository_path",
            "remote_url",
        ):
            self.assertNotIn(prohibited, encoded)


if __name__ == "__main__":
    unittest.main()

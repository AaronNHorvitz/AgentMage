from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "docs/verification/sprint-48-headless-corpus.json"
EXPECTED_BOUNDARIES = {
    "authority",
    "cancellation",
    "event_stream",
    "native_effect",
    "request",
    "transport",
}
EXPECTED_OUTCOMES = {"denied", "uncertain"}


class Sprint48HeadlessCorpusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.corpus = json.loads(CORPUS.read_text(encoding="utf-8"))

    def test_corpus_is_closed_unique_and_fail_closed(self) -> None:
        self.assertEqual(self.corpus["schema_version"], 1)
        self.assertEqual(
            self.corpus["record_type"], "sprint_48_headless_security_corpus"
        )
        self.assertEqual(set(self.corpus), {"schema_version", "record_type", "cases"})
        cases = self.corpus["cases"]
        self.assertGreaterEqual(len(cases), 40)
        identities = [case["id"] for case in cases]
        self.assertEqual(len(identities), len(set(identities)))
        self.assertEqual({case["boundary"] for case in cases}, EXPECTED_BOUNDARIES)
        self.assertEqual({case["expected"] for case in cases}, EXPECTED_OUTCOMES)
        for case in cases:
            self.assertEqual(
                set(case), {"id", "boundary", "mutation", "expected"}
            )
            for field in ("id", "boundary", "mutation", "expected"):
                self.assertRegex(case[field], r"^[a-z][a-z0-9_-]{2,127}$")

    def test_every_boundary_has_multiple_independent_mutations(self) -> None:
        counts = {
            boundary: sum(
                case["boundary"] == boundary for case in self.corpus["cases"]
            )
            for boundary in EXPECTED_BOUNDARIES
        }
        self.assertGreaterEqual(counts["request"], 8)
        self.assertGreaterEqual(counts["authority"], 9)
        self.assertGreaterEqual(counts["event_stream"], 12)
        self.assertGreaterEqual(counts["transport"], 4)
        self.assertGreaterEqual(counts["cancellation"], 3)
        self.assertGreaterEqual(counts["native_effect"], 4)

    def test_corpus_contains_no_secrets_commands_paths_or_raw_content(self) -> None:
        encoded = json.dumps(self.corpus, sort_keys=True)
        for prohibited in (
            "private_key",
            "secret_value",
            "credential_value",
            "raw_output",
            "command_argv",
            "repository_path",
            "remote_url",
            "prompt_text",
            "socket_path",
            "access_token",
        ):
            self.assertNotIn(prohibited, encoded)


if __name__ == "__main__":
    unittest.main()

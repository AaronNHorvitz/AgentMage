from __future__ import annotations

import json
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "fixtures/runtime/v1/coding-harness-mvp.json"
EXPECTED_CASES = {
    "S-048-MVP-E2E": {
        "repository-exploration",
        "one-file-patch",
        "controlled-new-file",
        "focused-bug-fix",
        "targeted-test",
        "failed-test-before-revision",
        "revised-change-after-failure",
        "user-denial",
        "user-cancellation",
        "admitted-local-model-workflow",
    },
    "S-048-MVP-STALE": {
        "workspace",
        "worktree",
        "base-commit",
        "path",
        "preimage",
        "plan",
        "tool-schema",
        "command-arguments",
        "model-profile",
        "policy",
        "grant",
        "preservation-manifest",
    },
    "S-048-MVP-ADVERSARIAL": {
        "path-traversal",
        "symlink-substitution",
        "hostile-instructions",
        "command-metacharacters",
        "environment-leakage",
        "git-hooks-and-filters",
        "output-forgery",
        "repeated-tool-calls",
        "malformed-model-proposal",
        "oversized-output",
        "false-completion",
        "failed-validation-forgery",
    },
    "S-048-MVP-ABSENCE": {
        "persistent-session-resume",
        "durable-journal",
        "complete-artifact-lifecycle",
        "remote-git",
        "git-commit",
        "git-push",
        "automatic-routing",
        "mcp-dependency",
        "browser-and-network",
        "package-installation",
        "complete-conversation-library",
        "workflow-execution",
        "child-agents",
        "autonomous-publication",
    },
}
EXPECTED_OUTCOMES = {
    "absent",
    "bounded_non_success",
    "cancelled_without_effect",
    "declined_without_effect",
    "denied_without_effect",
    "inert",
    "inert_or_narrowing_only",
    "truthful_failure",
    "truthful_non_success",
    "verified_no_op",
    "verified_success",
}


class Story48CodingHarnessCorpusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.corpus = json.loads(CORPUS.read_text(encoding="utf-8"))

    def test_four_named_groups_have_the_exact_required_cases(self) -> None:
        self.assertEqual(set(self.corpus), {"schema_version", "milestone", "groups"})
        self.assertEqual(self.corpus["schema_version"], 1)
        self.assertEqual(self.corpus["milestone"], "M-HARNESS-MVP")
        groups = self.corpus["groups"]
        self.assertEqual([group["id"] for group in groups], list(EXPECTED_CASES))
        for group in groups:
            self.assertEqual(set(group), {"id", "cases"})
            cases = group["cases"]
            self.assertEqual(
                {case["id"] for case in cases}, EXPECTED_CASES[group["id"]]
            )
            self.assertEqual(len(cases), len(EXPECTED_CASES[group["id"]]))
            for case in cases:
                self.assertEqual(
                    set(case),
                    {"id", "expected", "evidence_test", "prerequisite"},
                )
                self.assertRegex(case["id"], r"^[a-z][a-z0-9-]{2,127}$")
                self.assertIn(case["expected"], EXPECTED_OUTCOMES)

    def test_every_locally_executable_case_names_real_test_evidence(self) -> None:
        rust_sources = "\n".join(
            path.read_text(encoding="utf-8")
            for root in ("kernel", "capabilities", "platforms", "shells")
            for path in sorted((ROOT / root).rglob("*.rs"))
        )
        blocked = []
        for group in self.corpus["groups"]:
            for case in group["cases"]:
                evidence = case["evidence_test"]
                prerequisite = case["prerequisite"]
                if prerequisite is None:
                    self.assertIsInstance(evidence, str)
                    self.assertRegex(evidence, r"^[a-z][a-z0-9_]{2,255}$")
                    self.assertRegex(rust_sources, rf"\bfn\s+{re.escape(evidence)}\s*\(")
                else:
                    blocked.append((group["id"], case["id"], prerequisite, evidence))
        self.assertEqual(
            blocked,
            [
                (
                    "S-048-MVP-E2E",
                    "admitted-local-model-workflow",
                    "sprint-49-exact-runtime-admission",
                    None,
                )
            ],
        )

    def test_fixture_contains_no_authority_or_sensitive_payloads(self) -> None:
        encoded = json.dumps(self.corpus, sort_keys=True)
        for prohibited in (
            "arguments_sha256",
            "command_argv",
            "credential_value",
            "grant_id",
            "private_key",
            "raw_content",
            "remote_url",
            "repository_path",
            "secret",
            "token_value",
        ):
            self.assertNotIn(prohibited, encoded)


if __name__ == "__main__":
    unittest.main()

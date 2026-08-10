from __future__ import annotations

import json
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "documentation.yml"


class DocumentationControlTests(unittest.TestCase):
    def test_documentation_tools_are_exactly_pinned_and_locked(self) -> None:
        package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
        dependencies = package["devDependencies"]
        lock_dependencies = lock["packages"][""]["devDependencies"]

        for name in (
            "@mermaid-js/mermaid-cli",
            "ajv",
            "ajv-formats",
            "markdownlint-cli2",
        ):
            with self.subTest(package=name):
                version = dependencies[name]
                self.assertRegex(version, r"^\d+\.\d+\.\d+$")
                self.assertEqual(lock_dependencies[name], version)
                self.assertEqual(lock["packages"][f"node_modules/{name}"]["version"], version)
                self.assertIn(
                    "integrity",
                    lock["packages"][f"node_modules/{name}"],
                )

    def test_documentation_workflow_uses_immutable_actions_and_canonical_gate(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        action_references = re.findall(r"^\s*uses:\s*([^\s]+)$", workflow, re.MULTILINE)

        self.assertTrue(action_references)
        for reference in action_references:
            with self.subTest(action=reference):
                self.assertRegex(reference, r"^[^@\s]+@[0-9a-f]{40}$")

        self.assertIn("permissions:\n  contents: read", workflow)
        self.assertIn("runs-on: ubuntu-24.04", workflow)
        self.assertIn("run: npm ci --ignore-scripts", workflow)
        self.assertIn("run: npm run docs:check", workflow)

    def test_mermaid_parser_uses_only_the_locked_local_cli(self) -> None:
        checker = (ROOT / "scripts" / "check_mermaid.py").read_text(encoding="utf-8")

        self.assertIn('ROOT / "node_modules" / ".bin" / "mmdc"', checker)
        self.assertNotIn("npx", checker)
        self.assertIn("TemporaryDirectory", checker)


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import json
import re
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.validate_docs import (
    CANONICAL_DOCS,
    DECISION_BOUNDARIES,
    ROOT as VALIDATOR_ROOT,
    check_accepted_decision_contract,
    check_claims,
    check_cross_document_contract,
    check_identifiers,
    check_links,
    check_required,
    check_sensitive,
    main as validate_docs,
)


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "documentation.yml"


class DocumentationControlTests(unittest.TestCase):
    @staticmethod
    def is_ignored(relative: str) -> bool:
        result = subprocess.run(
            ["git", "check-ignore", "--no-index", "--quiet", "--", relative],
            cwd=ROOT,
            check=False,
        )
        if result.returncode not in (0, 1):
            raise AssertionError(f"git check-ignore failed for {relative}")
        return result.returncode == 0

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
        self.assertIn("run: npm run docs:clean-check", workflow)
        self.assertNotRegex(workflow, r"run: npm (?:ci --ignore-scripts|run docs:check)$")

    def test_one_command_reproduces_the_clean_documentation_gate(self) -> None:
        package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        readme = (ROOT / "README.md").read_text(encoding="utf-8")

        self.assertEqual(
            package["scripts"]["docs:clean-check"],
            "npm ci --ignore-scripts && npm run docs:check",
        )
        self.assertIn("npm run docs:clean-check", readme)
        self.assertNotIn("npm ci --ignore-scripts\nnpm run docs:check", readme)

    def test_mermaid_parser_uses_only_the_locked_local_cli(self) -> None:
        checker = (ROOT / "scripts" / "check_mermaid.py").read_text(encoding="utf-8")

        self.assertIn('ROOT / "node_modules" / ".bin" / "mmdc"', checker)
        self.assertNotIn("npx", checker)
        self.assertIn("TemporaryDirectory", checker)

    def test_canonical_documentation_passes_every_local_integrity_check(self) -> None:
        self.assertEqual(validate_docs(), 0)

        failures: list[str] = []
        check_required(failures)
        check_cross_document_contract(failures)
        check_accepted_decision_contract(failures)
        self.assertEqual(failures, [])

    def test_each_accepted_decision_boundary_detects_canonical_drift(self) -> None:
        canonical = {
            relative: (VALIDATOR_ROOT / relative).read_text(encoding="utf-8")
            for relative in CANONICAL_DOCS
        }

        for boundary, patterns in DECISION_BOUNDARIES.items():
            with self.subTest(boundary=boundary):
                changed = dict(canonical)
                mutated = changed["README.md"]
                for pattern in patterns:
                    mutated = pattern.sub("REMOVED-BOUNDARY", mutated)
                changed["README.md"] = mutated
                failures: list[str] = []

                check_accepted_decision_contract(failures, documents=changed)

                self.assertTrue(
                    any(
                        item.startswith(f"README.md: {boundary} boundary")
                        for item in failures
                    ),
                    failures,
                )

    def test_local_link_check_rejects_missing_and_escaping_targets(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            document = root / "document.md"
            document.write_text(
                "[valid](target.md) [missing](absent.md) [escape](../outside.md)\n",
                encoding="utf-8",
            )
            (root / "target.md").write_text("# Target\n", encoding="utf-8")
            failures: list[str] = []

            with patch("scripts.validate_docs.ROOT", root):
                check_links([document], failures)

        self.assertEqual(len(failures), 2)
        self.assertTrue(any("broken local link" in item for item in failures))
        self.assertTrue(any("link leaves repository" in item for item in failures))

    def test_sensitive_check_names_rule_without_echoing_matched_value(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            document = root / "synthetic.txt"
            synthetic_value = "gh" + "p_" + "0123456789abcdefghijklmnop"
            document.write_text(f"token={synthetic_value}\n", encoding="utf-8")
            failures: list[str] = []

            with patch("scripts.validate_docs.ROOT", root):
                check_sensitive([document], failures)

        self.assertEqual(len(failures), 1)
        self.assertIn("possible GitHub token", failures[0])
        self.assertNotIn(synthetic_value, failures[0])

    def test_claim_and_identifier_checks_report_the_exact_rule(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT) as temp_dir:
            root = Path(temp_dir)
            claims = root / "claim.md"
            claims.write_text("A prohibited federal deployment claim.\n", encoding="utf-8")
            identifiers = root / "identifier.md"
            identifiers.write_text("Unknown requirement AM-MISSING-999.\n", encoding="utf-8")
            claim_failures: list[str] = []
            identifier_failures: list[str] = []

            check_claims([claims], claim_failures)
            check_identifiers([identifiers], identifier_failures)

        self.assertEqual(len(claim_failures), 1)
        self.assertIn("prohibited deployment-specific claim", claim_failures[0])
        self.assertIn(
            "unresolved stable identifier: AM-MISSING-999",
            identifier_failures,
        )

    def test_private_and_generated_local_artifacts_are_ignored(self) -> None:
        ignored_paths = (
            ".env",
            ".env.local",
            "credentials/private.key",
            "credentials/certificate.pem",
            "credentials/identity.p12",
            "credentials/signing.pfx",
            "profile.mobileprovision",
            "models/candidate.gguf",
            ".local-models/candidate.safetensors",
            "runtime/model.onnx",
            "runtime/model.mlmodelc/weights.bin",
            "state/agentmage.sqlite",
            "state/agentmage.sqlite3",
            "state/cache.db",
            "state/cache.db-wal",
            "state/cache.db-shm",
            "logs/session.log",
            "artifacts/sprints/sprint-1/private-result.json",
            "review-evidence/private-report.json",
            "node_modules/package/index.js",
            "scripts/__pycache__/module.pyc",
            "target/debug/agentmage",
            "build/output.bin",
            "dist/agentmage.tar",
            "coverage/report.json",
            ".mermaid-output/diagram.svg",
            ".vscode-test/settings.json",
            "scratch.tmp",
            "buffer.swp",
            ".DS_Store",
            "Thumbs.db",
        )

        for relative in ignored_paths:
            with self.subTest(path=relative):
                self.assertTrue(self.is_ignored(relative))

    def test_canonical_records_and_admitted_evidence_are_not_ignored(self) -> None:
        versioned_paths = (
            ".env.example",
            "README.md",
            "PRD.md",
            "IMPLEMENTATION-PLAN.md",
            "Agent-Scaffolding-Inventory.md",
            "TASKS.md",
            "SECURITY-REVIEW.md",
            "SECURITY.md",
            "MODEL-PROVENANCE-POLICY.md",
            "RUNTIME-BOUNDARIES.md",
            "requirements/registry.json",
            "scripts/validate_docs.py",
            "tests/test_documentation_controls.py",
            ".github/workflows/documentation.yml",
            "artifacts/sprints/sprint-0/story-0.1/evidence-manifest.json",
            "artifacts/sprints/sprint-0/story-0.2/evidence-manifest.json",
        )

        for relative in versioned_paths:
            with self.subTest(path=relative):
                self.assertFalse(self.is_ignored(relative))


if __name__ == "__main__":
    unittest.main()

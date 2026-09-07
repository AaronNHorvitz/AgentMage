from __future__ import annotations

import json
import re
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.story_0_2_evidence import DEFAULT_OUTPUT
from scripts.validate_docs import (
    CANONICAL_DOCS,
    DECISION_BOUNDARIES,
    ROOT,
    check_accepted_decision_contract,
    check_sensitive,
)


class Story02AcceptanceTests(unittest.TestCase):
    def test_ac1_local_and_ci_entry_points_have_identical_blocking_semantics(self) -> None:
        package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        workflow = (ROOT / ".github/workflows/documentation.yml").read_text(
            encoding="utf-8"
        )
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        identity = json.loads(
            (DEFAULT_OUTPUT / "workflow-identity.json").read_text(encoding="utf-8")
        )
        raw = json.loads(
            (DEFAULT_OUTPUT / "raw-checker-output.json").read_text(encoding="utf-8")
        )

        self.assertEqual(
            package["scripts"]["docs:clean-check"],
            "npm ci --ignore-scripts && npm run docs:check",
        )
        self.assertIn("if: ${{ false }}", workflow)
        self.assertNotIn("run: npm run docs:clean-check", workflow)
        self.assertIn("npm run docs:clean-check", readme)
        self.assertEqual(identity["gate_command"], "npm run docs:clean-check")
        self.assertEqual(identity["gate_script"], package["scripts"]["docs:clean-check"])
        self.assertEqual(identity["execution_mode"], "local-only-no-runner-sentinel")
        self.assertTrue(identity["sentinel_job_disabled"])
        self.assertTrue(identity["manual_trigger_only"])
        self.assertTrue(identity["actions_immutable"])
        self.assertEqual(raw["summary"]["failed"], 0)
        self.assertEqual(raw["summary"]["skipped"], 0)

    def test_ac2_public_policies_control_release_and_model_review_evidence(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        license_text = (ROOT / "LICENSE").read_text(encoding="utf-8")
        security = (ROOT / "SECURITY.md").read_text(encoding="utf-8")
        provenance = (ROOT / "MODEL-PROVENANCE-POLICY.md").read_text(encoding="utf-8")
        runtime = (ROOT / "RUNTIME-BOUNDARIES.md").read_text(encoding="utf-8")
        release_schema = json.loads(
            (ROOT / "schemas/planning/release-manifest.schema.json").read_text(
                encoding="utf-8"
            )
        )

        for public_path in (
            "LICENSE",
            "SECURITY.md",
            "MODEL-PROVENANCE-POLICY.md",
            "RUNTIME-BOUNDARIES.md",
        ):
            self.assertIn(public_path, readme)
        self.assertIn("Business Source License 1.1", license_text)
        self.assertIn("## Supported Versions", security)
        self.assertIn("## Reporting a Vulnerability", security)
        self.assertIn("signed release manifest", security)
        self.assertIn("## 5. Required Admission Record", provenance)
        for evidence_field in ("License", "Runtime and codec", "Decision"):
            self.assertIn(f"| {evidence_field} |", provenance)
        self.assertIn("## 9. Reviewer Evidence", runtime)
        for required_field in (
            "components",
            "model_profiles",
            "documents",
            "evidence",
            "signer",
        ):
            self.assertIn(required_field, release_schema["required"])

    def test_ac3_policy_drift_and_secret_diagnostics_are_exact_and_redacted(self) -> None:
        canonical = {
            relative: (ROOT / relative).read_text(encoding="utf-8")
            for relative in CANONICAL_DOCS
        }
        changed = dict(canonical)
        changed["README.md"] = DECISION_BOUNDARIES["runtime"][2].sub(
            "REMOVED-RUNTIME-CONTRACT",
            changed["README.md"],
        )
        policy_failures: list[str] = []

        check_accepted_decision_contract(policy_failures, documents=changed)

        self.assertIn(
            "README.md: runtime boundary disagrees with Decision 0001",
            policy_failures,
        )

        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            document = root / "sensitive-fixture.txt"
            synthetic_value = "s" + "k-" + ("z" * 24)
            document.write_text(f"value={synthetic_value}\n", encoding="utf-8")
            secret_failures: list[str] = []
            with patch("scripts.validate_docs.ROOT", root):
                check_sensitive([document], secret_failures)

        self.assertEqual(len(secret_failures), 1)
        self.assertRegex(
            secret_failures[0],
            re.compile(r"sensitive-fixture\.txt: possible OpenAI-style token at character \d+"),
        )
        self.assertNotIn(synthetic_value, secret_failures[0])


if __name__ == "__main__":
    unittest.main()

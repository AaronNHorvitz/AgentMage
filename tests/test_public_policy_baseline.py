from __future__ import annotations

import hashlib
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class PublicPolicyBaselineTests(unittest.TestCase):
    def test_apache_2_license_is_complete_pinned_and_consistent(self) -> None:
        license_bytes = (ROOT / "LICENSE").read_bytes()
        license_text = license_bytes.decode("utf-8")
        package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))

        self.assertEqual(
            hashlib.sha256(license_bytes).hexdigest(),
            "02f41e321c6eabad29b0f412b9aaa710dfcff9df7fa563a8db0a408e35e5ba6f",
        )
        self.assertIn("Apache License", license_text)
        self.assertIn("Version 2.0, January 2004", license_text)
        for section in range(1, 10):
            self.assertIn(f"   {section}.", license_text)
        self.assertIn("END OF TERMS AND CONDITIONS", license_text)
        self.assertEqual(package["license"], "Apache-2.0")

        expected_references = {
            "README.md": "[Apache License 2.0](./LICENSE)",
            "PRD.md": "| **License** | Apache License 2.0 |",
            "IMPLEMENTATION-PLAN.md": "Apache-2.0 licensing",
            "Agent-Scaffolding-Inventory.md": "Apache-2.0 license",
            "TASKS.md": "Publish the Apache License 2.0",
            "docs/decisions/0001-product-security-and-runtime-baseline.md": (
                "distributed under the Apache License 2.0"
            ),
        }
        for relative, expected in expected_references.items():
            with self.subTest(document=relative):
                self.assertIn(
                    expected,
                    (ROOT / relative).read_text(encoding="utf-8"),
                )

    def test_security_policy_covers_the_complete_public_response_lifecycle(self) -> None:
        policy = (ROOT / "SECURITY.md").read_text(encoding="utf-8")

        for heading in (
            "## Supported Versions",
            "## Reporting a Vulnerability",
            "## Triage and Disclosure",
            "## Remediation and Patch Delivery",
            "## Emergency Disablement",
            "## Incident Handling",
            "## Scope",
        ):
            self.assertIn(heading, policy)
        for required_promise in (
            "No production binary is supported yet.",
            "Use GitHub private vulnerability reporting",
            "establish an initial severity and affected-version disposition",
            "new signed release",
            "Manual installation or replacement instructions",
            "performs no automatic update check",
            "local, user-controlled disable path",
            "There is no remote kill switch.",
            "support end recorded in its signed release manifest",
            "Unsupported versions receive no remediation commitment",
        ):
            with self.subTest(promise=required_promise):
                self.assertIn(required_promise, policy)
        for linked_policy in (
            "[SECURITY-REVIEW.md](./SECURITY-REVIEW.md)",
            "[MODEL-PROVENANCE-POLICY.md](./MODEL-PROVENANCE-POLICY.md)",
            "[RUNTIME-BOUNDARIES.md](./RUNTIME-BOUNDARIES.md)",
        ):
            self.assertIn(linked_policy, policy)

    def test_model_provenance_policy_covers_admission_and_revocation(self) -> None:
        policy = (ROOT / "MODEL-PROVENANCE-POLICY.md").read_text(encoding="utf-8")

        for heading in (
            "## 2. Authority and Change Control",
            "## 3. Covered Supply Chain",
            "## 4. Origin and Lineage Rule",
            "## 5. Required Admission Record",
            "## 6. Runtime Parity",
            "## 7. Initial and Fallback Profiles",
            "## 8. Re-Review Triggers",
        ):
            self.assertIn(heading, policy)
        for category in (
            "Identity",
            "Ownership and origin",
            "License",
            "Artifacts",
            "Transformation",
            "Runtime",
            "Resources",
            "Quality",
            "Security",
            "Decision",
        ):
            self.assertIn(f"| {category} |", policy)
        for required_rule in (
            "Unknown, contradictory, stale, or unverifiable evidence produces `BLOCKED`",
            "does not create new provenance",
            "never serve as release identity",
            "pinned by immutable OCI digest",
            "pinned by GGUF and supporting-file hashes",
            "same approved model profile",
            "initial candidate, not a pre-approved dependency",
            "named fallback candidate",
            "AgentMage never switches to it automatically",
            "vulnerability, compromise, revocation",
            "Admission expires and the profile is disabled or quarantined",
        ):
            with self.subTest(rule=required_rule):
                self.assertIn(required_rule, policy)


if __name__ == "__main__":
    unittest.main()

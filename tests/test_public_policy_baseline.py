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

    def test_runtime_boundaries_cover_platforms_processes_sockets_and_lifecycle(self) -> None:
        boundaries = (ROOT / "RUNTIME-BOUNDARIES.md").read_text(encoding="utf-8")

        for heading in (
            "## 2. Trust Boundaries and Data Flow",
            "## 3. Process Inventory",
            "## 4. Privilege and Installation",
            "### macOS",
            "### Fedora and Ubuntu",
            "## 5. Local IPC and Socket Inventory",
            "## 6. Lifecycle",
            "## 7. Runtime Parity Gate",
            "## 8. Reviewer Evidence",
        ):
            self.assertIn(heading, boundaries)
        for process in (
            "Visual Studio Code extension",
            "Native bridge",
            "AgentMage kernel",
            "Sandboxed tool worker",
            "Native model service",
            "Docker Model Runner",
            "Model installer/importer",
            "Review verifier",
        ):
            self.assertIn(f"| {process} |", boundaries)
        for connection in (
            "VS Code extension to bridge/kernel",
            "Kernel to tool worker",
            "Kernel to native model adapter",
            "Kernel adapter to Docker Model Runner",
        ):
            self.assertIn(f"| {connection} |", boundaries)
        for required_boundary in (
            "an unavailable mandatory control blocks the profile",
            "Native `llama.cpp` is the security-reference inference adapter",
            "Docker Model Runner's API is unauthenticated",
            "remains `BLOCKED` unless",
            "Offline startup verifies package, platform, policy, workspace, storage",
            "No AgentMage process silently persists as a system-wide daemon",
            "Neither adapter may borrow another adapter's result",
        ):
            with self.subTest(boundary=required_boundary):
                self.assertIn(required_boundary, boundaries)

    def test_accepted_baseline_decision_is_complete_and_independently_recorded(self) -> None:
        decision = (
            ROOT / "docs" / "decisions" / "0001-product-security-and-runtime-baseline.md"
        ).read_text(encoding="utf-8")

        self.assertIn("| Status | Accepted |", decision)
        self.assertIn("| Date | 2026-08-10 |", decision)
        self.assertIn("An external planning audit remains outside this repository.", decision)
        self.assertIn(
            "AgentMage incorporates only independently evaluated product findings",
            decision,
        )
        self.assertIn("Existing stable identifiers remain unchanged.", decision)
        for item in range(1, 13):
            self.assertIn(f"{item}.", decision)
        for governed_file in (
            "MODEL-PROVENANCE-POLICY.md",
            "SECURITY.md",
            "RUNTIME-BOUNDARIES.md",
        ):
            self.assertIn(governed_file, decision)
        for prohibited_import_marker in (
            "Findings Critical:",
            "Delivery-and-Federal-Security Audit",
            "Claude's remediation plan",
        ):
            self.assertNotIn(prohibited_import_marker, decision)


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import copy
import unittest

from scripts.artifact_scanner import (
    EXPECTED_SEEDS,
    build_report,
    load_policy,
    scan_sbom,
    scan_text,
    validate_report,
)


class ArtifactScannerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_all_four_artifact_surfaces_are_scanned(self) -> None:
        self.assertEqual(
            {item["surface"] for item in self.report["surfaces"]},
            {"source", "build-output", "package", "sbom"},
        )

    def test_canonical_surfaces_have_no_blocking_findings(self) -> None:
        self.assertEqual(self.report["summary"]["blocking_findings"], 0)

    def test_development_license_gap_remains_visible(self) -> None:
        findings = [
            finding
            for surface in self.report["surfaces"]
            for finding in surface["findings"]
            if finding["category"] == "license-review"
        ]
        self.assertTrue(findings)
        self.assertTrue(all(item["severity"] == "open-review" for item in findings))

    def test_approved_conjunctive_license_is_not_blocked(self) -> None:
        bom = {
            "components": [
                {
                    "bom-ref": "cargo:synthetic@1.0.0",
                    "licenses": [{"expression": "MIT AND BSD-3-Clause"}],
                    "properties": [
                        {
                            "name": "agentmage:dependency-class",
                            "value": "production",
                        }
                    ],
                }
            ]
        }
        policy = load_policy()
        self.assertEqual(scan_sbom(bom, policy)["findings"], [])

    def test_every_seeded_violation_is_detected(self) -> None:
        self.assertEqual(
            {item["seed"] for item in self.report["seeded_cases"]},
            set(EXPECTED_SEEDS),
        )
        for case in self.report["seeded_cases"]:
            with self.subTest(seed=case["seed"]):
                self.assertEqual(case["status"], "pass")
                self.assertIn(case["seed"], case["observed_categories"])

    def test_secret_scanner_does_not_echo_secret_value(self) -> None:
        value = "synthetic-value-never-echo"
        findings = scan_text(
            "token = '" + value + "'",
            "source",
            "synthetic/secret.txt",
        )
        self.assertTrue(findings)
        self.assertNotIn(value, str(findings))

    def test_defensive_docker_socket_comparison_is_not_privilege_assumption(self) -> None:
        findings = scan_text(
            'if mount.source == "/var/run/docker.sock" { reject(); }',
            "source",
            "synthetic/defensive.rs",
        )
        self.assertNotIn(
            "privileged-assumption", {item["category"] for item in findings}
        )

    def test_configured_docker_socket_is_privilege_assumption(self) -> None:
        findings = scan_text(
            'const DOCKER_SOCKET: &str = "/var/run/docker.sock";',
            "source",
            "synthetic/configured.rs",
        )
        self.assertIn(
            "privileged-assumption", {item["category"] for item in findings}
        )

    def test_missing_seed_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["seeded_cases"].pop()
        self.assertTrue(validate_report(mutated))

    def test_macos_status_cannot_be_promoted(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["platform_status"]["macos_binary_scan"] = "scanned"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()

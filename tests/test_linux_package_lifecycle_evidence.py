"""Tests for source-bound clean Linux package lifecycle evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import linux_package_lifecycle_evidence as evidence
from tests.test_package_lifecycle import PackageLifecycleTests


class LinuxPackageLifecycleEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        lifecycle = PackageLifecycleTests().valid_lifecycle()
        manifests = []
        for version in ("0.0.0", "0.0.1"):
            manifests.append(
                {
                    "version": version,
                    "sha256": "d" * 64,
                    "manifest": {
                        "schema_version": 1,
                        "record_type": "agentmage-package-manifest",
                        "status": "unsigned-candidate",
                        "package_id": f"agentmage-linux-x86_64-{version}-candidate",
                        "files": [
                            {
                                "path": path.as_posix(),
                                "sha256": "e" * 64,
                                "size": 1,
                                "mode": 0o755 if index < 5 else 0o644,
                            }
                            for index, path in enumerate(sorted(evidence.PAYLOAD_FILES))
                        ],
                    },
                }
            )
        return {
            "schema_version": 1,
            "artifact_id": "linux-clean-package-lifecycle",
            "task_ids": ["9.1.1.7"],
            "status": "pass-linux-clean-package-lifecycle",
            "source": {
                "revision": "a" * 40,
                "tree": "b" * 40,
                "files": [
                    {"path": "Cargo.toml", "sha256": "c" * 64, "bytes": 1}
                ],
            },
            "clean_source_build": {
                "report_path": "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
                "report_sha256": "f" * 64,
                "source_revision": "a" * 40,
                "network_boundary": "disabled-after-bootstrap",
                "linux_platforms": [
                    {
                        "platform_id": platform_id,
                        "status": "pass",
                        "base_image": "registry.invalid/base@sha256:" + "1" * 64,
                        "container_image_id": "sha256:" + "2" * 64,
                        "command_ids": ["build", "test"],
                    }
                    for platform_id in evidence.EXPECTED_PLATFORMS
                ],
                "macos_status": "blocked-macos",
            },
            "packages": [
                {
                    "version": version,
                    "format": package_format,
                    "filename": f"agentmage-{version}.{package_format}",
                    "sha256": "3" * 64,
                    "bytes": 1,
                }
                for version in ("0.0.0", "0.0.1")
                for package_format in evidence.EXPECTED_FORMATS
            ],
            "component_manifests": manifests,
            "package_verification": {
                "extracted_payloads": "pass",
                "mutation_refusal": "pass",
            },
            "container_lifecycle": lifecycle,
            "package_build_network_used": False,
            "lifecycle_network_used": False,
            "private_values_present": False,
            "enabled_models": 0,
            "inference_available": False,
            "release_claim": "none",
            "limitations": list(evidence.LIMITATIONS),
        }

    def test_expected_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_report_rejects_platform_or_source_build_overclaim(self) -> None:
        report = self.valid_report()
        report["clean_source_build"]["linux_platforms"][1]["status"] = "not-run"
        report["container_lifecycle"]["platforms"][0]["runtime_user"] = "0:0"
        failures = evidence.validate_report(report)
        self.assertIn("clean source build closure is invalid", failures)
        self.assertIn("container lifecycle closure is invalid", failures)

    def test_report_rejects_network_model_or_release_overclaim(self) -> None:
        report = self.valid_report()
        report["package_build_network_used"] = True
        report["enabled_models"] = 1
        report["release_claim"] = "supported"
        self.assertIn(
            "network, private-value, model, or release state was overclaimed",
            evidence.validate_report(report),
        )

    def test_report_rejects_package_or_manifest_mutation(self) -> None:
        report = self.valid_report()
        report["packages"][0]["sha256"] = "invalid"
        report["component_manifests"][0]["manifest"]["files"].pop()
        failures = evidence.validate_report(report)
        self.assertIn("package artifact closure is invalid", failures)
        self.assertIn("component manifest closure is invalid", failures)

    def test_source_scope_includes_lifecycle_policy_and_evidence(self) -> None:
        required = {
            "scripts/package_lifecycle.py",
            "scripts/linux_package_lifecycle_evidence.py",
            "tests/test_package_lifecycle.py",
            "tests/test_linux_package_lifecycle_evidence.py",
        }
        self.assertEqual(required - evidence.SOURCE_EXACT, set())

    def test_limitations_are_exact(self) -> None:
        report = self.valid_report()
        report["limitations"] = copy.deepcopy(evidence.LIMITATIONS)
        report["limitations"].pop()
        self.assertIn(
            "Linux package lifecycle limitations changed",
            evidence.validate_report(report),
        )

    def test_malformed_nested_records_fail_without_raising(self) -> None:
        report = self.valid_report()
        report["source"] = "not-a-source-record"
        report["packages"][0] = "not-a-package-record"
        failures = evidence.validate_report(report)
        self.assertIn("Linux package lifecycle report identity changed", failures)
        self.assertIn("package artifact closure is invalid", failures)


if __name__ == "__main__":
    unittest.main()

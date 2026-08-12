"""Tests for the Sprint 9 inactive Linux inference package boundary."""

from __future__ import annotations

import copy
import tomllib
import unittest

from scripts import linux_inference_boundary_evidence as evidence


class LinuxInferenceBoundaryEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "linux-native-inference-package-boundary",
            "task_ids": ["9.1.1.6"],
            "status": "pass-fedora-package-boundary",
            "source_revision": "a" * 40,
            "sources": [
                {"path": path, "sha256": "b" * 64, "bytes": 1}
                for path in evidence.SOURCE_PATHS
            ],
            "host": {
                "distribution": "fedora",
                "version": "44",
                "architecture": "x86_64",
            },
            "compile_boundary": {
                "allowed_dependencies": ["agentmage-kernel-contracts"],
                "kernel_engine": False,
                "workspace_authority": False,
                "tool_authority": False,
                "grant_authority": False,
                "credential_authority": False,
            },
            "process_boundary": copy.deepcopy(evidence.EXPECTED_DESCRIPTOR),
            "adapter_binary": {
                "path_class": "package-owned-libexec",
                "sha256": "c" * 64,
                "bytes": 1,
                "mode": 0o755,
            },
            "packages": [
                {"format": kind, "sha256": "d" * 64, "bytes": 1}
                for kind in ("rpm", "deb", "vsix")
            ],
            "package_verification": {
                "extracted_payloads": "pass",
                "mutation_refusal": "pass",
            },
            "platform_status": {
                "fedora_44_x86_64": "verified-local",
                "ubuntu_26_04_x86_64": "format-built-and-extracted-not-clean-run",
                "macos": "blocked-macos",
            },
            "enabled_models": 0,
            "model_artifacts_packaged": False,
            "inference_started": False,
            "network_used": False,
            "private_values_present": False,
            "release_claim": "none",
            "limitations": list(evidence.LIMITATIONS),
        }

    def test_canonical_source_boundary_is_narrow(self) -> None:
        self.assertEqual(evidence.validate_source_boundary(), [])

    def test_manifest_rejects_authority_or_build_dependencies(self) -> None:
        manifest = tomllib.loads(
            (evidence.ROOT / evidence.ADAPTER_MANIFEST).read_text(encoding="utf-8")
        )
        mutated = copy.deepcopy(manifest)
        mutated["dependencies"]["agentmage-kernel-engine"] = {"workspace": True}
        self.assertIn(
            "adapter compile dependency closure changed",
            evidence.validate_adapter_manifest(mutated),
        )
        mutated = copy.deepcopy(manifest)
        mutated["build-dependencies"] = {"unknown": "1"}
        self.assertIn(
            "adapter gained a development or build dependency",
            evidence.validate_adapter_manifest(mutated),
        )

    def test_expected_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_model_or_authority_overclaim_fails(self) -> None:
        report = self.valid_report()
        report["enabled_models"] = 1
        report["inference_started"] = True
        self.assertIn(
            "model or inference state was overclaimed",
            evidence.validate_report(report),
        )
        report = self.valid_report()
        report["compile_boundary"]["workspace_authority"] = True
        self.assertIn(
            "adapter authority boundary changed",
            evidence.validate_report(report),
        )

    def test_package_or_platform_overclaim_fails(self) -> None:
        report = self.valid_report()
        report["packages"][0]["sha256"] = "invalid"
        self.assertIn(
            "package identity closure is invalid",
            evidence.validate_report(report),
        )
        report = self.valid_report()
        report["platform_status"]["ubuntu_26_04_x86_64"] = "verified-local"
        self.assertIn(
            "platform evidence status changed",
            evidence.validate_report(report),
        )


if __name__ == "__main__":
    unittest.main()

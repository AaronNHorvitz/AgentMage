"""Tests for packaged Docker guard/collector prerequisite evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import linux_docker_prerequisite_evidence as evidence


class LinuxDockerPrerequisiteEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        payload = [
            {
                "mode": 0o755 if index < 5 else 0o644,
                "path": path.as_posix(),
                "sha256": "b" * 64,
                "size": 1,
            }
            for index, path in enumerate(sorted(evidence.PAYLOAD_FILES))
        ]
        components = [
            {
                "collector_descriptor": copy.deepcopy(
                    evidence.EXPECTED_COLLECTOR_DESCRIPTOR
                ),
                "format": package_format,
                "guard_descriptor": copy.deepcopy(evidence.EXPECTED_GUARD_DESCRIPTOR),
                "manifest_sha256": "c" * 64,
                "payload": copy.deepcopy(payload),
            }
            for package_format in ("rpm", "deb")
        ]
        return {
            "artifact_id": "linux-docker-production-prerequisites",
            "components": components,
            "docker_engine_directly_tested": False,
            "host": {
                "architecture": "x86_64",
                "distribution": "fedora",
                "version": "44",
                "docker_cli_available": False,
                "live_collector_executed": False,
            },
            "limitations": list(evidence.LIMITATIONS),
            "live_topology_inspected": False,
            "mutation_coverage": copy.deepcopy(evidence.MUTATION_COVERAGE),
            "packages": {
                kind: {
                    "bytes": 1,
                    "filename": f"agentmage.{kind}",
                    "sha256": "d" * 64,
                }
                for kind in ("deb", "rpm", "vsix")
            },
            "reproducible_package_rebuild": True,
            "release_claim": "none",
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "e" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-packaged-prerequisites-no-live-docker",
            "task_ids": ["9.2.1.5"],
        }

    def test_exact_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_component_package_and_mutation_drift_fail_closed(self) -> None:
        report = self.valid_report()
        report["components"][0]["payload"].pop()
        report["packages"].pop("rpm")
        report["mutation_coverage"]["collector"].pop()
        failures = evidence.validate_report(report)
        self.assertIn("Docker prerequisite component closure changed", failures)
        self.assertIn("Docker prerequisite package closure changed", failures)
        self.assertIn("Docker prerequisite mutation closure changed", failures)

    def test_live_support_and_authority_overclaims_fail_closed(self) -> None:
        mutations = (
            lambda value: value.update({"docker_engine_directly_tested": True}),
            lambda value: value.update({"live_topology_inspected": True}),
            lambda value: value["host"].update({"docker_cli_available": True}),
            lambda value: value["host"].update({"live_collector_executed": True}),
            lambda value: value.update({"release_claim": "supported"}),
            lambda value: value["components"][0]["guard_descriptor"].update(
                {"docker_control": True}
            ),
            lambda value: value["components"][1]["collector_descriptor"].update(
                {"docker_mutation": True}
            ),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                report = self.valid_report()
                mutate(report)
                self.assertTrue(evidence.validate_report(report))

    def test_source_identity_requires_complete_ordered_records(self) -> None:
        report = self.valid_report()
        report["sources"].pop()
        self.assertIn(
            "Docker prerequisite source closure changed",
            evidence.validate_report(report),
        )


if __name__ == "__main__":
    unittest.main()

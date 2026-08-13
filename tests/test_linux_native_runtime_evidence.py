from __future__ import annotations

import copy
import unittest

from scripts import linux_native_runtime_evidence as evidence


class LinuxNativeRuntimeEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        members = [
            {"mode": 0o444, "name": f"retained-{index}.so", "target": None, "type": "file"}
            for index in range(28)
        ]
        libraries = [
            {
                "destination": f"lib/retained-{index}.so",
                "needed": ["libc.so.6"],
                "sha256": "b" * 64,
                "size_bytes": 1,
            }
            for index in range(17)
        ]
        return {
            "artifact_id": "linux-native-llama-runtime-package",
            "authority": dict.fromkeys(
                ("credential", "grant", "network", "tool", "workspace"), False
            ),
            "deterministic_rebuild_count": 2,
            "dynamic_libraries": libraries,
            "enabled_models": 0,
            "host": {"architecture": "x86_64", "distribution": "fedora", "version": "44"},
            "inference_started": False,
            "limitations": list(evidence.LIMITATIONS),
            "model_store": {
                "directory_enumeration": False,
                "directory_mode": 0o700,
                "input": "kernel-held-read-only-model-descriptors",
                "path_input": False,
                "replacement": "refuse-existing-or-drifted-identity",
                "required_owner": "invoking-standard-user",
                "runtime_write": False,
            },
            "network_used": False,
            "package": {
                "bytes": 1,
                "file_count": 28,
                "members": members,
                "package_id": "agentmage-llama-cpp-b10333-cpu-linux-x86_64",
                "sha256": "c" * 64,
            },
            "platform_status": {
                "fedora_44_x86_64": "verified-native-package-build",
                "ubuntu_26_04_x86_64": "not-run-this-subtask",
            },
            "process_boundary": copy.deepcopy(evidence.EXPECTED_DESCRIPTOR),
            "profile_sha256": "d" * 64,
            "readelf": {
                "executable_sha256": "e" * 64,
                "path_class": "maintainer-host-tool",
                "version": "GNU readelf test",
            },
            "release_claim": "none",
            "resource_ceiling": {
                "cpu_percent": 3200,
                "memory_bytes": 64 * 1024 * 1024 * 1024,
                "output_bytes": 16 * 1024 * 1024,
                "parallel_slots": 1,
                "runtime_seconds": 3600,
                "swap_bytes": 0,
                "tasks": 64,
            },
            "retained_regular_library_count": 17,
            "schema_version": 1,
            "source_archive": {
                "bytes": 32521550,
                "name": "llama-b10333-bin-ubuntu-vulkan-x64.tar.gz",
                "sha256": "f14e312fbee33ce60d2eed7036de5debe31c1d7f4d8f0e37920eb0a2de0854a5",
            },
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "f" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-fedora-native-package-boundary",
            "task_ids": ["9.2.1.1"],
            "upstream_entrypoints_included": [],
        }

    def test_expected_report_contract_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_entrypoint_authority_and_runtime_overclaims_fail(self) -> None:
        report = self.valid_report()
        report["authority"]["workspace"] = True
        self.assertIn("runtime authority closure changed", evidence.validate_report(report))
        report = self.valid_report()
        report["upstream_entrypoints_included"] = ["llama-server"]
        report["inference_started"] = True
        self.assertIn(
            "runtime state or authority was overclaimed",
            evidence.validate_report(report),
        )
        report = self.valid_report()
        report["process_boundary"]["native_runtime_profile_sha256"] = "0" * 64
        self.assertIn(
            "adapter process boundary changed",
            evidence.validate_report(report),
        )

    def test_package_surface_and_library_dependency_mutations_fail(self) -> None:
        report = self.valid_report()
        report["package"]["members"][0]["name"] = "llama-server"
        self.assertIn("runtime package closure is invalid", evidence.validate_report(report))
        report = self.valid_report()
        report["dynamic_libraries"][0]["needed"] = ["libcurl.so.4"]
        self.assertIn(
            "retained dynamic-library closure is invalid",
            evidence.validate_report(report),
        )

    def test_store_resources_platform_and_source_are_closed(self) -> None:
        for key, mutate, expected in (
            (
                "model_store",
                lambda report: report["model_store"].update({"directory_mode": 0o755}),
                "model-store boundary changed",
            ),
            (
                "resources",
                lambda report: report["resource_ceiling"].update({"swap_bytes": 1}),
                "resource ceiling changed",
            ),
            (
                "platform",
                lambda report: report["platform_status"].update(
                    {"ubuntu_26_04_x86_64": "verified"}
                ),
                "platform status changed",
            ),
            (
                "source",
                lambda report: report["sources"][0].update({"sha256": "invalid"}),
                "source closure is invalid",
            ),
        ):
            with self.subTest(key=key):
                report = copy.deepcopy(self.valid_report())
                mutate(report)
                self.assertIn(expected, evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()

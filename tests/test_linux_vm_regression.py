from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.linux_vm_regression import (
    LinuxVmRegressionError,
    load_catalog,
    validate_overlay_report,
    validate_catalog,
    verify_cached_bases,
)


class LinuxVmRegressionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.catalog = load_catalog()

    def test_checked_in_catalog_is_exact(self) -> None:
        self.assertEqual(validate_catalog(self.catalog), [])
        self.assertEqual(
            [target["target_id"] for target in self.catalog["targets"]],
            ["fedora-44-x86_64", "ubuntu-26.04-x86_64"],
        )

    def test_identity_authority_and_phase_mutations_fail_closed(self) -> None:
        mutations = []
        for path, value in (
            (("decision",), "ADR-9999"),
            (("targets", 0, "architecture"), "aarch64"),
            (("targets", 0, "source", "sha256"), "0" * 64),
            (("targets", 0, "cache_path"), "../outside.qcow2"),
            (("virtual_hardware", "acceleration"), "tcg"),
            (("virtual_hardware", "secure_boot"), True),
            (("toolchain", "rust"), "stable"),
            (("visual_studio_code", "version"), "latest"),
            (("standard_user", "uid"), 0),
            (("network_phases",), ["strict-offline", "dependency-acquisition"]),
            (("repository_credentials_allowed",), True),
            (("private_user_data_allowed",), True),
            (("release_claim",), True),
        ):
            mutated = copy.deepcopy(self.catalog)
            cursor = mutated
            for component in path[:-1]:
                cursor = cursor[component]
            cursor[path[-1]] = value
            mutations.append(mutated)
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.assertTrue(validate_catalog(mutation))

    def test_missing_duplicate_and_unsorted_package_snapshots_fail(self) -> None:
        missing = copy.deepcopy(self.catalog)
        missing["targets"].pop()
        duplicate = copy.deepcopy(self.catalog)
        duplicate["targets"][1]["target_id"] = "fedora-44-x86_64"
        unsorted = copy.deepcopy(self.catalog)
        unsorted["targets"][0]["package_snapshot"].reverse()
        for mutation in (missing, duplicate, unsorted):
            self.assertTrue(validate_catalog(mutation))

    def test_cached_base_verification_rejects_missing_and_changed_bytes(self) -> None:
        catalog = copy.deepcopy(self.catalog)
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            for target in catalog["targets"]:
                path = home / target["cache_path"]
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(target["target_id"].encode())
            exact = [target["source"]["sha256"] for target in catalog["targets"]]
            with patch("scripts.linux_vm_regression.sha256_file", side_effect=exact):
                self.assertEqual(len(verify_cached_bases(catalog, home)), 2)
            (home / catalog["targets"][0]["cache_path"]).write_bytes(b"changed")
            with patch(
                "scripts.linux_vm_regression.sha256_file", return_value="0" * 64
            ):
                with self.assertRaises(LinuxVmRegressionError):
                    verify_cached_bases(catalog, home)

    def test_overlay_report_requires_cleanup_and_zero_authority(self) -> None:
        report = {
            "schema_version": 1,
            "record_type": "linux-vm-regression-overlay-evidence",
            "task_ids": ["9.1.4.1", "9.1.4.2"],
            "source_revision": "0" * 40,
            "status": "pass-local-overlay-lifecycle",
            "qemu": {
                "launcher_class": "fedora-toolbox",
                "version": "QEMU 10.2.2",
                "sha256": "1" * 64,
                "kvm_accessible": True,
            },
            "targets": [
                {
                    "target_id": target["target_id"],
                    "base_bytes": 1,
                    "base_sha256": target["source"]["sha256"],
                    "overlay_format": "qcow2",
                    "backing_format": "qcow2",
                    "overlay_created": True,
                    "overlay_cleanup_verified": True,
                }
                for target in self.catalog["targets"]
            ],
            "repository_credentials_injected": False,
            "private_user_data_used": False,
            "guest_started": False,
            "network_used": False,
            "release_claim": False,
        }
        self.assertEqual(validate_overlay_report(report, self.catalog), [])
        for field in (
            "repository_credentials_injected",
            "private_user_data_used",
            "guest_started",
            "network_used",
            "release_claim",
        ):
            mutation = copy.deepcopy(report)
            mutation[field] = True
            self.assertTrue(validate_overlay_report(mutation, self.catalog))
        mutation = copy.deepcopy(report)
        mutation["targets"][0]["overlay_cleanup_verified"] = False
        self.assertTrue(validate_overlay_report(mutation, self.catalog))


if __name__ == "__main__":
    unittest.main()

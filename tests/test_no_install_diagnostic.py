from __future__ import annotations

import copy
import os
import stat
import tempfile
import unittest
from pathlib import Path

from scripts.no_install_diagnostic import (
    build_report,
    load_inventory,
    validate_inventory,
    validate_report,
)


def snapshot(path: Path) -> list[tuple[str, int, int, bytes]]:
    result = []
    for candidate in sorted(path.rglob("*")):
        if not candidate.is_file():
            continue
        metadata = candidate.stat()
        result.append(
            (
                candidate.relative_to(path).as_posix(),
                stat.S_IMODE(metadata.st_mode),
                metadata.st_mtime_ns,
                candidate.read_bytes(),
            )
        )
    return result


class NoInstallDiagnosticTests(unittest.TestCase):
    def setUp(self) -> None:
        self.inventory = load_inventory()

    def test_canonical_inventory_passes(self) -> None:
        self.assertEqual(validate_inventory(self.inventory), [])

    def test_missing_optional_component_is_reported(self) -> None:
        report = build_report(self.inventory, {"PATH": ""}, "linux", "x86_64")
        docker = next(item for item in report["components"] if item["id"] == "docker-compatibility")
        self.assertEqual(docker["status"], "missing")
        self.assertEqual(report["summary"]["missing_optional_compatibility_components"], 1)

    def test_present_executable_is_unverified_not_approved(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            executable = root / "docker"
            executable.write_text("fixture", encoding="utf-8")
            executable.chmod(0o700)
            report = build_report(
                self.inventory,
                {"PATH": str(root)},
                "linux",
                "x86_64",
            )
        docker = next(item for item in report["components"] if item["id"] == "docker-compatibility")
        self.assertEqual(docker["status"], "present-unverified")

    def test_diagnostic_does_not_mutate_probe_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            executable = root / "bwrap"
            executable.write_text("fixture", encoding="utf-8")
            executable.chmod(0o700)
            before = snapshot(root)
            build_report(self.inventory, {"PATH": str(root)}, "linux", "x86_64")
            after = snapshot(root)
        self.assertEqual(before, after)

    def test_report_does_not_emit_paths_or_environment_values(self) -> None:
        secret_path = "/private/example/do-not-emit"
        report = build_report(
            self.inventory,
            {"PATH": secret_path, "HOME": "/private/home"},
            "linux",
            "x86_64",
        )
        serialized = str(report)
        self.assertNotIn(secret_path, serialized)
        self.assertNotIn("/private/home", serialized)

    def test_other_platform_components_are_not_applicable(self) -> None:
        report = build_report(self.inventory, {"PATH": ""}, "linux", "x86_64")
        swift = next(item for item in report["components"] if item["id"] == "macos-swift-build")
        self.assertEqual(swift["status"], "not-applicable")

    def test_side_effect_contract_cannot_be_weakened(self) -> None:
        mutated = copy.deepcopy(self.inventory)
        mutated["side_effect_contract"]["writes_files"] = True
        self.assertTrue(validate_inventory(mutated))

    def test_macos_status_cannot_be_promoted(self) -> None:
        mutated = copy.deepcopy(self.inventory)
        mutated["platform_status"]["macos_verification"] = "verified"
        self.assertTrue(validate_inventory(mutated))

    def test_report_validator_rejects_side_effect_claim(self) -> None:
        report = build_report(self.inventory, {"PATH": ""}, "linux", "x86_64")
        report["summary"]["changes_made"] = 1
        self.assertTrue(validate_report(report, self.inventory))


if __name__ == "__main__":
    unittest.main()

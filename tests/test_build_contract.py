from __future__ import annotations

import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.build_contract import ROOT, load_contract, validate_contract


class BuildContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = load_contract()

    def copy_build_inputs(self, destination: Path) -> None:
        for relative in self.contract["required_files"]:
            source = ROOT / relative
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

    def test_canonical_build_contract_passes(self) -> None:
        self.assertEqual(validate_contract(self.contract), [])

    def test_missing_lockfile_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_build_inputs(root)
            (root / "Cargo.lock").unlink()
            failures = validate_contract(self.contract, root)
        self.assertTrue(any("Cargo.lock" in item for item in failures))

    def test_kernel_reverse_dependency_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_build_inputs(root)
            manifest = root / "kernel/engine/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8").replace(
                    "agentmage-kernel-contracts.workspace = true\n",
                    "agentmage-kernel-contracts.workspace = true\n"
                    "agentmage-capability-read-only.workspace = true\n",
                ),
                encoding="utf-8",
            )
            failures = validate_contract(self.contract, root)
        self.assertTrue(any("kernel/engine Cargo dependencies" in item for item in failures))

    def test_contract_serialization_dependencies_cannot_disappear(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_build_inputs(root)
            manifest = root / "kernel/contracts/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8").replace(
                    "serde_json.workspace = true\n", ""
                ),
                encoding="utf-8",
            )
            failures = validate_contract(self.contract, root)
        self.assertTrue(
            any("kernel/contracts Cargo dependencies" in item for item in failures)
        )

    def test_host_pdf_fixture_dependency_cannot_disappear(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_build_inputs(root)
            manifest = root / "shells/host/Cargo.toml"
            manifest.write_text(
                manifest.read_text(encoding="utf-8").replace(
                    "lopdf.workspace = true\n", ""
                ),
                encoding="utf-8",
            )
            failures = validate_contract(self.contract, root)
        self.assertTrue(
            any("shells/host Cargo development dependencies" in item for item in failures)
        )

    def test_floating_typescript_version_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_build_inputs(root)
            package_path = root / "shells/vscode/package.json"
            package = json.loads(package_path.read_text(encoding="utf-8"))
            package["devDependencies"]["typescript"] = "^5.9.3"
            package_path.write_text(json.dumps(package), encoding="utf-8")
            failures = validate_contract(self.contract, root)
        self.assertTrue(any("dependencies must be exact" in item for item in failures))

    def test_undeclared_swift_pin_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_build_inputs(root)
            lock_path = root / "platforms/macos/Package.resolved"
            lock_path.write_text(
                json.dumps({"pins": [{"identity": "unknown"}], "version": 2}),
                encoding="utf-8",
            )
            failures = validate_contract(self.contract, root)
        self.assertTrue(any("Swift lock" in item for item in failures))

    def test_macos_lane_cannot_be_marked_verified(self) -> None:
        mutated = copy.deepcopy(self.contract)
        mutated["platform_status"]["macos_verification"] = "verified"
        self.assertTrue(validate_contract(mutated))

    def test_ambient_end_user_toolchain_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.contract)
        mutated["end_user_ambient_toolchains"] = ["cargo"]
        self.assertIn(
            "end users must not require ambient development toolchains",
            validate_contract(mutated),
        )

    def test_command_drift_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.contract)
        mutated["commands"]["test"] = "cargo test"
        self.assertIn(
            "development commands do not match the build contract",
            validate_contract(mutated),
        )


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.build_contract import load_contract
from scripts.dependency_classes import ROOT, load_classes, validate_classes


class DependencyClassTests(unittest.TestCase):
    def setUp(self) -> None:
        self.record = load_classes()
        self.build_contract = load_contract()

    def copy_inputs(self, destination: Path) -> None:
        for relative in self.build_contract["required_files"]:
            source = ROOT / relative
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

    def test_canonical_dependency_classes_pass(self) -> None:
        self.assertEqual(validate_classes(self.record), [])

    def test_later_capability_dependency_cannot_be_enabled(self) -> None:
        for field in ("enabled", "included_in_default_build", "included_in_release"):
            with self.subTest(field=field):
                mutated = copy.deepcopy(self.record)
                mutated["optional_later_capabilities"][field] = True
                self.assertTrue(validate_classes(mutated))

    def test_platform_packaging_manifest_policy_cannot_drift(self) -> None:
        mutated = copy.deepcopy(self.record)
        mutated["platform_packaging"]["may_enter_product_manifests"] = False
        self.assertTrue(validate_classes(mutated))

    def test_vscode_development_package_cannot_become_production(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / "shells/vscode/package.json"
            package = json.loads(path.read_text(encoding="utf-8"))
            package["dependencies"] = {"typescript": package["devDependencies"]["typescript"]}
            path.write_text(json.dumps(package), encoding="utf-8")
            failures = validate_classes(self.record, root)
        self.assertTrue(any("undeclared dependencies" in item for item in failures))

    def test_cargo_development_dependency_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / "kernel/contracts/Cargo.toml"
            path.write_text(
                path.read_text(encoding="utf-8")
                + "\n[dev-dependencies]\nagentmage-kernel-engine.workspace = true\n",
                encoding="utf-8",
            )
            failures = validate_classes(self.record, root)
        self.assertTrue(any("Cargo development dependencies" in item for item in failures))

    def test_linux_fixture_dependency_cannot_drift(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / "platforms/linux/Cargo.toml"
            manifest = path.read_text(encoding="utf-8").replace(
                "serde_json.workspace = true",
                'serde_json.workspace = true\nrand = "0.9"',
            )
            path.write_text(manifest, encoding="utf-8")
            failures = validate_classes(self.record, root)
        self.assertTrue(any("do not match the test fixtures" in item for item in failures))

    def test_host_pdf_fixture_dependency_cannot_drift(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / "shells/host/Cargo.toml"
            path.write_text(
                path.read_text(encoding="utf-8").replace(
                    "lopdf.workspace = true\n", ""
                ),
                encoding="utf-8",
            )
            failures = validate_classes(self.record, root)
        self.assertTrue(any("do not match the test harness" in item for item in failures))

    def test_swift_external_package_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / "platforms/macos/Package.swift"
            path.write_text(
                path.read_text(encoding="utf-8") + "\n// .package(url: \"invalid\")\n",
                encoding="utf-8",
            )
            failures = validate_classes(self.record, root)
        self.assertTrue(any("Swift manifest" in item for item in failures))

    def test_root_development_package_must_remain_private(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / "package.json"
            package = json.loads(path.read_text(encoding="utf-8"))
            package["private"] = False
            path.write_text(json.dumps(package), encoding="utf-8")
            failures = validate_classes(self.record, root)
        self.assertTrue(any("must remain private" in item for item in failures))


if __name__ == "__main__":
    unittest.main()

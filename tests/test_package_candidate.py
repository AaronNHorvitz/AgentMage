from __future__ import annotations

import json
import shutil
import stat
import tempfile
import unittest
import zipfile
from pathlib import Path

from scripts.package_candidate import (
    MANIFEST_PATH,
    PackageCandidateError,
    build_deb,
    build_payload,
    build_vsix,
    valid_version,
    verify_payload,
)


class PackageCandidateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="agentmage-package-test-")
        self.root = Path(self.temporary.name)
        self.extension = self.root / "extension"
        (self.extension / "dist/src").mkdir(parents=True)
        package = {
            "name": "@agentmage/vscode-shell",
            "publisher": "agentmage-project",
            "main": "./dist/src/extension.js",
        }
        (self.extension / "package.json").write_text(json.dumps(package), encoding="utf-8")
        (self.extension / "dist/src/extension.js").write_text("export {};\n", encoding="utf-8")
        self.license = self.root / "LICENSE"
        self.license.write_text("Apache-2.0\n", encoding="utf-8")
        self.host = self.root / "agentmage-host"
        self.host.write_bytes(b"synthetic-host")
        self.host.chmod(0o755)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_vsix_is_deterministic_and_closed(self) -> None:
        first = self.root / "first.vsix"
        second = self.root / "second.vsix"
        build_vsix(self.extension, self.license, first)
        build_vsix(self.extension, self.license, second)
        self.assertEqual(first.read_bytes(), second.read_bytes())
        with zipfile.ZipFile(first) as archive:
            self.assertIn("extension.vsixmanifest", archive.namelist())
            self.assertIn("extension/package.json", archive.namelist())
            self.assertIn("extension/dist/src/extension.js", archive.namelist())
            self.assertNotIn("node_modules", "\n".join(archive.namelist()))
            self.assertNotIn("dist/test", "\n".join(archive.namelist()))
            self.assertFalse(any(name.endswith(".d.ts") for name in archive.namelist()))
            packaged = json.loads(archive.read("extension/package.json"))
            self.assertEqual(packaged["name"], "agentmage-vscode-shell")
            self.assertEqual(packaged["publisher"], "agentmage-project")

    def test_payload_verification_rejects_content_mode_and_manifest_mutation(self) -> None:
        vsix = self.root / "agentmage.vsix"
        build_vsix(self.extension, self.license, vsix)
        payload = self.root / "payload"
        build_payload(self.host, vsix, self.license, payload)
        verify_payload(payload)

        installed_host = payload / "usr/libexec/agentmage/agentmage-host"
        installed_host.write_bytes(b"mutated")
        with self.assertRaisesRegex(PackageCandidateError, "package.file_mismatch"):
            verify_payload(payload)

        shutil.copyfile(self.host, installed_host)
        installed_host.chmod(0o644)
        with self.assertRaisesRegex(PackageCandidateError, "package.file_mismatch"):
            verify_payload(payload)

        installed_host.chmod(0o755)
        manifest = json.loads((payload / MANIFEST_PATH).read_text(encoding="utf-8"))
        manifest["status"] = "signed-release"
        (payload / MANIFEST_PATH).write_text(json.dumps(manifest), encoding="utf-8")
        with self.assertRaisesRegex(PackageCandidateError, "package.manifest_identity"):
            verify_payload(payload)

    def test_deb_is_deterministic_and_contains_only_declared_payload(self) -> None:
        vsix = self.root / "agentmage.vsix"
        build_vsix(self.extension, self.license, vsix)
        payload = self.root / "payload"
        build_payload(self.host, vsix, self.license, payload)
        control = self.root / "control.in"
        control.write_text(
            "Package: agentmage\nVersion: @VERSION@\nArchitecture: amd64\n"
            "Maintainer: test <test@example.invalid>\nDescription: test\n",
            encoding="utf-8",
        )
        first = self.root / "first.deb"
        second = self.root / "second.deb"
        build_deb(payload, control, first)
        build_deb(payload, control, second)
        self.assertEqual(first.read_bytes(), second.read_bytes())
        self.assertTrue(first.read_bytes().startswith(b"!<arch>\n"))

    def test_payload_modes_are_fixed(self) -> None:
        vsix = self.root / "agentmage.vsix"
        build_vsix(self.extension, self.license, vsix)
        payload = self.root / "payload"
        build_payload(self.host, vsix, self.license, payload)
        self.assertEqual(
            stat.S_IMODE((payload / "usr/libexec/agentmage/agentmage-host").stat().st_mode),
            0o755,
        )
        self.assertEqual(stat.S_IMODE((payload / MANIFEST_PATH).stat().st_mode), 0o644)

    def test_versions_are_closed_and_bound_into_payload_identity(self) -> None:
        self.assertTrue(valid_version("0.0.1"))
        for invalid in ("v1", "1", "1.0", "1.01.0", "1.0.0-beta", "../1.0.0"):
            self.assertFalse(valid_version(invalid))
        vsix = self.root / "agentmage.vsix"
        build_vsix(self.extension, self.license, vsix, "0.0.1")
        payload = self.root / "payload"
        manifest = build_payload(self.host, vsix, self.license, payload, "0.0.1")
        self.assertEqual(manifest["package_id"], "agentmage-linux-x86_64-0.0.1-candidate")
        verify_payload(payload, version="0.0.1")
        with self.assertRaisesRegex(PackageCandidateError, "package.manifest_identity"):
            verify_payload(payload, version="0.0.0")


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import copy
import gzip
import io
import os
import tarfile
import tempfile
import unittest
from pathlib import Path

from scripts.native_llama_runtime import (
    EXPECTED_DESTINATIONS,
    NativeLlamaRuntimeError,
    _verify_runtime_package,
    build_package_bytes,
    load_profile,
    publish_no_replace,
    read_source_payload,
    sha256_bytes,
)


def source_archive_bytes(members: list[tarfile.TarInfo], bodies: list[bytes]) -> bytes:
    uncompressed = io.BytesIO()
    with tarfile.open(fileobj=uncompressed, mode="w", format=tarfile.USTAR_FORMAT) as archive:
        for member, body in zip(members, bodies, strict=True):
            archive.addfile(member, io.BytesIO(body) if member.isreg() else None)
    compressed = io.BytesIO()
    with gzip.GzipFile(fileobj=compressed, mode="wb", filename="", mtime=0) as stream:
        stream.write(uncompressed.getvalue())
    return compressed.getvalue()


class NativeLlamaRuntimeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="agentmage-native-runtime-test-")
        self.root = Path(self.temporary.name)
        self.profile, self.profile_sha256 = load_profile()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def synthetic_profile_and_payload(self) -> tuple[dict, dict[str, bytes | str]]:
        profile = copy.deepcopy(self.profile)
        payload: dict[str, bytes | str] = {}
        for record in profile["source_files"]:
            if record["type"] == "file":
                content = (record["destination"] + "\n").encode("utf-8")
                record["size_bytes"] = len(content)
                record["sha256"] = sha256_bytes(content)
                payload[record["source"]] = content
            else:
                payload[record["source"]] = record["target"]
        return profile, payload

    def test_checked_profile_is_closed_to_libraries_and_license(self) -> None:
        self.assertEqual(
            [record["destination"] for record in self.profile["source_files"]],
            list(EXPECTED_DESTINATIONS),
        )
        destinations = "\n".join(EXPECTED_DESTINATIONS).lower()
        for prohibited in (
            "llama-server",
            "llama-cli",
            "ggml-rpc",
            "vulkan",
            "libmtmd",
            "libllama-common",
            "-impl",
        ):
            self.assertNotIn(prohibited, destinations)
        self.assertEqual(self.profile["authority"], dict.fromkeys(
            ("credential", "grant", "network", "tool", "workspace"), False
        ))
        self.assertEqual(self.profile["decision"]["enabled_models"], 0)
        self.assertFalse(self.profile["decision"]["inference_implemented"])

    def test_package_is_deterministic_read_only_and_has_no_upstream_entrypoint(self) -> None:
        profile, payload = self.synthetic_profile_and_payload()
        first = build_package_bytes(profile, "a" * 64, payload)
        second = build_package_bytes(profile, "a" * 64, payload)
        self.assertEqual(first, second)
        package = self.root / "runtime.tar.gz"
        package.write_bytes(first)
        report = _verify_runtime_package(package, profile, "a" * 64)
        self.assertEqual(report["file_count"], 28)
        with tarfile.open(package, mode="r:gz") as archive:
            members = archive.getmembers()
        self.assertTrue(all(member.uid == 0 and member.gid == 0 for member in members))
        self.assertTrue(all(member.mtime == 0 for member in members))
        self.assertFalse(any(member.mode & 0o222 for member in members if member.isreg()))
        names = "\n".join(member.name for member in members).lower()
        for prohibited in ("server", "rpc", "cli", "vulkan", "download"):
            self.assertNotIn(prohibited, names)

    def test_source_archive_hash_size_and_unsafe_members_fail_closed(self) -> None:
        archive = self.root / "source.tar.gz"
        archive.write_bytes(b"not the pinned archive")
        with self.assertRaisesRegex(NativeLlamaRuntimeError, "archive-identity"):
            read_source_payload(archive, self.profile)

        traversal = tarfile.TarInfo("../escape")
        traversal.size = 1
        content = source_archive_bytes([traversal], [b"x"])
        archive.write_bytes(content)
        changed = copy.deepcopy(self.profile)
        changed["source_archive"]["size_bytes"] = len(content)
        changed["source_archive"]["sha256"] = sha256_bytes(content)
        with self.assertRaisesRegex(NativeLlamaRuntimeError, "path-invalid"):
            read_source_payload(archive, changed)

        hardlink = tarfile.TarInfo("llama-b10333/unexpected")
        hardlink.type = tarfile.LNKTYPE
        hardlink.linkname = "llama-b10333/LICENSE"
        content = source_archive_bytes([hardlink], [b""])
        archive.write_bytes(content)
        changed["source_archive"]["size_bytes"] = len(content)
        changed["source_archive"]["sha256"] = sha256_bytes(content)
        with self.assertRaisesRegex(NativeLlamaRuntimeError, "member-type"):
            read_source_payload(archive, changed)

    def test_publication_never_overwrites_existing_output(self) -> None:
        output = self.root / "runtime.tar.gz"
        publish_no_replace(output, b"first")
        self.assertEqual(output.read_bytes(), b"first")
        self.assertEqual(os.stat(output).st_mode & 0o777, 0o444)
        with self.assertRaisesRegex(NativeLlamaRuntimeError, "output-exists"):
            publish_no_replace(output, b"second")
        self.assertEqual(output.read_bytes(), b"first")

    def test_manifest_or_library_drift_is_rejected(self) -> None:
        profile, payload = self.synthetic_profile_and_payload()
        package = self.root / "runtime.tar.gz"
        package.write_bytes(build_package_bytes(profile, "a" * 64, payload))
        _verify_runtime_package(package, profile, "a" * 64)
        changed = copy.deepcopy(profile)
        changed["source_files"][-1]["sha256"] = "0" * 64
        with self.assertRaisesRegex(NativeLlamaRuntimeError, "package-manifest"):
            _verify_runtime_package(package, changed, "a" * 64)


if __name__ == "__main__":
    unittest.main()

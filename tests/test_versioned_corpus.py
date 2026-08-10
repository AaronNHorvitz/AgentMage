from __future__ import annotations

import copy
import hashlib
import io
import json
import tempfile
import unittest
import zipfile
import zlib
from pathlib import Path, PurePosixPath

from scripts.versioned_corpus import (
    EXPECTED_SOURCES,
    PROFILE_PATH,
    archive_bytes,
    build_corpus,
    build_report,
    canonical_json,
    check_checked_corpus,
    check_report,
    generate,
    materialize_entries,
    read_json,
    sha256_bytes,
    validate_archive,
    validate_manifest,
    validate_profile,
    validate_report,
)


class VersionedCorpusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)
        self.entries = materialize_entries(self.profile)
        self.archive, self.manifest = build_corpus(self.profile)

    def test_checked_in_profile_archive_manifest_and_report_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(check_checked_corpus(), [])
        self.assertEqual(check_report(), [])

    def test_all_source_families_are_versioned_and_nonempty(self) -> None:
        expected_families = {source[0] for source in EXPECTED_SOURCES}
        self.assertEqual({entry.family for entry in self.entries.values()}, expected_families)
        for family in expected_families:
            self.assertGreater(self.manifest["family_counts"][family], 0)
        self.assertEqual(self.manifest["archive"]["entry_count"], len(self.entries))
        self.assertEqual(self.manifest["archive"]["entry_count"], 77)

    def test_archive_is_stored_safe_non_executable_and_not_a_symlink_container(self) -> None:
        self.assertEqual(validate_archive(self.archive, self.entries), [])
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            for info in archive.infolist():
                path = PurePosixPath(info.filename)
                mode = info.external_attr >> 16
                self.assertFalse(path.is_absolute())
                self.assertNotIn("..", path.parts)
                self.assertEqual(mode & 0o111, 0)
                self.assertNotEqual(mode & 0o170000, 0o120000)
                self.assertEqual(info.compress_type, zipfile.ZIP_STORED)
                self.assertEqual(info.flag_bits & 0x1, 0)

    def test_path_symlinks_are_inert_declarations_only(self) -> None:
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            declarations = json.loads(archive.read("paths/symlink-declarations.json"))
            self.assertEqual(len(declarations["symlinks"]), 2)
            self.assertTrue(
                all(item["materialize_only_in_temporary_sandbox"] for item in declarations["symlinks"])
            )
            archive_names = set(archive.namelist())
            for item in declarations["symlinks"]:
                self.assertNotIn(f"paths/{item['path']}", archive_names)
                self.assertFalse(PurePosixPath(item["target"]).is_absolute())

    def test_golden_manifests_cover_each_evidence_state_and_bind_receipts(self) -> None:
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            goldens = [
                json.loads(archive.read(name))
                for name in archive.namelist()
                if name.startswith("golden/")
            ]
        self.assertEqual(len(goldens), 4)
        self.assertEqual(
            {item["expected_output"]["evidence"]["state"] for item in goldens},
            {"Observed", "Derived", "Inferred", "Unknown/Blocked"},
        )
        self.assertTrue(all(len(item["expected_receipt"]["receipt_sha256"]) == 64 for item in goldens))

    def test_binary_malformed_and_git_object_bytes_survive_archival(self) -> None:
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            malformed = archive.read("base/malformed/invalid-utf8.txt")
            self.assertIn(b"\xff\xfe", malformed)
            object_name = next(
                name for name in archive.namelist() if name.startswith("base/git/repository/.git/objects/")
            )
            object_id = object_name.rsplit("/objects/", 1)[1].replace("/", "")
            payload = zlib.decompress(archive.read(object_name))
            self.assertEqual(hashlib.sha1(payload, usedforsecurity=False).hexdigest(), object_id)

    def test_document_containers_remain_valid_nested_archives(self) -> None:
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            for name in (
                "documents/word/status-report.docx",
                "documents/spreadsheets/project-ledger.xlsx",
                "documents/presentations/project-brief.pptx",
            ):
                with self.subTest(name=name):
                    with zipfile.ZipFile(io.BytesIO(archive.read(name))) as nested:
                        self.assertIn("[Content_Types].xml", nested.namelist())
                        self.assertIsNone(nested.testzip())
            self.assertTrue(archive.read("documents/pdf/status-report.pdf").startswith(b"%PDF-1.4"))
            self.assertTrue(archive.read("documents/images/status-grid.png").startswith(b"\x89PNG"))

    def test_manifest_binds_archive_entries_profiles_generators_and_assembler(self) -> None:
        self.assertEqual(self.manifest["archive"]["sha256"], sha256_bytes(self.archive))
        for record in self.manifest["entries"]:
            self.assertEqual(record["sha256"], sha256_bytes(self.entries[record["archive_path"]].content))
        for source in self.manifest["source_provenance"]:
            self.assertEqual(source["profile_sha256"], sha256_bytes(Path(source["profile_path"]).read_bytes()))
            self.assertEqual(source["generator_sha256"], sha256_bytes(Path(source["generator"]).read_bytes()))
        self.assertEqual(
            self.manifest["assembler"]["sha256"],
            sha256_bytes(Path(self.manifest["assembler"]["path"]).read_bytes()),
        )

    def test_manifest_self_hash_and_archive_hash_detect_corruption(self) -> None:
        unhashed = copy.deepcopy(self.manifest)
        recorded_hash = unhashed.pop("manifest_sha256")
        self.assertEqual(sha256_bytes(canonical_json(unhashed)), recorded_hash)
        corrupted_manifest = copy.deepcopy(self.manifest)
        corrupted_manifest["archive"]["entry_count"] -= 1
        self.assertTrue(
            any(
                "self-hash" in failure or "entry count" in failure
                for failure in validate_manifest(
                    corrupted_manifest, self.profile, self.entries, self.archive
                )
            )
        )

    def test_archive_corruption_is_detected_before_use(self) -> None:
        corrupted = bytearray(self.archive)
        corrupted[len(corrupted) // 3] ^= 0xFF
        failures = validate_archive(bytes(corrupted), self.entries)
        self.assertTrue(failures)

    def test_generation_is_byte_identical_and_refuses_existing_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first"
            second = Path(temporary) / "second"
            self.assertEqual(generate(self.profile, first), generate(self.profile, second))
            self.assertEqual(
                {path.name: path.read_bytes() for path in first.iterdir()},
                {path.name: path.read_bytes() for path in second.iterdir()},
            )
            self.assertTrue(all(path.stat().st_mode & 0o111 == 0 for path in first.iterdir()))
            with self.assertRaises(FileExistsError):
                generate(self.profile, first)

    def test_archive_excludes_common_real_credential_and_private_path_shapes(self) -> None:
        with zipfile.ZipFile(io.BytesIO(self.archive)) as archive:
            combined = b"\n".join(archive.read(name) for name in archive.namelist())
        for prohibited in (
            b"AKIA",
            b"BEGIN PRIVATE KEY",
            b"ghp_",
            b"/home/",
            b"/Users/",
        ):
            self.assertNotIn(prohibited, combined)
        self.assertIn(b"AM_SYNTHETIC_CANARY_", combined)

    def test_profile_cannot_enable_compression_symlinks_or_product_claims(self) -> None:
        compressed = copy.deepcopy(self.profile)
        compressed["archive_safety_contract"]["compressed_entries"] = True
        symlinks = copy.deepcopy(self.profile)
        symlinks["path_fixture_materialization"] = "active-symlinks"
        claimed = copy.deepcopy(self.profile)
        claimed["content_contract"]["product_parser_support_claim"] = "supported"
        self.assertTrue(validate_profile(compressed))
        self.assertTrue(validate_profile(symlinks))
        self.assertTrue(validate_profile(claimed))

    def test_report_is_current_bounded_and_retains_blocked_macos_status(self) -> None:
        report = build_report()
        self.assertEqual(report["corpus"]["entry_count"], 77)
        self.assertEqual(report["golden_manifest_count"], 4)
        self.assertEqual(report["path_fixture_materialization"], "inert-declarations-only")
        self.assertEqual(report["product_support_claim"], "none")
        self.assertEqual(report["macos_execution_status"], "blocked-macos")
        mutated = copy.deepcopy(report)
        mutated["macos_execution_status"] = "pass"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()

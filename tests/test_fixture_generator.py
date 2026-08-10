from __future__ import annotations

import copy
import hashlib
import tempfile
import unittest
import zlib
from pathlib import Path

from scripts.fixture_generator import (
    EXPECTED_CATEGORIES,
    PROFILE_PATH,
    check_report,
    corpus_identity,
    generate,
    materialize_specs,
    read_json,
    validate_profile,
    validate_report,
)


class FixtureGeneratorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)

    def test_checked_in_profile_and_report_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(check_report(), [])

    def test_two_generations_are_byte_identical(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first"
            second = Path(temporary) / "second"
            first_result = generate(self.profile, first)
            second_result = generate(self.profile, second)
            self.assertEqual(first_result, second_result)
            first_files = {
                path.relative_to(first): path.read_bytes()
                for path in first.rglob("*")
                if path.is_file()
            }
            second_files = {
                path.relative_to(second): path.read_bytes()
                for path in second.rglob("*")
                if path.is_file()
            }
            self.assertEqual(first_files, second_files)

    def test_every_required_category_is_nonempty(self) -> None:
        specs = materialize_specs(self.profile)
        categories = {item.category for item in specs.values()}
        self.assertEqual(categories, set(EXPECTED_CATEGORIES))
        for category in EXPECTED_CATEGORIES:
            self.assertTrue(any(item.category == category for item in specs.values()))

    def test_supported_and_unsupported_language_families_are_separate(self) -> None:
        specs = materialize_specs(self.profile)
        supported = [path for path in specs if path.startswith("parser-supported/")]
        unsupported = [path for path in specs if path.startswith("parser-unsupported/")]
        self.assertEqual(len(supported), 7)
        self.assertEqual(len(unsupported), 3)
        self.assertTrue(
            all(specs[path].category == "supported-parser-languages" for path in supported)
        )
        self.assertTrue(
            all(specs[path].category == "unsupported-languages" for path in unsupported)
        )

    def test_git_fixture_contains_valid_content_addressed_objects(self) -> None:
        specs = materialize_specs(self.profile)
        object_prefix = "git/repository/.git/objects/"
        object_paths = [path for path in specs if path.startswith(object_prefix)]
        self.assertTrue(object_paths)
        for path in object_paths:
            object_id = path[len(object_prefix) :].replace("/", "")
            payload = zlib.decompress(specs[path].content)
            self.assertEqual(
                hashlib.sha1(payload, usedforsecurity=False).hexdigest(), object_id
            )
        ref = specs["git/repository/.git/refs/heads/main"].content.decode().strip()
        self.assertIn(f"{object_prefix}{ref[:2]}/{ref[2:]}", specs)

    def test_malformed_cases_retain_their_invalid_bytes(self) -> None:
        specs = materialize_specs(self.profile)
        self.assertIn(b"\xff\xfe", specs["malformed/invalid-utf8.txt"].content)
        self.assertIn(b"\x00", specs["malformed/nul.txt"].content)
        with self.assertRaises(UnicodeDecodeError):
            specs["malformed/invalid-utf8.txt"].content.decode("utf-8")

    def test_generated_content_has_no_private_paths_remote_references_or_credentials(self) -> None:
        specs = materialize_specs(self.profile)
        combined = b"\n".join(item.content for item in specs.values())
        for prohibited in (
            b"/home/",
            b"/Users/",
            b"http://",
            b"https://",
            b"AKIA",
            b"BEGIN PRIVATE KEY",
        ):
            self.assertNotIn(prohibited, combined)

    def test_generation_refuses_existing_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(FileExistsError):
                generate(self.profile, Path(temporary))

    def test_generated_files_are_not_executable_or_symlinks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "corpus"
            generate(self.profile, output)
            for path in output.rglob("*"):
                if path.is_file():
                    self.assertFalse(path.is_symlink())
                    self.assertEqual(path.stat().st_mode & 0o111, 0)

    def test_seed_change_changes_corpus_identity(self) -> None:
        original = materialize_specs(self.profile)
        mutated_profile = copy.deepcopy(self.profile)
        mutated_profile["seed"] = "different-synthetic-seed"
        mutated = materialize_specs(mutated_profile)
        self.assertNotEqual(corpus_identity(original), corpus_identity(mutated))

    def test_product_parser_support_claim_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.profile)
        mutated["product_parser_support_claim"] = "supported"
        self.assertTrue(validate_profile(mutated))

    def test_report_cannot_change_versioned_corpus_disposition(self) -> None:
        report = read_json(
            Path("artifacts/sprints/sprint-2/story-2.1/fixture-generator-report.json")
        )
        report["versioned_corpus_status"] = "complete"
        self.assertTrue(validate_report(report))


if __name__ == "__main__":
    unittest.main()

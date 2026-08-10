from __future__ import annotations

import io
import json
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path

from scripts.requirement_registry import (
    DEFAULT_OUTPUT,
    DEFAULT_SOURCE,
    RegistryError,
    build_registry,
    check_registry,
    extract_identifiers,
    render_registry,
    write_registry,
)


class ExtractIdentifiersTests(unittest.TestCase):
    def test_extracts_supported_table_definitions_in_sorted_order(self) -> None:
        markdown = "\n".join(
            (
                "| `CR-P0-ONE` | Competitive |",
                "| `AM-KRN-001` | Requirement |",
                "| `AT-ARCH-001` | Test |",
            )
        )

        self.assertEqual(
            extract_identifiers(markdown),
            [
                ("AM-KRN-001", "product_requirement"),
                ("AT-ARCH-001", "acceptance_test"),
                ("CR-P0-ONE", "competitive_requirement"),
            ],
        )

    def test_rejects_duplicate_definitions(self) -> None:
        markdown = "\n".join(
            (
                "| `AM-KRN-001` | First |",
                "| `AM-KRN-001` | Duplicate |",
                "| `AT-ARCH-001` | Test |",
                "| `CR-P0-ONE` | Competitive |",
            )
        )

        with self.assertRaisesRegex(RegistryError, "AM-KRN-001"):
            extract_identifiers(markdown)

    def test_rejects_a_missing_canonical_category(self) -> None:
        markdown = "\n".join(
            (
                "| `AM-KRN-001` | Requirement |",
                "| `AT-ARCH-001` | Test |",
            )
        )

        with self.assertRaisesRegex(RegistryError, "CR"):
            extract_identifiers(markdown)


class RegistryArtifactTests(unittest.TestCase):
    def test_committed_registry_covers_every_canonical_definition(self) -> None:
        registry = build_registry(DEFAULT_SOURCE)
        committed = json.loads(DEFAULT_OUTPUT.read_text(encoding="utf-8"))
        expected_ids = [item["id"] for item in registry["requirements"]]
        committed_ids = [item["id"] for item in committed["requirements"]]

        self.assertEqual(committed, registry)
        self.assertEqual(committed_ids, expected_ids)
        self.assertEqual(len(committed_ids), len(set(committed_ids)))
        self.assertEqual(committed["counts"]["total"], len(committed_ids))

    def test_output_is_byte_deterministic(self) -> None:
        expected = render_registry(build_registry(DEFAULT_SOURCE))
        source_before = DEFAULT_SOURCE.read_bytes()

        with tempfile.TemporaryDirectory() as temp_dir:
            first = Path(temp_dir) / "first.json"
            second = Path(temp_dir) / "second.json"
            write_registry(DEFAULT_SOURCE, first)
            write_registry(DEFAULT_SOURCE, second)

            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(first.read_text(encoding="utf-8"), expected)
            self.assertEqual(DEFAULT_SOURCE.read_bytes(), source_before)

    def test_check_mode_detects_missing_and_stale_registry(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "registry.json"
            with redirect_stderr(io.StringIO()):
                self.assertFalse(check_registry(DEFAULT_SOURCE, output))
            self.assertFalse(output.exists())

            output.write_text("{}\n", encoding="utf-8")
            stale_bytes = output.read_bytes()
            with redirect_stderr(io.StringIO()):
                self.assertFalse(check_registry(DEFAULT_SOURCE, output))
            self.assertEqual(output.read_bytes(), stale_bytes)

            write_registry(DEFAULT_SOURCE, output)
            self.assertTrue(check_registry(DEFAULT_SOURCE, output))


if __name__ == "__main__":
    unittest.main()

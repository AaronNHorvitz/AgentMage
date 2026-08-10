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
    definition_from_cells,
    extract_identifiers,
    parse_definitions,
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


class DefinitionParsingTests(unittest.TestCase):
    def test_normalizes_each_canonical_definition_type(self) -> None:
        markdown = "\n".join(
            (
                "## Executable v0.1 Backlog",
                "| `AM-KRN-001` | Kernel `contract` | None | Build | v0.1 | `AT-ARCH-001` |",
                "## 31B. v0.1 Quantitative Acceptance Matrix",
                "| `AT-ARCH-001` | Architecture check | Zero reverse edges |",
                "## 35. Competitive Review Integration Register",
                "| `CR-P0-REP` | Repository map | v0.1 | 7A | Added |",
            )
        )

        definitions = parse_definitions(markdown, "inventory.md")

        self.assertEqual([item["id"] for item in definitions], ["AM-KRN-001", "AT-ARCH-001", "CR-P0-REP"])
        product = definitions[0]
        self.assertEqual(product["title"], "Kernel `contract`")
        self.assertEqual(product["source"]["heading"], "Executable v0.1 Backlog")
        self.assertEqual(product["source"]["line"], 2)
        self.assertEqual(product["release"], "v0.1")
        self.assertEqual(product["dependencies"], [])
        self.assertEqual(product["disposition"], "build")
        self.assertEqual(product["acceptance_tests"], ["AT-ARCH-001"])
        self.assertEqual(product["status"], "planned")
        self.assertEqual(definitions[1]["disposition"], "required")
        self.assertEqual(definitions[2]["disposition"], "integrated")

    def test_rejects_wrong_heading_and_column_count(self) -> None:
        with self.assertRaisesRegex(RegistryError, "expected 'Executable v0.1 Backlog'"):
            definition_from_cells(
                ["`AM-KRN-001`", "Kernel", "None", "Build", "v0.1", "`AT-ARCH-001`"],
                heading="Wrong heading",
                line_number=4,
                source_document="inventory.md",
                source_line="fixture",
            )

        with self.assertRaisesRegex(RegistryError, "has 2 columns; expected 3"):
            definition_from_cells(
                ["`AT-ARCH-001`", "Architecture"],
                heading="31B. v0.1 Quantitative Acceptance Matrix",
                line_number=8,
                source_document="inventory.md",
                source_line="fixture",
            )


class RegistryArtifactTests(unittest.TestCase):
    def test_committed_registry_covers_every_canonical_definition(self) -> None:
        registry = build_registry(DEFAULT_SOURCE)
        committed = json.loads(DEFAULT_OUTPUT.read_text(encoding="utf-8"))
        expected_ids = [item["id"] for item in registry["requirements"]]
        committed_ids = [item["id"] for item in committed["requirements"]]

        self.assertEqual(committed, registry)
        self.assertEqual(committed["schema_version"], 2)
        self.assertEqual(committed["source"]["document"], "Agent-Scaffolding-Inventory.md")
        self.assertRegex(committed["source"]["sha256"], r"^[a-f0-9]{64}$")
        self.assertEqual(committed_ids, expected_ids)
        self.assertEqual(len(committed_ids), len(set(committed_ids)))
        self.assertEqual(committed["counts"]["total"], len(committed_ids))
        for requirement in committed["requirements"]:
            self.assertEqual(
                set(requirement),
                {
                    "acceptance_tests",
                    "dependencies",
                    "disposition",
                    "id",
                    "kind",
                    "release",
                    "source",
                    "status",
                    "title",
                },
            )
            self.assertEqual(
                set(requirement["source"]),
                {"definition_sha256", "document", "heading", "line"},
            )

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

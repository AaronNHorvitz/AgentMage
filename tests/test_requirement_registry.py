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
        with self.assertRaisesRegex(RegistryError, "expected one of"):
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

    def test_normalizes_productivity_first_ga_definitions(self) -> None:
        product = definition_from_cells(
            [
                "`AM-ATC-001`",
                "Autonomy Center",
                "`AM-ADP-001`",
                "Build",
                "v1.0",
                "`AT-AUT-001`",
            ],
            heading="37. First-GA Productivity, Finance, and Cloud Observer Backlog",
            line_number=20,
            source_document="inventory.md",
            source_line="fixture",
        )
        acceptance = definition_from_cells(
            ["`AT-AUT-001`", "Autonomy matrix", "Zero authority broadening"],
            heading="37A. Productivity, Finance, and Cloud Quantitative Acceptance Matrix",
            line_number=21,
            source_document="inventory.md",
            source_line="fixture",
        )

        self.assertEqual(product["release"], "v1.0")
        self.assertEqual(product["dependencies"], ["AM-ADP-001"])
        self.assertEqual(product["acceptance_tests"], ["AT-AUT-001"])
        self.assertEqual(acceptance["release"], "v1.0")
        self.assertEqual(acceptance["disposition"], "required")

    def test_normalizes_trusted_operations_and_post_ga_definitions(self) -> None:
        trusted = definition_from_cells(
            [
                "`AM-TRU-001`",
                "Trusted operations",
                "`AM-KRN-001`",
                "Build",
                "v1.0",
                "`AT-TRU-001`",
            ],
            heading=(
                "38. First-GA Trusted Operations, Research, Continuity, "
                "and Model Management Backlog"
            ),
            line_number=30,
            source_document="inventory.md",
            source_line="fixture",
        )
        trusted_acceptance = definition_from_cells(
            ["`AT-TRU-001`", "Trusted operations matrix", "Zero authority union"],
            heading="38A. First-GA Trusted Operations Quantitative Acceptance Matrix",
            line_number=31,
            source_document="inventory.md",
            source_line="fixture",
        )
        lab = definition_from_cells(
            [
                "`AM-EML-001`",
                "Experimental Model Lab",
                "None",
                "Build",
                "post-GA",
                "`AT-EML-001`",
            ],
            heading="39. Post-GA Experimental Model Lab Backlog",
            line_number=32,
            source_document="inventory.md",
            source_line="fixture",
        )
        lab_acceptance = definition_from_cells(
            ["`AT-EML-001`", "Lab isolation", "Zero connected authority"],
            heading="39A. Post-GA Experimental Model Acceptance Matrix",
            line_number=33,
            source_document="inventory.md",
            source_line="fixture",
        )

        self.assertEqual(trusted["release"], "v1.0")
        self.assertEqual(trusted_acceptance["release"], "v1.0")
        self.assertEqual(lab["release"], "post-GA")
        self.assertEqual(lab["dependencies"], [])
        self.assertEqual(lab_acceptance["release"], "post-GA")

    def test_normalizes_whole_codebase_audit_first_ga_definitions(self) -> None:
        audit = definition_from_cells(
            [
                "`AM-CBA-001`",
                "Whole-codebase audit",
                "`AM-TRU-001`",
                "Build",
                "v1.0",
                "`AT-CBA-001`",
            ],
            heading="40. First-GA Whole-Codebase Audit Backlog",
            line_number=40,
            source_document="inventory.md",
            source_line="fixture",
        )
        audit_acceptance = definition_from_cells(
            [
                "`AT-CBA-001`",
                "Complete audit lifecycle",
                "Zero silent omissions or undeclared authority",
            ],
            heading="40A. First-GA Whole-Codebase Audit Quantitative Acceptance Matrix",
            line_number=41,
            source_document="inventory.md",
            source_line="fixture",
        )

        self.assertEqual(audit["release"], "v1.0")
        self.assertEqual(audit["dependencies"], ["AM-TRU-001"])
        self.assertEqual(audit["acceptance_tests"], ["AT-CBA-001"])
        self.assertEqual(audit_acceptance["release"], "v1.0")
        self.assertEqual(audit_acceptance["disposition"], "required")


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
            source_line = DEFAULT_SOURCE.read_text(encoding="utf-8").splitlines()[
                requirement["source"]["line"] - 1
            ]
            self.assertEqual(
                source_line.split("|", maxsplit=2)[1].strip(),
                f"`{requirement['id']}`",
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

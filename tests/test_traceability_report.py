from __future__ import annotations

import io
import json
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path

from scripts.traceability_report import (
    DEFAULT_NORMATIVE_MAP,
    DEFAULT_OUTPUT,
    DEFAULT_POLICY_REGISTER,
    DEFAULT_REGISTRY,
    DEFAULT_TASKS,
    TraceabilityError,
    build_traceability_report,
    check_traceability_report,
    render_traceability_report,
    write_traceability_report,
)


class TraceabilityReportTests(unittest.TestCase):
    def test_committed_report_is_complete_and_current(self) -> None:
        report = build_traceability_report()
        committed = json.loads(DEFAULT_OUTPUT.read_text(encoding="utf-8"))

        self.assertEqual(committed, report)
        self.assertEqual(report["schema_version"], 1)
        self.assertEqual(report["counts"]["total"], 78)
        self.assertEqual(
            report["counts"]["by_kind"],
            {
                "acceptance_test": 33,
                "competitive_requirement": 18,
                "product_requirement": 27,
            },
        )

    def test_every_requirement_resolves_to_plan_test_release_status_and_evidence(self) -> None:
        report = build_traceability_report()
        for requirement in report["requirements"]:
            with self.subTest(requirement=requirement["id"]):
                self.assertTrue(requirement["source"]["heading"])
                self.assertGreater(requirement["source"]["line"], 0)
                self.assertTrue(requirement["release"])
                self.assertTrue(requirement["status"])
                self.assertTrue(requirement["implementation"]["planning_items"])
                self.assertIn(
                    requirement["implementation"]["external_issue_status"],
                    {"not_created", "created"},
                )
                self.assertIn(
                    requirement["evidence"]["status"],
                    {"not_yet_produced", "current", "stale", "blocked"},
                )
                self.assertTrue(requirement["evidence"]["expected_roots"])
                self.assertIn(
                    requirement["exclusion"]["state"],
                    {"not_excluded", "outside_v0.1_release", "explicit_policy_expectation"},
                )
                if requirement["kind"] == "product_requirement":
                    self.assertTrue(requirement["acceptance_tests"])
                if requirement["kind"] == "acceptance_test":
                    self.assertTrue(requirement["verifies_requirement_ids"])

    def test_normative_statements_reverse_map_to_product_requirements(self) -> None:
        report = build_traceability_report()
        records = {record["id"]: record for record in report["requirements"]}
        normative_count = sum(len(record["normative_statements"]) for record in records.values())

        self.assertGreaterEqual(normative_count, 26)
        self.assertTrue(records["AM-KRN-001"]["normative_statements"])
        for record in records.values():
            if record["normative_statements"]:
                self.assertEqual(record["kind"], "product_requirement")

    def test_unlisted_acceptance_test_inherits_owning_requirement_plan(self) -> None:
        report = build_traceability_report()
        records = {record["id"]: record for record in report["requirements"]}
        architecture_test = records["AT-ARCH-001"]

        self.assertEqual(
            architecture_test["implementation"]["derivation"],
            "assigned_product_requirement",
        )
        self.assertEqual(
            [item["sprint"] for item in architecture_test["implementation"]["planning_items"]],
            [1, 4, 12],
        )

    def test_missing_plan_mapping_blocks_generation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            tasks = Path(temp_dir) / "TASKS.md"
            text = DEFAULT_TASKS.read_text(encoding="utf-8").replace(
                "`CR-P2-MAG`",
                "multi-agent requirement",
            )
            tasks.write_text(text, encoding="utf-8")

            with self.assertRaisesRegex(TraceabilityError, "CR-P2-MAG"):
                build_traceability_report(tasks_path=tasks)

    def test_duplicate_registry_identity_blocks_generation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            registry = Path(temp_dir) / "registry.json"
            data = json.loads(DEFAULT_REGISTRY.read_text(encoding="utf-8"))
            data["requirements"][1]["id"] = data["requirements"][0]["id"]
            registry.write_text(json.dumps(data), encoding="utf-8")

            with self.assertRaisesRegex(TraceabilityError, "duplicate requirement IDs"):
                build_traceability_report(registry_path=registry)

    def test_report_generation_is_byte_deterministic_and_side_effect_free(self) -> None:
        inputs = (
            DEFAULT_REGISTRY,
            DEFAULT_NORMATIVE_MAP,
            DEFAULT_POLICY_REGISTER,
            DEFAULT_TASKS,
        )
        before = tuple(path.read_bytes() for path in inputs)
        first = render_traceability_report(build_traceability_report())
        second = render_traceability_report(build_traceability_report())

        self.assertEqual(first, second)
        self.assertEqual(before, tuple(path.read_bytes() for path in inputs))

    def test_check_mode_detects_missing_and_stale_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "traceability-report.json"
            with redirect_stderr(io.StringIO()):
                self.assertFalse(check_traceability_report(output))
            self.assertFalse(output.exists())

            output.write_text("{}\n", encoding="utf-8")
            stale = output.read_bytes()
            with redirect_stderr(io.StringIO()):
                self.assertFalse(check_traceability_report(output))
            self.assertEqual(output.read_bytes(), stale)

            write_traceability_report(output)
            self.assertTrue(check_traceability_report(output))


if __name__ == "__main__":
    unittest.main()

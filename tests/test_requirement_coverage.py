from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.requirement_coverage import (
    DEFAULT_NORMATIVE_MAP,
    DEFAULT_REGISTRY,
    ROOT,
    build_coverage_report,
    load_json,
)


class RequirementCoverageTests(unittest.TestCase):
    def setUp(self) -> None:
        self.registry = load_json(DEFAULT_REGISTRY)
        self.normative_map = load_json(DEFAULT_NORMATIVE_MAP)

    def categories(self, report: dict[str, object]) -> list[str]:
        return [item["category"] for item in report["diagnostics"]]

    def test_committed_registry_and_normative_map_have_complete_coverage(self) -> None:
        report = build_coverage_report(self.registry, self.normative_map, ROOT)

        self.assertTrue(report["ok"], report["diagnostics"])
        self.assertEqual(report["counts"]["diagnostics"], 0)
        self.assertEqual(
            report["counts"]["normative_statements"],
            report["counts"]["normative_mappings"],
        )

    def test_reports_duplicate_identifiers(self) -> None:
        mutated = copy.deepcopy(self.registry)
        mutated["requirements"].append(copy.deepcopy(mutated["requirements"][0]))

        report = build_coverage_report(mutated, self.normative_map, ROOT)

        self.assertIn("duplicate_identifier", self.categories(report))

    def test_s_000_ut01_reports_noncanonical_order_and_stale_source_anchors(self) -> None:
        reordered = copy.deepcopy(self.registry)
        reordered["requirements"][0], reordered["requirements"][1] = (
            reordered["requirements"][1],
            reordered["requirements"][0],
        )
        reordered_report = build_coverage_report(reordered, self.normative_map, ROOT)
        self.assertIn("noncanonical_order", self.categories(reordered_report))

        stale = copy.deepcopy(self.registry)
        stale["source"]["sha256"] = "0" * 64
        stale["requirements"][0]["source"]["line"] = 1
        stale["requirements"][1]["source"]["heading"] = "Wrong heading"
        stale["requirements"][2]["source"]["definition_sha256"] = "0" * 64
        stale_report = build_coverage_report(stale, self.normative_map, ROOT)
        self.assertGreaterEqual(self.categories(stale_report).count("stale_source_anchor"), 4)

    def test_s_000_ut01_rejects_malformed_registry_documents(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "registry.json"
            path.write_text("[]\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "expected a JSON object"):
                load_json(path)

        malformed = copy.deepcopy(self.registry)
        malformed["schema_version"] = 999
        malformed["requirements"] = {}
        report = build_coverage_report(malformed, self.normative_map, ROOT)
        self.assertIn("malformed_registry", self.categories(report))

    def test_reports_unresolved_dependencies(self) -> None:
        mutated = copy.deepcopy(self.registry)
        product = next(
            item for item in mutated["requirements"]
            if item["kind"] == "product_requirement"
        )
        product["dependencies"] = ["AM-NOT-DEFINED"]

        report = build_coverage_report(mutated, self.normative_map, ROOT)

        self.assertIn("unresolved_dependency", self.categories(report))

    def test_reports_missing_and_unassigned_acceptance_tests(self) -> None:
        mutated = copy.deepcopy(self.registry)
        product = next(
            item for item in mutated["requirements"]
            if item["kind"] == "product_requirement"
        )
        product["acceptance_tests"] = []

        report = build_coverage_report(mutated, self.normative_map, ROOT)

        self.assertIn("missing_acceptance_test", self.categories(report))

    def test_reports_dependency_and_acceptance_test_release_mismatches(self) -> None:
        mutated = copy.deepcopy(self.registry)
        product = next(
            item for item in mutated["requirements"]
            if item["kind"] == "product_requirement" and item["dependencies"]
        )
        mutated_by_id = {item["id"]: item for item in mutated["requirements"]}
        mutated_by_id[product["dependencies"][0]]["release"] = "v9.0"
        mutated_by_id[product["acceptance_tests"][0]]["release"] = "v9.0"

        report = build_coverage_report(mutated, self.normative_map, ROOT)

        self.assertGreaterEqual(self.categories(report).count("release_mismatch"), 2)

    def test_reports_new_unmapped_normative_statement_without_editing_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = (ROOT / "PRD.md").read_text(encoding="utf-8")
            marker = "## 18. v0.1 Requirement Traceability"
            source = source.replace(
                marker,
                "AgentMage must reject this unmapped fixture.\n\n" + marker,
                1,
            )
            (root / "PRD.md").write_text(source, encoding="utf-8")

            report = build_coverage_report(self.registry, self.normative_map, root)

        self.assertIn("unmapped_normative_statement", self.categories(report))

    def test_reports_stale_wrong_kind_and_release_incompatible_mappings(self) -> None:
        stale = copy.deepcopy(self.normative_map)
        stale["mappings"][0]["statement_sha256"] = "0" * 64
        stale_report = build_coverage_report(self.registry, stale, ROOT)
        self.assertIn("stale_normative_mapping", self.categories(stale_report))

        wrong_kind = copy.deepcopy(self.normative_map)
        acceptance_test = next(
            item["id"] for item in self.registry["requirements"]
            if item["kind"] == "acceptance_test"
        )
        wrong_kind["mappings"][0]["requirement_ids"] = [acceptance_test]
        wrong_kind_report = build_coverage_report(self.registry, wrong_kind, ROOT)
        self.assertIn("unresolved_normative_reference", self.categories(wrong_kind_report))

        release_mismatch = copy.deepcopy(self.registry)
        mapped_id = self.normative_map["mappings"][0]["requirement_ids"][0]
        next(
            item for item in release_mismatch["requirements"] if item["id"] == mapped_id
        )["release"] = "v9.0"
        mismatch_report = build_coverage_report(
            release_mismatch,
            self.normative_map,
            ROOT,
        )
        self.assertIn("release_mismatch", self.categories(mismatch_report))

    def test_report_is_deterministic_and_does_not_mutate_inputs(self) -> None:
        registry_before = copy.deepcopy(self.registry)
        map_before = copy.deepcopy(self.normative_map)

        first = build_coverage_report(self.registry, self.normative_map, ROOT)
        second = build_coverage_report(self.registry, self.normative_map, ROOT)

        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(second, sort_keys=True))
        self.assertEqual(self.registry, registry_before)
        self.assertEqual(self.normative_map, map_before)


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import copy
import unittest

from scripts.kernel_architecture_report import (
    EXPECTED_UNMATERIALIZED,
    RULES_PATH,
    build_report,
    cargo_dependency_records,
    find_cycle,
    graph_findings,
    read_json,
    validate_report,
)


class KernelArchitectureReportTests(unittest.TestCase):
    def test_manifest_graph_has_only_the_seven_materialized_product_edges(self) -> None:
        internal, _ = cargo_dependency_records()
        graph = graph_findings(read_json(RULES_PATH), internal)
        self.assertEqual(graph["observed_product_edge_count"], 7)
        self.assertEqual(graph["prohibited_observed_edges"], [])
        self.assertIsNone(graph["observed_cycle"])
        self.assertEqual(
            {
                (edge["source"], edge["target"]): edge["reason"]
                for edge in graph["declared_but_unmaterialized_edges"]
            },
            EXPECTED_UNMATERIALIZED,
        )

    def test_cycle_detection_reports_the_exact_closed_path(self) -> None:
        edges = [
            {"source": "kernel-contracts", "target": "kernel-engine"},
            {"source": "kernel-engine", "target": "kernel-contracts"},
        ]
        self.assertEqual(
            find_cycle(edges),
            ["kernel-contracts", "kernel-engine", "kernel-contracts"],
        )

    def test_stale_or_broadened_report_is_rejected(self) -> None:
        report = build_report("a" * 40)
        self.assertEqual(validate_report(report), [])
        mutated = copy.deepcopy(report)
        mutated["graph"]["prohibited_observed_edges"].append(
            {"source": "kernel-engine", "target": "shell-host"}
        )
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()

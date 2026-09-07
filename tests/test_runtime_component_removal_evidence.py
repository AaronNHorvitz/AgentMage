"""Currentness and mutation tests for retained Story 50.2 removal evidence."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from scripts import runtime_component_removal as campaign

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT = ROOT / campaign.DEFAULT_OUTPUT


class RuntimeComponentRemovalEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = json.loads((ROOT / campaign.PROFILE).read_text(encoding="utf-8"))
        self.report = json.loads(
            (ARTIFACT / campaign.REPORT_NAME).read_text(encoding="utf-8")
        )

    def test_retained_report_sources_logs_and_manifests_recompute(self) -> None:
        self.assertEqual(
            campaign.report_failures(self.report, self.profile, ARTIFACT), []
        )
        self.assertTrue(self.report["campaign_passed"])
        self.assertEqual(self.report["disposition"], "LOCAL-SOURCE-PASS")
        self.assertEqual(
            sum(item["tests"]["passed"] for item in self.report["scenarios"]),
            1788,
        )
        self.assertEqual(
            sum(item["tests"]["ignored"] for item in self.report["scenarios"]),
            62,
        )

    def test_source_log_result_manifest_and_limitation_mutations_fail_closed(self) -> None:
        mutations = []

        source = copy.deepcopy(self.report)
        first_source = next(iter(source["source_sha256"]))
        source["source_sha256"][first_source] = "f" * 64
        mutations.append(source)

        log = copy.deepcopy(self.report)
        log["scenarios"][0]["test_log_sha256"] = "f" * 64
        mutations.append(log)

        result = copy.deepcopy(self.report)
        result["scenarios"][1]["removal_verified"] = False
        mutations.append(result)

        manifest = copy.deepcopy(self.report)
        manifest["scenarios"][2]["compiled_source_count"] += 1
        mutations.append(manifest)

        limitations = copy.deepcopy(self.report)
        limitations["declared_limitations"].pop()
        mutations.append(limitations)

        for index, value in enumerate(mutations):
            with self.subTest(index=index):
                self.assertTrue(
                    campaign.report_failures(value, self.profile, ARTIFACT)
                )


if __name__ == "__main__":
    unittest.main()

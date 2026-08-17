"""Currentness and mutation tests for retained Story 50.2 load evidence."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from scripts import runtime_hardening_load as campaign

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT = ROOT / campaign.DEFAULT_OUTPUT


class RuntimeHardeningLoadEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = json.loads(
            (ROOT / campaign.PROFILE).read_text(encoding="utf-8")
        )
        self.report = json.loads(
            (ARTIFACT / "report.json").read_text(encoding="utf-8")
        )

    def test_retained_report_sources_logs_and_metrics_recompute(self) -> None:
        self.assertEqual(
            campaign.report_failures(self.report, self.profile, ARTIFACT), []
        )
        self.assertTrue(self.report["campaign_passed"])
        self.assertEqual(self.report["disposition"], "PARTIAL-PASS")
        self.assertEqual(sum(item["tests"]["passed"] for item in self.report["commands"]), 39)

    def test_source_log_metric_and_disposition_mutations_fail_closed(self) -> None:
        mutations = []

        source = copy.deepcopy(self.report)
        first_source = next(iter(source["source_sha256"]))
        source["source_sha256"][first_source] = "f" * 64
        mutations.append(source)

        log = copy.deepcopy(self.report)
        log["commands"][0]["output_sha256"] = "f" * 64
        mutations.append(log)

        metric = copy.deepcopy(self.report)
        metric["metrics"]["event_count"] -= 1
        metric["commands"][0]["metrics"]["event_count"] -= 1
        mutations.append(metric)

        disposition = copy.deepcopy(self.report)
        disposition["campaign_passed"] = False
        disposition["disposition"] = "FAILED"
        mutations.append(disposition)

        fields = copy.deepcopy(self.report)
        del fields["declared_limitations"]
        mutations.append(fields)

        for index, value in enumerate(mutations):
            with self.subTest(index=index):
                self.assertTrue(
                    campaign.report_failures(value, self.profile, ARTIFACT)
                )


if __name__ == "__main__":
    unittest.main()

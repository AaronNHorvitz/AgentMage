from __future__ import annotations

import copy
import unittest

from scripts.engineering_artifact_rv51_evidence import (
    MARKERS,
    expected_report,
    validate_raw,
    validate_report,
)


class EngineeringArtifactRv51EvidenceTests(unittest.TestCase):
    def test_applicable_report_retains_every_required_evidence_family(self) -> None:
        report = expected_report()
        self.assertEqual(validate_report(report), [])
        self.assertEqual(len(report["fixture_manifests"]), 6)
        self.assertEqual(report["source_hash_count"], 26)
        self.assertEqual(report["derivative_hash_count"], 21)
        self.assertEqual(report["context_receipt_count"], 8)
        self.assertEqual(report["resource_observation_count"], 10)

    def test_zero_silent_drop_reconciles_all_sources_in_every_scenario(self) -> None:
        result = expected_report()["zero_silent_drop"]
        self.assertEqual(result["supplied_source_count"], 26)
        self.assertEqual(result["lineage_source_count"], 26)
        self.assertEqual(result["scenario_count"], 8)
        for scenario in result["scenario_results"]:
            self.assertEqual(scenario["explicit_disposition_count"], 26)
            self.assertEqual(scenario["missing_source_artifact_ids"], [])
            self.assertEqual(scenario["unexpected_source_artifact_ids"], [])
            self.assertEqual(scenario["duplicate_source_artifact_ids"], [])
            self.assertEqual(scenario["result"], "PASS_LOCAL_FIXTURE")

    def test_parser_and_hash_lineage_never_becomes_a_product_claim(self) -> None:
        report = expected_report()
        self.assertTrue(all(not item["product_parser_executed"] for item in report["parser_identities"]))
        self.assertEqual(sum(item["source_count"] for item in report["parser_identities"]), 26)
        for item in report["derivative_hashes"]:
            self.assertEqual(item["source_sha256"], item["derivative_sha256"])
            self.assertEqual(item["source_range"], item["derivative_range"])

    def test_required_unseen_context_and_hostile_content_never_complete(self) -> None:
        report = expected_report()
        blocked = [item for item in report["context_receipts"] if item["outcome"] == "blocked"]
        self.assertEqual(len(blocked), 4)
        self.assertTrue(all(not item["completion_allowed"] for item in blocked))
        self.assertTrue(all(item["required_unseen_artifact_ids"] for item in blocked))
        for result in report["hostile_fixture_results"]:
            self.assertFalse(result["active_content_executed"])
            self.assertFalse(result["external_relationship_fetched"])
            self.assertFalse(result["residue_retained"])
            self.assertFalse(result["may_claim_complete"])

    def test_full_protocol_and_acceptance_tests_remain_explicitly_partial(self) -> None:
        report = expected_report()
        self.assertFalse(report["protocol_complete"])
        self.assertEqual(report["status"], "PARTIAL_LOCAL_FIXTURE_EVIDENCE")
        self.assertTrue(all(item["status"] == "PARTIAL_LOCAL_FIXTURE" for item in report["acceptance_tests"]))
        self.assertEqual(len(report["remaining_protocol_scenarios"]), 4)

    def test_overclaims_hidden_gaps_and_missing_accounting_fail_validation(self) -> None:
        mutations = []
        overclaim = copy.deepcopy(expected_report())
        overclaim["protocol_complete"] = True
        overclaim["status"] = "PASS"
        mutations.append(overclaim)
        acceptance = copy.deepcopy(expected_report())
        acceptance["acceptance_tests"][0]["status"] = "PASS"
        mutations.append(acceptance)
        parser = copy.deepcopy(expected_report())
        parser["parser_identities"][0]["product_parser_executed"] = True
        mutations.append(parser)
        omitted = copy.deepcopy(expected_report())
        omitted["zero_silent_drop"]["scenario_results"][0]["missing_source_artifact_ids"] = ["source-missing"]
        mutations.append(omitted)
        hidden = copy.deepcopy(expected_report())
        hidden["remaining_protocol_scenarios"] = []
        mutations.append(hidden)
        for mutation in mutations:
            self.assertTrue(validate_report(mutation))

    def test_raw_results_require_every_marker_and_no_failure(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()

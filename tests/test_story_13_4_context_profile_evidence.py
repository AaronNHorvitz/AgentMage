from __future__ import annotations

import copy
import unittest

from scripts.story_13_4_context_profile_evidence import (
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_sources,
    validate_upstream,
)
from scripts.story_13_4_profile_campaign import expected_ledger, validate_ledger, validate_sources as validate_campaign_sources


class Story134ContextProfileEvidenceTests(unittest.TestCase):
    def test_sources_upstream_and_expected_report_are_current(self) -> None:
        self.assertEqual(validate_sources(), [])
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_every_execution_marker_is_required(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS:
            self.assertTrue(validate_raw(valid.replace(marker, "")), marker)

    def test_external_profile_and_release_overclaims_are_rejected(self) -> None:
        for field in (
            "live_admitted_profile_corpus_complete",
            "cross_platform_profile_campaign_complete",
            "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["product_truth"][field] = not TRUTH[field]
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "ready"
        self.assertTrue(validate_report(changed))

    def test_no_fallback_or_borrowed_candidate_disposition_is_permitted(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["automatic_fallback_enabled"] = True
        self.assertTrue(validate_report(changed))
        for field in ("muse_profile_disposition", "gemma_profile_disposition"):
            changed = copy.deepcopy(expected_report())
            changed["product_truth"][field] = "APPROVED"
            self.assertTrue(validate_report(changed), field)

    def test_profile_campaign_is_exact_non_aggregated_and_enables_nothing(self) -> None:
        campaign = expected_ledger()
        self.assertEqual(validate_campaign_sources(), [])
        self.assertEqual(validate_ledger(campaign), [])
        self.assertFalse(campaign["tuple_aggregation_permitted"])
        self.assertEqual(campaign["admitted_profile_count"], 0)
        self.assertEqual(campaign["future_admitted_profiles"], [])
        self.assertEqual(len(campaign["fixture_profile"]["context_ledgers"]), 8)
        self.assertEqual(campaign["fixture_profile"]["workflow_metrics"]["workflow_case_count"], 11)
        self.assertTrue(all(item["profile_metrics"] is None for item in campaign["rejected_candidates"]))

    def test_campaign_overclaim_and_identity_mutation_are_rejected(self) -> None:
        for mutation in (
            lambda value: value.update({"admitted_profile_count": 1}),
            lambda value: value.update({"tuple_aggregation_permitted": True}),
            lambda value: value["claims"].update({"live_admitted_profile_corpus_complete": True}),
            lambda value: value["fixture_profile"]["exact_identity"].update({"runtime_identity": "changed"}),
            lambda value: value["rejected_candidates"][0].update({"profile_metrics": {"borrowed": 1}}),
            lambda value: value["protocol_mappings"]["RV-13"].update({"status": "pass"}),
        ):
            changed = copy.deepcopy(expected_ledger())
            mutation(changed)
            self.assertTrue(validate_ledger(changed))


if __name__ == "__main__":
    unittest.main()

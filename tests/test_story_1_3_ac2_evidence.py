from __future__ import annotations

import copy
import unittest

from scripts.story_1_3_ac2_evidence import (
    MARKERS, TRUTH, expected_report, validate_raw, validate_report, validate_upstream,
)


class Story13Ac2EvidenceTests(unittest.TestCase):
    def test_current_cross_client_parity_evidence_is_complete(self) -> None:
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(TRUTH["canonical_record_family_count"], 9)
        self.assertEqual(TRUTH["independent_contract_caller_count"], 2)

    def test_deterministic_identity_and_byte_parity_is_required(self) -> None:
        for field in (
            "canonical_bytes_match_across_callers", "canonical_sha256_matches_across_callers",
            "content_and_identity_bindings_preserved", "serialized_enum_meaning_preserved",
            "borrowed_publication_matches_authoritative_encoder",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = False
            self.assertTrue(validate_report(changed), field)

    def test_client_authority_widening_is_rejected(self) -> None:
        for field in (
            "client_owns_runtime_state", "client_owns_persistence", "client_owns_lifecycle_transition",
            "client_receives_execution_authority", "model_inference_executed", "runtime_effect_executed",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)

    def test_later_gate_overclaims_are_rejected(self) -> None:
        for field in (
            "installed_client_parity_complete", "native_platform_complete",
            "independent_review_complete", "story_completion_claim", "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["release_claim"] = "pass"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_both_callers_and_upstream_evidence(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS:
            self.assertTrue(validate_raw(valid.replace(marker, "")), marker)
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()

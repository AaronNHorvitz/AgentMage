from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import story_13_4_dispatch_preflight_evidence as evidence


def report() -> dict[str, object]:
    raw = "\n".join(evidence.MARKERS).encode()
    with patch.object(evidence, "git_file", return_value=b"committed-source"):
        return evidence.expected_report("a" * 40, raw)


class Story134DispatchPreflightEvidenceTests(unittest.TestCase):
    def test_every_execution_marker_is_required(self) -> None:
        valid = "\n".join(evidence.MARKERS)
        self.assertEqual(evidence.validate_raw(valid), [])
        for marker in evidence.MARKERS:
            self.assertTrue(evidence.validate_raw(valid.replace(marker, "")), marker)

    def test_matrix_and_claim_limits_are_closed(self) -> None:
        value = report()
        self.assertEqual(set(value["ctx_fit"]), set(evidence.CTX_FIT))
        self.assertEqual(set(value["ctx_dispatch"]), set(evidence.CTX_DISPATCH))
        self.assertFalse(value["claims"]["native_boundary_campaign_executed"])
        self.assertFalse(value["claims"]["model_profile_enabled"])
        self.assertFalse(value["recovery"]["unchanged_retry"])
        self.assertTrue(value["recovery"]["compaction_requires_g2_admission"])

    def test_case_removal_and_overclaim_mutations_fail(self) -> None:
        raw = "\n".join(evidence.MARKERS).encode()
        baseline = report()
        mutations = (
            lambda value: value["ctx_fit"].pop(),
            lambda value: value["ctx_dispatch"].pop(),
            lambda value: value["stable_refusal_codes"].pop(),
            lambda value: value["claims"].update({"native_boundary_campaign_executed": True}),
            lambda value: value["claims"].update({"model_profile_enabled": True}),
            lambda value: value["recovery"].update({"unchanged_retry": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(baseline)
            mutate(changed)
            with patch.object(evidence, "git_file", return_value=b"committed-source"):
                self.assertTrue(evidence.validate_report(changed, "a" * 40, raw))


if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import copy
import unittest

from scripts import muse_early_disposition as disposition


class MuseEarlyDispositionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.record = disposition.build_record("HEAD")

    def test_current_evidence_produces_exact_rejection(self) -> None:
        self.assertEqual(disposition.validate_record(self.record), [])
        self.assertEqual(self.record["decision"]["status"], "REJECTED")
        self.assertFalse(self.record["decision"]["quality_profile_enabled"])
        self.assertFalse(self.record["decision"]["automatic_fallback"])

    def test_observation_activation_fallback_and_family_mutations_fail_closed(self) -> None:
        mutations = (
            lambda value: value["observed"].update({"quality_closed_proposal_count": 12}),
            lambda value: value["decision"].update({"status": "PASS-EVALUATION"}),
            lambda value: value["decision"].update({"quality_profile_enabled": True}),
            lambda value: value["decision"].update({"automatic_fallback": True}),
            lambda value: value["decision"].update({"family_wide_conclusion": True}),
            lambda value: value["evidence"][0].update({"sha256": "0" * 64}),
            lambda value: value["re_review_triggers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(self.record)
            mutate(changed)
            self.assertTrue(disposition.validate_record(changed))


if __name__ == "__main__":
    unittest.main()

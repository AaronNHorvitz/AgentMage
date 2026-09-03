from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_25_incident_tabletop as tabletop


def report() -> dict[str, object]:
    with patch.object(tabletop, "source_bytes", return_value=b"source"):
        return tabletop.build_report("a" * 40)


class Sprint25IncidentTabletopTests(unittest.TestCase):
    def test_complete_tabletop_passes_without_production_overclaim(self) -> None:
        value = report()
        self.assertEqual(tabletop.validate_report(value, verify_current=False), [])
        self.assertEqual(value["summary"], {"result": "PASS", "scenario_count": 4})
        self.assertFalse(value["human_participant_claim"])

    def test_scenario_signal_transition_and_role_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["scenarios"].pop(),
            lambda value: value["scenarios"][0]["injected_signals"].pop(),
            lambda value: value["scenarios"][0]["timeline"].pop(),
            lambda value: value["participants"].pop(),
            lambda value: value["participants"][0].update({"component_independent": False}),
        )
        for mutate in mutations:
            value = copy.deepcopy(report())
            mutate(value)
            self.assertTrue(tabletop.validate_report(value, verify_current=False))

    def test_authority_content_communication_and_claim_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["scenarios"][0]["timeline"][0].update(
                {"authority_delta": "network"}
            ),
            lambda value: value["scenarios"][0]["timeline"][0].update(
                {"private_content_retained": True}
            ),
            lambda value: value["scenarios"][0]["timeline"][0].update(
                {"external_notification_sent": True}
            ),
            lambda value: value.update({"human_participant_claim": True}),
            lambda value: value["content_scan"].update({"canary_matches": 1}),
            lambda value: value["verification"].update({"production_rv22_claim": True}),
        )
        for mutate in mutations:
            value = copy.deepcopy(report())
            mutate(value)
            self.assertTrue(tabletop.validate_report(value, verify_current=False))


if __name__ == "__main__":
    unittest.main()

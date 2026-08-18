from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_15_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {"id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64}
        for identifier, argv in evidence.COMMANDS
    ]


def state() -> dict[str, object]:
    return {
        "doctor_component_count": 13,
        "doctor_fixture_count": 100,
        "manual_selection": True,
        "automatic_fallback": False,
        "inference_slot_default": 1,
        "resource_limit_classes": 9,
        "candidate_count": 416,
        "candidate_dispositions": {"BLOCKED": 415, "INELIGIBLE": 1},
        "exact_muse_disposition": "REJECTED",
        "enabled_model_count": 0,
        "unsupported_claim_count": 0,
        "reviewed_export": True,
    }


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"source"),
        patch.object(evidence, "sha256_file", return_value="b" * 64),
        patch.object(evidence, "evidence_state", return_value=state()),
    ):
        return evidence.build_report("c" * 40, commands())


class Sprint15EvidenceTests(unittest.TestCase):
    def test_local_evidence_closes_without_product_or_release_overclaim(self) -> None:
        value = report()
        with patch.object(evidence, "evidence_state", return_value=state()):
            self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertTrue(value["summary"]["local_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(len(value["blockers"]), 5)
        self.assertTrue(value["stories"][0]["kernel_resource_stop_unloads_model"])
        self.assertTrue(value["stories"][0]["chat_model_selection"])

    def test_product_resource_accessibility_and_model_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"product_model_available": True}),
            lambda value: value["stories"][0].update({"os_worker_resource_enforcement": True}),
            lambda value: value["stories"][0].update({"kernel_resource_stop_unloads_model": False}),
            lambda value: value["stories"][0].update({"chat_model_selection": False}),
            lambda value: value["stories"][1].update({"full_surface_canary_sweep": True}),
            lambda value: value["stories"][2].update({"exact_gemma_trials_complete": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            with patch.object(evidence, "evidence_state", return_value=state()):
                self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_command_and_security_mapping_mutations_fail(self) -> None:
        for mutate in (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"]["15.1"].pop(),
        ):
            changed = copy.deepcopy(report())
            mutate(changed)
            with patch.object(evidence, "evidence_state", return_value=state()):
                self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()

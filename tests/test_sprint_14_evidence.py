from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_14_evidence as evidence


def command_records() -> list[dict[str, object]]:
    return [
        {
            "id": identifier,
            "argv": list(argv),
            "exit_code": 0,
            "output_sha256": "a" * 64,
        }
        for identifier, argv in evidence.COMMANDS
    ]


def state() -> dict[str, object]:
    return {
        "frozen_source_entries": 416,
        "google_source_entries": 412,
        "meta_source_entries": 4,
        "normalized_entries": 416,
        "inventory_validation_failures": [],
        "inventory_dispositions": {
            "BLOCKED": 415,
            "BLOCKED-HARDWARE": 0,
            "CANDIDATE": 0,
            "INELIGIBLE": 1,
            "REJECTED": 0,
        },
        "role_matrix_entries": 416,
        "enabled_models": 0,
        "authorized_acquisitions": 0,
        "packaged_installer_records": 2,
        "packaged_installer_modes": [493],
        "isolated_capture_harness_passed": True,
        "capture_observed_product_acquisition": False,
    }


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"source"),
        patch.object(evidence, "sha256_file", return_value="b" * 64),
        patch.object(evidence, "evidence_state", return_value=state()),
    ):
        return evidence.build_report("c" * 40, command_records())


class Sprint14EvidenceTests(unittest.TestCase):
    def test_local_contract_closure_preserves_external_and_process_blockers(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertTrue(value["summary"]["local_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(len(value["blockers"]), 6)
        self.assertTrue(value["stories"][0]["acquisition_review_facts_displayed"])

    def test_process_artifact_platform_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["stories"][0].update({"end_user_process_protocol_active": True}),
            lambda value: value["stories"][1].update({"exact_artifact_profiles_admitted": True}),
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["summary"].update({"product_activation": True}),
            lambda value: value["summary"].update({"release_approval": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_command_and_security_map_mutations_fail(self) -> None:
        for mutate in (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"]["14.1"].pop(),
        ):
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()

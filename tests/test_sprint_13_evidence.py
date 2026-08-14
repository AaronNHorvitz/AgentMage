from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_13_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {
            "id": identifier,
            "story_id": story,
            "argv": list(argv),
            "exit_code": 0,
            "output_sha256": "a" * 64,
        }
        for identifier, argv, story in evidence.COMMANDS
    ]


def report() -> dict[str, object]:
    with (
        patch.object(evidence, "git_file", return_value=b"committed-source"),
        patch.object(evidence, "sha256_file", return_value="b" * 64),
        patch.object(evidence, "production_family_references", return_value=[]),
        patch.object(
            evidence,
            "evidence_state",
            return_value={
                "adapter_contract": "CONTRACT-PASS-LIVE-BLOCKED",
                "codec_contract": "PASS-CONTRACT",
                "live_lifecycle": "ISOLATED-LIFECYCLE-PASS",
                "live_inference": "SANDBOXED-LIVE-INFERENCE-PASS",
                "muse_evaluation": "REJECTED",
                "muse_disposition": "REJECTED",
                "gemma_e4b_disposition": "REJECTED",
                "gemma_12b_disposition": "REJECTED",
                "packet_capture_executed": False,
                "quality_trial_count": 12,
                "repeatability_trial_count": 5,
            },
        ),
    ):
        return evidence.build_report("c" * 40, commands())


class Sprint13EvidenceTests(unittest.TestCase):
    def test_local_contract_pass_preserves_all_external_blockers(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertTrue(value["summary"]["local_contract_passed"])
        self.assertEqual(value["summary"]["candidate_disposition"], "REJECTED")
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(len(value["blockers"]), 3)

    def test_command_family_activation_release_and_blocker_mutations_fail(self) -> None:
        value = report()
        mutations = (
            lambda item: item["commands"][0].update({"exit_code": 1}),
            lambda item: item["production_kernel_family_references"].append(
                {"family": "muse"}
            ),
            lambda item: item["stories"][0].update({"macos_adapter_implemented": True}),
            lambda item: item["stories"][1].update({"docker_live_parity_evidence": True}),
            lambda item: item["stories"][2].update({"profile_enabled": True}),
            lambda item: item["blockers"].pop(),
            lambda item: item["summary"].update({"automatic_fallback": True}),
            lambda item: item["summary"].update({"release_approval": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(value)
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()

"""Tests for independent Docker control-disablement evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import linux_docker_control_disablement_evidence as evidence


class DockerControlDisablementEvidenceTests(unittest.TestCase):
    def minimal_report(self) -> dict[str, object]:
        cases = [
            {
                "control": control,
                "mutation": mutation,
                "expected_refusal": refusal,
                "observed_refusal": refusal,
                "docker_admitted": False,
                "native_fallback_selected": False,
                "inference_performed": False,
                "cleanup": {"complete": True},
            }
            for control, mutation, refusal in evidence.CONTROLS
        ]
        observation = {
            "baseline": {"status": "admitted", "cleanup": {"complete": True}},
            "native_descriptor": {
                "docker_compatibility_available": False,
                "inference_available": False,
                "enabled_models": 0,
            },
            "cases": cases,
            "fallback_selected": False,
            "inference_performed": False,
            "final_cleanup": {"complete": True},
        }
        return {
            "artifact_id": "linux-docker-independent-control-disablement",
            "task_ids": ["9.2.2.3"],
            "source_revision": "a" * 40,
            "targets": [
                {
                    "target_id": target.target_id,
                    "observation": copy.deepcopy(observation),
                    "host_cleanup": {"complete": True},
                }
                for target in evidence.kvm.TARGETS
            ],
            "fallback_selected": False,
            "inference_performed": False,
            "release_support": False,
            "sources": [
                {"path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
        }

    def test_exact_control_matrix_is_accepted(self) -> None:
        self.assertEqual(evidence.validate(self.minimal_report()), [])

    def test_each_control_refusal_is_required(self) -> None:
        for index, (control, _, _) in enumerate(evidence.CONTROLS):
            with self.subTest(control=control):
                changed = self.minimal_report()
                changed["targets"][0]["observation"]["cases"][index][
                    "observed_refusal"
                ] = "docker-preflight.observation.identity"
                self.assertTrue(evidence.validate(changed))

    def test_fallback_or_inference_overclaim_is_rejected(self) -> None:
        for field in ("fallback_selected", "inference_performed", "release_support"):
            with self.subTest(field=field):
                changed = self.minimal_report()
                changed[field] = True
                self.assertIn(
                    "control-disablement report overclaimed",
                    evidence.validate(changed),
                )


if __name__ == "__main__":
    unittest.main()

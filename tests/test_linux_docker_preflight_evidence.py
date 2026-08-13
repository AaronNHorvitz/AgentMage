from __future__ import annotations

import copy
import unittest

from scripts import linux_docker_preflight_evidence as evidence


class LinuxDockerPreflightEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "artifact_id": "linux-docker-drift-preflight",
            "admission_contract": {
                "automatic_fallback": False,
                "complete_observation_required": True,
                "fresh_session_required": True,
                "permit_cloneable": False,
                "permit_serializable": False,
                "replay_allowed": False,
                "terminal_on_refusal": True,
            },
            "contract_version": 2,
            "docker_engine_directly_tested": False,
            "host": {
                "architecture": "x86_64",
                "distribution": "fedora",
                "version": "44",
                "docker_cli_available": False,
                "live_collector_executed": False,
            },
            "limitations": list(evidence.LIMITATIONS),
            "live_topology_inspected": False,
            "mutation_dimensions": copy.deepcopy(evidence.MUTATION_DIMENSIONS),
            "mutation_dimension_count": sum(
                len(value) for value in evidence.MUTATION_DIMENSIONS.values()
            ),
            "refusal_classes": list(evidence.REFUSAL_CLASSES),
            "release_claim": "none",
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-preflight-contract-no-live-docker",
            "task_ids": ["9.2.1.4"],
        }

    def test_expected_report_contract_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_every_refusal_class_and_mutation_dimension_is_unique(self) -> None:
        self.assertEqual(len(evidence.REFUSAL_CLASSES), 9)
        self.assertEqual(len(evidence.REFUSAL_CLASSES), len(set(evidence.REFUSAL_CLASSES)))
        flattened = [
            dimension
            for dimensions in evidence.MUTATION_DIMENSIONS.values()
            for dimension in dimensions
        ]
        self.assertEqual(len(flattened), len(set(flattened)))
        self.assertGreaterEqual(len(flattened), 50)

    def test_live_and_fallback_overclaims_fail_closed(self) -> None:
        mutations = (
            lambda value: value.update({"docker_engine_directly_tested": True}),
            lambda value: value.update({"live_topology_inspected": True}),
            lambda value: value["host"].update({"live_collector_executed": True}),
            lambda value: value["admission_contract"].update(
                {"automatic_fallback": True}
            ),
            lambda value: value["admission_contract"].update(
                {"terminal_on_refusal": False}
            ),
            lambda value: value.update({"release_claim": "pass"}),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                changed = self.valid_report()
                mutate(changed)
                self.assertTrue(evidence.validate_report(changed))

    def test_missing_refusal_or_mutation_fails_closed(self) -> None:
        changed = self.valid_report()
        changed["refusal_classes"].pop()
        self.assertIn(
            "Docker preflight refusal closure changed",
            evidence.validate_report(changed),
        )
        changed = self.valid_report()
        changed["mutation_dimensions"]["zero_egress"].pop()
        changed["mutation_dimension_count"] -= 1
        self.assertIn(
            "Docker preflight mutation closure changed",
            evidence.validate_report(changed),
        )


if __name__ == "__main__":
    unittest.main()

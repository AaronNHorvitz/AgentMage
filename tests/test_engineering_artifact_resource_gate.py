from __future__ import annotations

import copy
import json
import unittest

from scripts import engineering_artifact_resource_gate as gate


class EngineeringArtifactResourceGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.suite = json.loads(gate.OUTPUT_PATH.read_text(encoding="utf-8"))

    def test_all_named_resource_families_reach_and_refuse_the_boundary(self) -> None:
        observations = self.suite["resource_observations"]
        self.assertEqual(
            [item["dimension"] for item in observations],
            ["bytes", "pages", "rows", "cells", "files", "archive", "time", "memory", "disk", "processes"],
        )
        for observation in observations:
            self.assertEqual(observation["peak_admitted"], observation["ceiling"])
            self.assertGreater(observation["first_denied_attempt"], observation["ceiling"])
            self.assertFalse(observation["overflow_admitted"])
            self.assertTrue(observation["runtime_probe"])

    def test_real_local_runtime_probes_are_bounded(self) -> None:
        self.assertEqual(
            gate.runtime_probes(),
            {
                "bytes": True,
                "pages": True,
                "rows": True,
                "cells": True,
                "files": True,
                "archive": True,
                "time": True,
                "memory": True,
                "disk": True,
                "processes": True,
            },
        )

    def test_every_terminal_boundary_stops_workers_and_cleans_owned_state(self) -> None:
        observations = self.suite["lifecycle_observations"]
        self.assertEqual([item["event"] for item in observations], list(gate.LIFECYCLE_EVENTS))
        for observation in observations:
            self.assertTrue(observation["worker_stopped"])
            self.assertGreaterEqual(observation["residue_before_cleanup"], 1)
            self.assertEqual(observation["residue_after_cleanup"], 0)
            self.assertTrue(observation["owned_root_removed"])
            self.assertEqual(observation["cleanup_result"], "verified_zero_residue")

    def test_fault_probes_reproduce_without_retaining_private_paths(self) -> None:
        for event in gate.LIFECYCLE_EVENTS:
            observation = gate.lifecycle_probe(event)
            self.assertTrue(observation["worker_stopped"])
            self.assertTrue(observation["owned_root_removed"])
            self.assertNotIn("/tmp/", json.dumps(observation))

    def test_suite_is_self_hashed_and_dependency_bound(self) -> None:
        unhashed = copy.deepcopy(self.suite)
        recorded = unhashed["suite_sha256"]
        unhashed["suite_sha256"] = "0" * 64
        self.assertEqual(recorded, gate.sha256_bytes(gate.canonical_json(unhashed)))
        self.assertEqual(
            [item["path"] for item in self.suite["dependencies"]],
            [
                "fixtures/artifact-admission/v1/adversarial-manifest.json",
                "fixtures/artifact-admission/v1/context-delivery-receipts.json",
            ],
        )

    def test_mutations_and_overclaims_fail_validation(self) -> None:
        mutations = []
        missing = copy.deepcopy(self.suite)
        missing["resource_observations"].pop()
        mutations.append(missing)
        overflow = copy.deepcopy(self.suite)
        overflow["resource_observations"][0]["overflow_admitted"] = True
        mutations.append(overflow)
        residue = copy.deepcopy(self.suite)
        residue["lifecycle_observations"][0]["residue_after_cleanup"] = 1
        mutations.append(residue)
        overclaim = copy.deepcopy(self.suite)
        overclaim["rv51_claim"] = "passed"
        mutations.append(overclaim)
        for mutation in mutations:
            self.assertTrue(gate.validate_suite(mutation))

    def test_checked_suite_matches_reproducible_builder(self) -> None:
        self.assertEqual(gate.build_suite(), self.suite)
        self.assertEqual(gate.check(), [])


if __name__ == "__main__":
    unittest.main()

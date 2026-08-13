"""Tests for Docker hostile-position reachability evidence."""

from __future__ import annotations

import unittest

from scripts import linux_docker_reachability_evidence as evidence


class DockerReachabilityEvidenceTests(unittest.TestCase):
    def test_position_closure_is_exact(self) -> None:
        self.assertEqual(len(evidence.POSITIONS), 7)
        self.assertEqual(evidence.POSITIONS[0], "host")
        self.assertEqual(evidence.POSITIONS[-1], "ordinary-container")

    def test_validator_rejects_missing_targets(self) -> None:
        report = {
            "artifact_id": "linux-docker-hostile-reachability",
            "task_ids": ["9.2.2.2"],
            "source_revision": "a" * 40,
            "targets": [],
            "inference_performed": False,
            "release_support": False,
            "sources": [],
        }
        failures = "; ".join(evidence.validate(report))
        self.assertIn("target closure", failures)
        self.assertIn("source closure", failures)


if __name__ == "__main__":
    unittest.main()

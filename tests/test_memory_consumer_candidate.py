"""Validate a consumer-owned scenario specification, NOT a runtime or producer."""

import hashlib
import json
import re
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/memory-consumer/v1/conformance-candidate.json"
DOCUMENT = ROOT / "docs/architecture/generic-memory-consumer-requirements-v1.md"
BASELINE = "5b731b36d646aa22baa78f310544ea6ebce668e3"


def closed_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate candidate field")
        result[key] = value
    return result


class MemoryConsumerCandidateTests(unittest.TestCase):
    def candidate(self):
        return json.loads(FIXTURE.read_bytes(), object_pairs_hook=closed_object)

    def test_candidate_is_not_producer_approval_or_execution_evidence(self):
        fixture = self.candidate()
        self.assertEqual(set(fixture), {
            "fixture_schema_version", "contract_role", "status", "counterpart_approval",
            "live_integration", "production_enabled", "source_baseline_commit", "profile", "cases",
        })
        self.assertEqual(fixture["fixture_schema_version"], 1)
        self.assertEqual(fixture["contract_role"], "agentmage_consumer_requirements_only")
        self.assertEqual(fixture["status"], "candidate_unexecuted")
        for key in ("counterpart_approval", "live_integration", "production_enabled"):
            self.assertIs(fixture[key], False)
        self.assertEqual(fixture["source_baseline_commit"], BASELINE)

    def test_specification_has_exact_case_closure_and_bounded_profile(self):
        fixture = self.candidate()
        self.assertEqual(fixture["profile"], {
            "max_hits": 16, "max_excerpt_bytes": 65536,
            "max_reply_bytes": 1048576, "max_elapsed_ms": 5000,
        })
        self.assertEqual([case["id"] for case in fixture["cases"]], [
            f"AMC-{index:03d}" for index in range(1, 35)
        ])
        for case in fixture["cases"]:
            self.assertEqual(set(case), {"id", "input", "expected"})
            for field in ("input", "expected"):
                self.assertIsInstance(case[field], str)
                self.assertTrue(0 < len(case[field].encode()) <= 512)
        # These strings describe future cases; no adapter or counterpart is invoked.

    def test_existing_owner_pins_resolve_only_in_agentmage_history(self):
        document = DOCUMENT.read_text()
        records = re.findall(r"^\| ([a-zA-Z0-9_./-]+) \| ([0-9a-f]{64}) \|$", document, re.M)
        self.assertEqual(len(records), 7)
        self.assertEqual(len({path for path, _ in records}), 7)
        for path, expected in records:
            self.assertFalse(Path(path).is_absolute())
            self.assertNotIn("..", Path(path).parts)
            result = subprocess.run(
                ["git", "show", f"{BASELINE}:{path}"], cwd=ROOT,
                capture_output=True, check=True, timeout=30,
            )
            self.assertEqual(hashlib.sha256(result.stdout).hexdigest(), expected, path)

    def test_consumer_scope_and_unperformed_gates_stay_explicit(self):
        document = DOCUMENT.read_text()
        for boundary in (
            "Accepted under owner delegation, 2026-09-20",
            "never to counterpart approval", "CAP-29 runtime memory",
            "dependent on AMR-02/AMR-04", "or the producer wire",
            "NOT passing test evidence", "No arbitrary SQL",
        ):
            # Documentation phrases are deliberate refusal/ownership requirements.
            self.assertIn(boundary, document)
        task = next(line for line in (ROOT / "TASKS.md").read_text().splitlines()
                    if "| AMR-03.1.3 |" in line)
        self.assertTrue(task.startswith("| [ ] |"))
        self.assertIn("unexecuted specifications", task)


if __name__ == "__main__":
    unittest.main()

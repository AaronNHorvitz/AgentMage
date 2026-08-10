from __future__ import annotations

import json
import unittest

from scripts.kernel_contract_fixtures import (
    EXPECTED_VALID_NAMES,
    FixtureValidationError,
    invalid_fixtures,
    parse_bundle,
)


class KernelContractFixtureTests(unittest.TestCase):
    def test_bundle_parser_rejects_duplicate_and_incomplete_types(self) -> None:
        duplicate = [
            {"name": "task", "canonical_json": "{}"},
            {"name": "task", "canonical_json": "{}"},
        ]
        with self.assertRaises(FixtureValidationError):
            parse_bundle(
                (
                    "AGENTMAGE_CONTRACT_FIXTURES=" + json.dumps(duplicate) + "\n"
                ).encode()
            )
        incomplete = [{"name": name, "canonical_json": "{}"} for name in EXPECTED_VALID_NAMES[:-1]]
        with self.assertRaises(FixtureValidationError):
            parse_bundle(
                (
                    "AGENTMAGE_CONTRACT_FIXTURES=" + json.dumps(incomplete) + "\n"
                ).encode()
            )

    def test_invalid_fixture_mutations_are_deterministic_and_closed(self) -> None:
        task = {
            "schema_version": 1,
            "task_id": "task-0001",
            "session_id": "session-0001",
            "objective": "fixture",
            "acceptance_criteria": [],
            "constraints": [],
            "status": "ready",
        }
        encoded = json.dumps(task, separators=(",", ":")).encode()
        first = invalid_fixtures(encoded)
        second = invalid_fixtures(encoded)
        self.assertEqual(first, second)
        self.assertEqual(len(first), 7)
        self.assertNotIn(b"objective", first["task.v1.missing-field.json"])
        self.assertIn(b"capability_grant", first["task.v1.unknown-field.json"])
        self.assertTrue(first["task.v1.trailing-value.json"].endswith(b"[]"))


if __name__ == "__main__":
    unittest.main()

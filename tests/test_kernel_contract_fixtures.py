from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.kernel_contract_fixtures import (
    EXPECTED_VALID_NAMES,
    FixtureValidationError,
    enable_fixture_verifier,
    invalid_fixtures,
    parse_bundle,
)


class KernelContractFixtureTests(unittest.TestCase):
    def test_verifier_target_is_added_explicitly_to_normalized_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_name:
            root = Path(temporary_name)
            (root / "Cargo.toml").write_text("[package]\nname = \"fixture\"\n")
            enable_fixture_verifier(root)
            manifest = (root / "Cargo.toml").read_text()
            self.assertIn('name = "fixture_verifier"', manifest)
            self.assertIn('path = "tests/fixture_verifier.rs"', manifest)

    def test_bundle_parser_rejects_duplicate_and_incomplete_types(self) -> None:
        self.assertEqual(len(EXPECTED_VALID_NAMES), 15)
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
            "schema_version": 2,
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
        self.assertEqual(len(first), 8)
        self.assertNotIn(b"objective", first["task.v2.missing-field.json"])
        self.assertIn(b"capability_grant", first["task.v2.unknown-field.json"])
        self.assertTrue(first["task.v2.trailing-value.json"].endswith(b"[]"))


if __name__ == "__main__":
    unittest.main()

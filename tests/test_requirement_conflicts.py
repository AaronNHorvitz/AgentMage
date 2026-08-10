from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.requirement_conflicts import (
    DEFAULT_POLICY,
    ConflictPolicyError,
    load_policy,
    resolve_conflict,
)


def boundary(identifier: str) -> dict[str, object]:
    return {
        "id": identifier,
        "allow": {
            "actions": ["read", "search"],
            "releases": ["v0.1", "v0.2"],
        },
        "deny": {"capabilities": ["shell"]},
        "required_controls": ["receipt"],
        "ceilings": {"max_bytes": 1024, "max_network_bytes": 0},
        "exact": {"authority_model": "capability_grant"},
    }


class ConflictPolicyTests(unittest.TestCase):
    def test_policy_is_versioned_and_fail_closed(self) -> None:
        policy = load_policy()
        self.assertEqual(policy["schema_version"], 1)
        self.assertEqual(
            policy["automatic_resolution"],
            "provisional_narrower_boundary_only",
        )
        self.assertEqual(policy["unknown_field_behavior"], "blocked")

    def test_rejects_unsupported_or_weakened_policy(self) -> None:
        policy = json.loads(DEFAULT_POLICY.read_text(encoding="utf-8"))
        policy["schema_version"] = 2
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "policy.json"
            path.write_text(json.dumps(policy), encoding="utf-8")
            with self.assertRaisesRegex(ConflictPolicyError, "unsupported"):
                load_policy(path)

        policy = json.loads(DEFAULT_POLICY.read_text(encoding="utf-8"))
        policy["empty_intersection_behavior"] = "allow"
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "policy.json"
            path.write_text(json.dumps(policy), encoding="utf-8")
            with self.assertRaisesRegex(ConflictPolicyError, "must block"):
                load_policy(path)


class ConflictResolutionTests(unittest.TestCase):
    def test_resolves_to_narrower_provisional_boundary(self) -> None:
        left = boundary("REQ-LEFT")
        right = boundary("REQ-RIGHT")
        right["allow"]["actions"] = ["read"]
        right["allow"]["releases"] = ["v0.2"]
        right["deny"]["capabilities"] = ["browser"]
        right["required_controls"] = ["sandbox"]
        right["ceilings"]["max_bytes"] = 512

        result = resolve_conflict(left, right)

        self.assertEqual(result["status"], "provisional")
        self.assertTrue(result["requires_approved_decision"])
        self.assertEqual(result["boundary"]["allow"]["actions"], ["read"])
        self.assertEqual(result["boundary"]["allow"]["releases"], ["v0.2"])
        self.assertEqual(
            result["boundary"]["deny"]["capabilities"],
            ["browser", "shell"],
        )
        self.assertEqual(
            result["boundary"]["required_controls"], ["receipt", "sandbox"]
        )
        self.assertEqual(result["boundary"]["ceilings"]["max_bytes"], 512)
        self.assertEqual(result["unresolved_fields"], [])
        self.assertEqual(resolve_conflict(right, left), result)

    def test_blocks_empty_scope_and_exact_identity_conflicts(self) -> None:
        left = boundary("REQ-LEFT")
        right = boundary("REQ-RIGHT")
        right["allow"]["actions"] = ["write"]
        right["exact"]["authority_model"] = "ambient_authority"

        result = resolve_conflict(left, right)

        self.assertEqual(result["status"], "blocked")
        self.assertEqual(
            result["unresolved_fields"],
            ["allow.actions", "exact.authority_model"],
        )
        self.assertEqual(result["boundary"]["allow"]["actions"], [])
        self.assertNotIn("authority_model", result["boundary"]["exact"])

    def test_equivalent_boundaries_need_no_decision(self) -> None:
        left = boundary("REQ-LEFT")
        right = boundary("REQ-RIGHT")

        result = resolve_conflict(left, right)

        self.assertEqual(result["status"], "unchanged")
        self.assertFalse(result["requires_approved_decision"])
        self.assertEqual(result["sources"], ["REQ-LEFT", "REQ-RIGHT"])
        self.assertEqual(result["boundary"]["id"], "equivalent:REQ-LEFT+REQ-RIGHT")

    def test_rejects_unknown_fields_and_invalid_ceilings(self) -> None:
        left = boundary("REQ-LEFT")
        right = boundary("REQ-RIGHT")
        right["allow"]["undeclared"] = ["anything"]
        with self.assertRaisesRegex(ConflictPolicyError, "unknown allow fields"):
            resolve_conflict(left, right)

        right = boundary("REQ-RIGHT")
        right["ceilings"]["max_bytes"] = -1
        with self.assertRaisesRegex(ConflictPolicyError, "finite and non-negative"):
            resolve_conflict(left, right)

    def test_resolution_does_not_mutate_inputs(self) -> None:
        left = boundary("REQ-LEFT")
        right = boundary("REQ-RIGHT")
        before_left = copy.deepcopy(left)
        before_right = copy.deepcopy(right)

        resolve_conflict(left, right)

        self.assertEqual(left, before_left)
        self.assertEqual(right, before_right)


if __name__ == "__main__":
    unittest.main()

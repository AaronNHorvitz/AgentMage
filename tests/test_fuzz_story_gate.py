from __future__ import annotations

import copy
import unittest

from scripts.fuzz_story_gate import (
    EVIDENCE_FIELDS,
    POLICY_PATH,
    REPORT_PATH,
    build_policy,
    build_report,
    check_artifacts,
    evaluate_sprint_gate,
    read_json,
    validate_policy,
    validate_report,
)


def passing_evidence(target_id: str, policy: dict) -> dict:
    return {
        "target_id": target_id,
        "boundary_implemented": True,
        "target_registered": True,
        "target_registry_sha256": policy["inputs"]["target_registry"]["sha256"],
        "corpus_path": f"fuzzing/corpus/{target_id}/v1",
        "corpus_sha256": "1" * 64,
        "resource_policy_sha256": policy["inputs"]["toolchain_policy"]["sha256"],
        "result_path": f"artifacts/fuzz/{target_id}/result.json",
        "result_sha256": "2" * 64,
        "result_schema_valid": True,
        "result_status": "pass",
        "regression_path": f"fuzzing/regressions/{target_id}",
        "regression_replay_passed": True,
    }


class FuzzStoryGateTests(unittest.TestCase):
    def test_checked_policy_and_report_are_current(self) -> None:
        self.assertEqual(check_artifacts(), [])
        self.assertEqual(read_json(POLICY_PATH), build_policy())
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_every_target_is_assigned_to_all_declared_owner_gates(self) -> None:
        policy = build_policy()
        self.assertEqual(len(policy["owner_gates"]), 13)
        self.assertEqual(sum(item["target_count"] for item in policy["owner_gates"]), 14)
        self.assertEqual(policy["required_evidence_fields"], list(EVIDENCE_FIELDS))

    def test_complete_passing_evidence_can_pass_an_owner_gate(self) -> None:
        policy = build_policy()
        evidence = [
            passing_evidence("FT-MANIFEST-001", policy),
            passing_evidence("FT-CONFIG-001", policy),
        ]
        result = evaluate_sprint_gate(3, evidence)
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["blocking_target_count"], 0)

    def test_missing_evidence_or_unimplemented_boundary_blocks(self) -> None:
        policy = build_policy()
        evidence = passing_evidence("FT-PATH-001", policy)
        evidence["boundary_implemented"] = False
        result = evaluate_sprint_gate(6, [evidence])
        self.assertEqual(result["status"], "block")
        self.assertIn("boundary-not-implemented", result["target_results"][0]["failures"])
        missing = evaluate_sprint_gate(6, [])
        self.assertEqual(missing["status"], "block")
        self.assertIn("evidence-field-closure", missing["target_results"][0]["failures"])

    def test_non_pass_schema_resource_and_regression_states_block(self) -> None:
        policy = build_policy()
        for field, value, expected in (
            ("result_status", "timeout", "result-non-pass"),
            ("result_schema_valid", False, "result-schema-invalid"),
            ("resource_policy_sha256", "0" * 64, "resource-policy-stale"),
            ("regression_replay_passed", False, "regression-replay-non-pass"),
        ):
            evidence = passing_evidence("FT-GRANT-001", policy)
            evidence[field] = value
            result = evaluate_sprint_gate(5, [evidence])
            self.assertEqual(result["status"], "block")
            self.assertIn(expected, result["target_results"][0]["failures"])

    def test_current_future_gates_remain_blocked_without_product_boundaries(self) -> None:
        policy = build_policy()
        self.assertEqual(policy["current_ready_gate_count"], 0)
        self.assertEqual(policy["current_blocked_gate_count"], 13)
        self.assertTrue(
            all(
                item["status"] == "blocked-boundary-not-implemented"
                for item in policy["current_readiness"]
            )
        )

    def test_waiver_readiness_and_macos_overclaims_are_rejected(self) -> None:
        policy = build_policy()
        waiver = copy.deepcopy(policy)
        waiver["evaluation_contract"]["normal_development_waiver_permitted"] = True
        ready = copy.deepcopy(policy)
        ready["current_ready_gate_count"] = 1
        macos = copy.deepcopy(policy)
        macos["macos_execution_status"] = "pass"
        for changed in (waiver, ready, macos):
            self.assertTrue(validate_policy(changed))

    def test_report_rejects_product_execution_claim(self) -> None:
        report = build_report()
        claimed = copy.deepcopy(report)
        claimed["product_boundary_execution_claim"] = "pass"
        self.assertTrue(validate_report(claimed))


if __name__ == "__main__":
    unittest.main()

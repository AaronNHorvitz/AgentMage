from __future__ import annotations

import copy
import unittest

from scripts.fallback_admission import (
    ARTIFACT_RECORD,
    EVALUATION_PLAN,
    SOURCE_RECORD,
    read_json,
    validate_artifact,
    validate_plan,
    validate_source,
)


class FallbackAdmissionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.source = read_json(SOURCE_RECORD)
        self.artifact = read_json(ARTIFACT_RECORD)
        self.plan = read_json(EVALUATION_PLAN)

    def test_committed_fallback_records_are_valid_blocked_and_disabled(self) -> None:
        self.assertEqual(validate_source(self.source), [])
        self.assertEqual(validate_artifact(self.artifact), [])
        self.assertEqual(validate_plan(self.plan, self.source, self.artifact), [])
        self.assertEqual(self.source["decision"]["status"], "BLOCKED")
        self.assertEqual(self.artifact["decision"]["status"], "BLOCKED")
        self.assertEqual(self.plan["candidate_state"], "EVALUATION_REQUIRED_DISABLED")

    def test_source_identity_license_origin_and_policy_drift_are_rejected(self) -> None:
        mutations = {
            "source identity": lambda value: value["identity"].update(
                {"upstream_revision": "latest"}
            ),
            "supplier": lambda value: value["ownership_and_origin"].update(
                {"control_disposition": "unknown"}
            ),
            "license": lambda value: value["license"].update({"spdx": "unknown"}),
            "policy": lambda value: value["policy"].update({"sha256": "0" * 64}),
        }
        for expected, mutate in mutations.items():
            with self.subTest(expected=expected):
                changed = copy.deepcopy(self.source)
                mutate(changed)
                failures = validate_source(changed)
                self.assertTrue(failures)

    def test_selected_gguf_and_docker_layers_cannot_diverge(self) -> None:
        changed = copy.deepcopy(self.artifact)
        changed["docker_model"]["model_layer_digest"] = "sha256:" + ("0" * 64)
        failures = validate_artifact(changed)
        self.assertTrue(any("differs from selected GGUF" in item for item in failures))

    def test_official_qat_artifact_cannot_be_misrepresented_as_selected(self) -> None:
        changed = copy.deepcopy(self.artifact)
        changed["official_qat_gguf"]["selected_for_cross_adapter_evaluation"] = True
        failures = validate_artifact(changed)
        self.assertTrue(any("QAT GGUF distinction" in item for item in failures))

    def test_mutable_or_substituted_runtime_identities_are_rejected(self) -> None:
        changed = copy.deepcopy(self.artifact)
        changed["docker_model"]["digest"] = "latest"
        changed["native_runtime"]["source_commit"] = "main"
        failures = validate_artifact(changed)
        self.assertTrue(any("not immutable" in item for item in failures))
        self.assertTrue(any("runtime identity changed" in item for item in failures))

    def test_evaluation_plan_cannot_change_thresholds_or_enable_fallback(self) -> None:
        changed = copy.deepcopy(self.plan)
        changed["corpus"]["threshold_change_authorized"] = True
        changed["state"]["automatic_switch"] = True
        changed["state"]["activation_authorized"] = True
        failures = validate_plan(changed, self.source, self.artifact)
        self.assertTrue(any("fixed corpus" in item for item in failures))
        self.assertTrue(any("overstates completion or authority" in item for item in failures))

    def test_rejected_e4b_trigger_and_admission_hashes_are_required(self) -> None:
        changed = copy.deepcopy(self.plan)
        changed["trigger_disposition"]["status"] = "PASS"
        changed["admission"]["artifact_sha256"] = "0" * 64
        failures = validate_plan(changed, self.source, self.artifact)
        self.assertTrue(any("rejected E4B" in item for item in failures))
        self.assertTrue(any("admission binding" in item for item in failures))


if __name__ == "__main__":
    unittest.main()

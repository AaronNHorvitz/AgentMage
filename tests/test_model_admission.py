from __future__ import annotations

import copy
import unittest

from scripts.model_admission import (
    DEFAULT_ARTIFACT_RECORD,
    load_record,
    validate_artifact_record,
    validate_record,
)


class ModelAdmissionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.record = load_record()
        self.artifact_record = load_record(DEFAULT_ARTIFACT_RECORD)

    def test_committed_source_admission_is_valid_and_blocked(self) -> None:
        self.assertEqual(validate_record(self.record), [])
        self.assertEqual(self.record["decision"]["status"], "BLOCKED")
        self.assertFalse(
            self.record["ownership_and_origin"]["lineage_complete_under_policy"]
        )

    def test_missing_unknown_and_mutable_identity_fields_are_rejected(self) -> None:
        missing = copy.deepcopy(self.record)
        del missing["tokenizer"]
        self.assertTrue(any("missing top-level" in item for item in validate_record(missing)))

        unknown = copy.deepcopy(self.record)
        unknown["silent_override"] = True
        self.assertTrue(any("unknown top-level" in item for item in validate_record(unknown)))

        mutable = copy.deepcopy(self.record)
        mutable["identity"]["upstream_revision"] = "latest"
        self.assertTrue(any("immutable revision" in item for item in validate_record(mutable)))

    def test_license_origin_tokenizer_and_context_drift_are_rejected(self) -> None:
        mutations = {
            "Apache-2.0": lambda value: value["license"].update({"spdx": "unknown"}),
            "supplier control": lambda value: value["ownership_and_origin"].update(
                {"control_disposition": "unknown"}
            ),
            "tokenizer": lambda value: value["tokenizer"].update(
                {"tokenizer_json_sha256": "0" * 64}
            ),
            "context": lambda value: value["context_contract"].update(
                {"declared_tokens": 262144}
            ),
        }
        for expected, mutate in mutations.items():
            with self.subTest(field=expected):
                changed = copy.deepcopy(self.record)
                mutate(changed)
                failures = validate_record(changed)
                self.assertTrue(any(expected in item for item in failures), failures)

    def test_incomplete_lineage_cannot_be_changed_to_pass_or_activation(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["decision"]["status"] = "PASS"
        changed["decision"]["prohibited_actions"].remove("AgentMage profile activation")

        failures = validate_record(changed)

        self.assertTrue(any("BLOCKED decision" in item for item in failures))
        self.assertTrue(any("prohibit activation" in item for item in failures))

    def test_committed_artifact_admission_is_exact_and_blocked(self) -> None:
        self.assertEqual(validate_artifact_record(self.artifact_record), [])
        self.assertEqual(self.artifact_record["decision"]["status"], "BLOCKED")
        self.assertEqual(
            self.artifact_record["native_runtime"]["local_state"],
            "staged_and_hash_verified",
        )
        self.assertTrue(self.artifact_record["gguf_identity"]["downloaded"])
        self.assertTrue(
            self.artifact_record["gguf_identity"]["multimodal_projector"]["downloaded"]
        )
        self.assertFalse(self.artifact_record["evaluation_host"]["docker_available"])
        self.assertTrue(self.artifact_record["evaluation_host"]["podman_available"])
        self.assertTrue(
            self.artifact_record["evaluation_host"]["dmr_compatibility_execution_complete"]
        )
        self.assertEqual(
            self.artifact_record["docker_engine"]["local_state"],
            "exact_image_staged_and_executed_via_rootless_podman",
        )

    def test_gguf_runtime_and_oci_substitutions_are_rejected(self) -> None:
        mutations = {
            "GGUF identity": lambda value: value["gguf_identity"].update(
                {"sha256": "1" * 64}
            ),
            "native runtime identity": lambda value: value["native_runtime"].update(
                {"source_commit": "2" * 40}
            ),
            "Docker engine digest substitution": lambda value: value["docker_engine"].update(
                {"digest": "sha256:" + ("3" * 64)}
            ),
            "Docker model digest substitution": lambda value: value["docker_model"].update(
                {"digest": "sha256:" + ("4" * 64)}
            ),
        }
        for expected, mutate in mutations.items():
            with self.subTest(identity=expected):
                changed = copy.deepcopy(self.artifact_record)
                mutate(changed)
                failures = validate_artifact_record(changed)
                self.assertTrue(any(expected in item for item in failures), failures)

    def test_dmr_compatibility_evidence_cannot_be_overstated_or_removed(self) -> None:
        changed = copy.deepcopy(self.artifact_record)
        changed["evaluation_host"]["docker_available"] = True
        changed["evaluation_host"]["dmr_compatibility_execution_complete"] = False
        changed["docker_engine"]["compatibility_container_engine"] = "Docker Engine"
        changed["decision"]["blockers"][2]["code"] = "DOCKER-RUNTIME-UNAVAILABLE"

        failures = validate_artifact_record(changed)

        self.assertTrue(any("Docker unavailability" in item for item in failures))
        self.assertTrue(any("Podman compatibility execution" in item for item in failures))
        self.assertTrue(any("compatibility engine" in item for item in failures))
        self.assertTrue(any("blockers are incomplete" in item for item in failures))


if __name__ == "__main__":
    unittest.main()

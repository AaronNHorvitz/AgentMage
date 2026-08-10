from __future__ import annotations

import copy
import unittest

from scripts.model_disposition import build_record, validate_record


class ModelDispositionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.record = build_record("HEAD")

    def test_generated_rejection_reconciles_with_raw_evidence(self) -> None:
        self.assertEqual(validate_record(copy.deepcopy(self.record)), [])
        self.assertEqual(self.record["decision"]["status"], "REJECTED")
        self.assertEqual(
            self.record["cross_adapter"]["shared_failed_cases"],
            ["CITE-001", "TOOL-001"],
        )
        self.assertEqual(
            self.record["cross_adapter"]["shared_failed_thresholds"],
            ["citation_recall", "tool_call_valid_rate"],
        )

    def test_mandatory_failure_cannot_be_downgraded_to_pass_or_blocked(self) -> None:
        for status in ("PASS", "BLOCKED"):
            with self.subTest(status=status):
                changed = copy.deepcopy(self.record)
                changed["decision"]["status"] = status
                failures = validate_record(changed, check_revision=False)
                self.assertTrue(any("must produce REJECTED" in item for item in failures))

    def test_threshold_or_raw_score_mutation_is_rejected(self) -> None:
        changed_threshold = copy.deepcopy(self.record)
        changed_threshold["corpus"]["thresholds"]["citation_recall"] = 0.5
        failures = validate_record(changed_threshold, check_revision=False)
        self.assertTrue(any("thresholds differ" in item for item in failures))

        changed_score = copy.deepcopy(self.record)
        changed_score["adapters"]["native_linux"]["metrics"]["citation_recall"] = 1.0
        failures = validate_record(changed_score, check_revision=False)
        self.assertTrue(any("metrics does not match" in item for item in failures))

    def test_unavailable_mac_cannot_be_represented_by_linux_evidence(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["adapters"]["macos_native"]["linux_evidence_substituted"] = True
        failures = validate_record(changed, check_revision=False)
        self.assertTrue(any("non-substitution changed" in item for item in failures))

    def test_podman_compatibility_cannot_be_overstated_as_docker_engine(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["adapters"]["dmr_linux"]["deployment"][
            "docker_engine_directly_tested"
        ] = True
        failures = validate_record(changed, check_revision=False)
        self.assertTrue(any("overstates direct Docker Engine" in item for item in failures))

    def test_fallback_remains_disabled_and_unapproved(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["fallback"]["automatic_switch"] = True
        changed["fallback"]["activation_authorized"] = True
        failures = validate_record(changed, check_revision=False)
        self.assertTrue(any("disabled pending evaluation" in item for item in failures))

    def test_policy_and_admission_hash_substitutions_are_rejected(self) -> None:
        for path in (
            ("policy",),
            ("admission", "source"),
            ("admission", "artifact"),
        ):
            with self.subTest(path=path):
                changed = copy.deepcopy(self.record)
                target = changed
                for part in path:
                    target = target[part]
                target["sha256"] = "0" * 64
                failures = validate_record(changed, check_revision=False)
                self.assertTrue(any("hash binding changed" in item for item in failures))


if __name__ == "__main__":
    unittest.main()

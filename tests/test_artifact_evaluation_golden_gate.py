from __future__ import annotations

import copy
import json
import unittest

from scripts import artifact_evaluation_golden_gate as gate


def reseal(value: dict[str, object]) -> dict[str, object]:
    value["manifest_sha256"] = gate.ZERO_SHA256
    return gate.sealed(value, "manifest_sha256")


class ArtifactEvaluationGoldenGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = json.loads(gate.OUTPUT_PATH.read_text(encoding="utf-8"))

    def test_manifest_and_metric_versions_are_exact(self) -> None:
        self.assertEqual(self.manifest["golden_manifest_version"], "1.0.0")
        self.assertEqual(self.manifest["metric_manifest_versions"], {"artifact": "1.0.0", "workflow": "1.0.0"})

    def test_every_input_and_outcome_is_visible(self) -> None:
        self.assertEqual(len(self.manifest["required_inputs"]), 8)
        self.assertEqual(self.manifest["outcome_count"], 77)
        self.assertEqual(self.manifest["success_terminal_count"], 17)
        self.assertEqual(self.manifest["non_success_terminal_count"], 60)

    def test_missing_input_blocks_even_when_resealed(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["required_inputs"].pop()
        self.assertTrue(gate.validate_manifest(reseal(changed)))

    def test_hidden_non_pass_state_blocks_even_when_resealed(self) -> None:
        changed = copy.deepcopy(self.manifest)
        index = next(index for index, item in enumerate(changed["outcomes"]) if not item["success_terminal"])
        changed["outcomes"].pop(index)
        changed["outcome_count"] -= 1
        changed["non_success_terminal_count"] -= 1
        self.assertTrue(gate.validate_manifest(reseal(changed)))

    def test_false_pass_blocks_even_when_resealed(self) -> None:
        changed = copy.deepcopy(self.manifest)
        item = next(item for item in changed["outcomes"] if not item["success_terminal"])
        item["success_terminal"] = True
        changed["success_terminal_count"] += 1
        changed["non_success_terminal_count"] -= 1
        self.assertTrue(gate.validate_manifest(reseal(changed)))

    def test_raw_secret_field_blocks_even_when_resealed(self) -> None:
        changed = copy.deepcopy(self.manifest)
        forbidden_key = "raw" + "_secret"
        changed["mutation_probe"] = {forbidden_key: "synthetic-redacted-probe"}
        failures = gate.validate_manifest(reseal(changed))
        self.assertTrue(any("raw-secret" in failure for failure in failures))

    def test_changed_expected_side_effect_blocks_even_when_resealed(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["expected_side_effects"]["network_calls"] = 1
        failures = gate.validate_manifest(reseal(changed))
        self.assertTrue(any("side effects changed" in failure for failure in failures))

    def test_checked_manifest_matches_reproducible_builder(self) -> None:
        self.assertTrue(gate.valid_hash(self.manifest, "manifest_sha256"))
        self.assertEqual(gate.raw_key_paths(self.manifest), [])
        self.assertEqual(gate.check(), [])


if __name__ == "__main__":
    unittest.main()

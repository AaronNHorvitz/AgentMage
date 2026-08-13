"""Tests for the hostile network-surface injection gate."""

from __future__ import annotations

import copy
import unittest

from scripts import hostile_network_injection_gate as gate


class HostileNetworkInjectionGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = gate.load_fixture()

    def test_exact_fixture_blocks_every_case_before_execution(self) -> None:
        result = gate.run_gate(self.fixture)
        self.assertEqual(gate.validate_result(result), [])
        self.assertFalse(result["injected_code_executed"])
        self.assertFalse(result["external_network_used"])

    def test_missing_case_or_changed_order_fails_closed(self) -> None:
        missing = copy.deepcopy(self.fixture)
        missing["cases"].pop()
        self.assertEqual(
            gate.validate_fixture(missing),
            ["hostile fixture identity closure changed"],
        )
        reordered = copy.deepcopy(self.fixture)
        reordered["cases"][0], reordered["cases"][1] = (
            reordered["cases"][1],
            reordered["cases"][0],
        )
        self.assertEqual(
            gate.validate_fixture(reordered),
            ["hostile fixture identity closure changed"],
        )

    def test_unknown_fields_or_empty_detector_set_fail_closed(self) -> None:
        unknown = copy.deepcopy(self.fixture)
        unknown["cases"][0]["execute"] = True
        self.assertIn(
            "hostile fixture case fields changed: telemetry-upload",
            gate.validate_fixture(unknown),
        )
        empty = copy.deepcopy(self.fixture)
        empty["cases"][0]["expected_failures"] = []
        self.assertIn(
            "hostile fixture detector closure changed: telemetry-upload",
            gate.validate_fixture(empty),
        )

    def test_changed_mutation_must_not_silently_pass(self) -> None:
        changed = copy.deepcopy(self.fixture)
        changed["cases"][0]["content"] = "const localOnly = true;"
        with self.assertRaisesRegex(
            gate.HostileInjectionGateError,
            "hostile mutation detector mismatch: telemetry-upload",
        ):
            gate.run_gate(changed)

    def test_result_claim_mutations_fail_closed(self) -> None:
        result = gate.run_gate(self.fixture)
        for key in ("injected_code_executed", "external_network_used"):
            changed = copy.deepcopy(result)
            changed[key] = True
            self.assertEqual(
                gate.validate_result(changed), ["hostile gate result changed"]
            )
        changed = copy.deepcopy(result)
        changed["results"][0]["status"] = "executed"
        self.assertEqual(gate.validate_result(changed), ["hostile gate result changed"])


if __name__ == "__main__":
    unittest.main()

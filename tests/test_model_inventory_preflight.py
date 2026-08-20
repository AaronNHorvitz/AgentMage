from __future__ import annotations

import copy
import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "model_inventory_preflight", ROOT / "scripts/model_inventory_preflight.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ModelInventoryPreflightTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.envelopes = MODULE.read_json(MODULE.ENVELOPES)
        cls.inventory = MODULE.read_json(MODULE.INVENTORY)
        cls.report = MODULE.build()

    def test_full_cartesian_preflight_reconciles_and_is_non_acquiring(self) -> None:
        report = self.report
        self.assertEqual(report["entry_count"], len(self.inventory["entries"]))
        self.assertEqual(report["envelope_count"], len(self.envelopes["envelopes"]))
        self.assertEqual(
            report["decision_count"], report["entry_count"] * report["envelope_count"]
        )
        self.assertEqual(MODULE.validate(report), [])
        for item in report["entries"]:
            self.assertEqual(
                {row["envelope_id"] for row in item["per_envelope"]},
                {env["envelope_id"] for env in self.envelopes["envelopes"]},
            )
            for row in item["per_envelope"]:
                self.assertFalse(row["acquisition_started"])
                self.assertFalse(row["runtime_started"])
                self.assertFalse(row["network_egress_opened"])
                self.assertFalse(row["role_borrowed_from_family"])
                self.assertFalse(row["result_borrowed_from_sibling"])
                self.assertEqual(set(row["axes"]), set(MODULE.AXES))

    def test_specialist_ineligible_and_role_isolation_preserved(self) -> None:
        report = self.report
        by_id = {item["entry_id"]: item for item in report["entries"]}
        for source in self.inventory["entries"]:
            preflight = by_id[source["entry_id"]]
            if source["disposition"] == "INELIGIBLE":
                for row in preflight["per_envelope"]:
                    self.assertEqual(row["preflight_status"], "INELIGIBLE")
                    self.assertIn(
                        "source-ineligible-remains-ineligible-per-envelope",
                        row["reason_codes"],
                    )
            if "coding_planner" in source.get("prohibited_roles", []):
                self.assertNotIn("coding_planner", preflight["roles"])

    def test_axis_mismatches_produce_blocked_hardware_not_omission(self) -> None:
        entry = next(
            copy.deepcopy(item)
            for item in self.inventory["entries"]
            if item["disposition"] == "BLOCKED" and item.get("artifact_formats")
        )
        envelope = copy.deepcopy(self.envelopes["envelopes"][0])
        entry["artifact_formats"] = ["ExecuTorch-PTE"]
        entry["modalities"] = ["holographic-projection"]
        envelope["supported_artifact_formats"] = ["GGUF", "safetensors"]
        envelope["modality_support"] = ["text-generation"]
        axes = MODULE._preflight_axes(entry, envelope)
        self.assertEqual(axes["format"]["status"], "BLOCKED-HARDWARE")
        self.assertEqual(axes["runtime"]["status"], "BLOCKED-HARDWARE")
        self.assertEqual(axes["modality"]["status"], "BLOCKED-HARDWARE")
        status, reasons = MODULE._entry_status(entry, axes)
        self.assertEqual(status, "BLOCKED-HARDWARE")
        self.assertIn("format-blocked-hardware", reasons)
        self.assertIn("modality-blocked-hardware", reasons)
        self.assertIn("runtime-blocked-hardware", reasons)

    def test_family_borrow_and_activation_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["entries"][0]["per_envelope"][0].update(
                {"acquisition_started": True}
            ),
            lambda value: value["entries"][0]["per_envelope"][0].update(
                {"runtime_started": True}
            ),
            lambda value: value["entries"][0]["per_envelope"][0].update(
                {"network_egress_opened": True}
            ),
            lambda value: value["entries"][0]["per_envelope"][0].update(
                {"result_borrowed_from_sibling": True}
            ),
            lambda value: value["entries"][0]["per_envelope"][0].update(
                {"role_borrowed_from_family": True}
            ),
            lambda value: value["product_state"].update({"acquisitions_started": 1}),
            lambda value: value["entries"][0]["per_envelope"].pop(),
            lambda value: value["entries"].pop(),
        )
        for mutate in mutations:
            report = copy.deepcopy(self.report)
            mutate(report)
            self.assertTrue(MODULE.validate(report))

    def test_family_wide_conclusion_is_prevented_by_per_entry_reasoning(self) -> None:
        report = self.report
        siblings: dict[str, list[dict]] = {}
        for item in report["entries"]:
            family = item["repository"].split("/", 1)[0]
            siblings.setdefault(family, []).append(item)
        self.assertGreater(len(siblings), 0)
        for family, items in siblings.items():
            entry_ids = {item["entry_id"] for item in items}
            self.assertEqual(len(entry_ids), len(items))

    def test_envelopes_declaration_publishes_every_required_axis(self) -> None:
        required = {
            "envelope_id",
            "cpu",
            "os",
            "accelerator",
            "memory_bytes",
            "storage_bytes",
            "context_token_ceiling",
            "modality_support",
            "supported_artifact_formats",
            "supported_runtimes",
            "expected_working_set_ceiling_bytes",
        }
        for envelope in self.envelopes["envelopes"]:
            missing = required - set(envelope)
            self.assertFalse(missing, f"envelope missing axes: {missing}")
        for rule in (
            "non_acquisition_rule",
            "role_isolation_rule",
            "hardware_admission_rule",
        ):
            self.assertIn(rule, self.envelopes)


if __name__ == "__main__":
    unittest.main()

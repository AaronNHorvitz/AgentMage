from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.fuzz_target_registry import (
    BOUNDARIES,
    REGISTRY_PATH,
    REPORT_PATH,
    SCENARIOS,
    build_registry,
    build_report,
    check_artifacts,
    discover_ffi_boundaries,
    read_json,
    validate_registry,
    validate_report,
)


class FuzzTargetRegistryTests(unittest.TestCase):
    def test_checked_registry_and_report_are_current(self) -> None:
        self.assertEqual(check_artifacts(), [])
        self.assertEqual(read_json(REGISTRY_PATH), build_registry())
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_all_required_boundaries_have_one_target_and_all_seed_scenarios(self) -> None:
        registry = build_registry()
        targets = registry["targets"]
        self.assertEqual(
            [(item["target_id"], item["boundary_class"]) for item in targets],
            [(item[0], item[1]) for item in BOUNDARIES],
        )
        self.assertTrue(
            all(
                item["seed_selection"]["required_scenarios"] == list(SCENARIOS)
                for item in targets
            )
        )

    def test_every_target_is_bounded_inert_and_fail_closed(self) -> None:
        for target in build_registry()["targets"]:
            self.assertFalse(target["product_boundary_implemented"])
            self.assertEqual(target["product_support_claim"], "none")
            self.assertLessEqual(
                target["input_contract"]["maximum_materialized_input_bytes"], 4096
            )
            self.assertTrue(
                all(
                    value is False
                    for field, value in target["execution_contract"].items()
                    if field.endswith("_permitted")
                )
            )
            self.assertTrue(all(target["outcome_contract"].values()))

    def test_current_tree_records_the_reviewed_ffi_boundary(self) -> None:
        self.assertEqual(
            discover_ffi_boundaries(),
            [
                {
                    "path": "platforms/windows/src/native_identity.rs",
                    "reason": "rust-ffi-token",
                }
            ],
        )
        discovery = build_registry()["ffi_discovery"]
        self.assertEqual(discovery["active_boundary_count"], 1)
        self.assertEqual(discovery["unregistered_boundary_count"], 0)

    def test_new_unregistered_ffi_boundary_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "kernel/contracts/src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text('unsafe extern "C" fn boundary() {}\n', encoding="utf-8")
            self.assertEqual(
                discover_ffi_boundaries(root),
                [{"path": "kernel/contracts/src/lib.rs", "reason": "rust-ffi-token"}],
            )

    def test_missing_target_unsafe_execution_or_product_claim_fails_closed(self) -> None:
        registry = build_registry()
        missing = copy.deepcopy(registry)
        missing["targets"].pop()
        unsafe = copy.deepcopy(registry)
        unsafe["targets"][0]["execution_contract"]["network_permitted"] = True
        claimed = copy.deepcopy(registry)
        claimed["targets"][0]["product_boundary_implemented"] = True
        self.assertTrue(validate_registry(missing))
        self.assertTrue(validate_registry(unsafe))
        self.assertTrue(validate_registry(claimed))

    def test_report_rejects_coverage_and_macos_overclaims(self) -> None:
        report = build_report()
        incomplete = copy.deepcopy(report)
        incomplete["coverage"]["complete"] = False
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        self.assertTrue(validate_report(incomplete))
        self.assertTrue(validate_report(macos))


if __name__ == "__main__":
    unittest.main()

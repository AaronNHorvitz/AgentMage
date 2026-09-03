from __future__ import annotations

import copy
import unittest

from scripts.contract_boundary_gate import (
    CHECKS,
    expected_report,
    validate_capability_absence,
    validate_current_boundary,
    validate_report,
)
from scripts.dependency_dispositions import load_record as load_dispositions
from scripts.parser_ocr_placement import load_record as load_placement
from scripts.runtime_ownership import load_ownership
from scripts.status_model import load_status_model


class ContractBoundaryGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.status = load_status_model()
        self.ownership = load_ownership()
        self.dispositions = load_dispositions()
        self.placement = load_placement()

    def validate(
        self,
        *,
        status: dict | None = None,
        ownership: dict | None = None,
        dispositions: dict | None = None,
        placement: dict | None = None,
    ) -> list[str]:
        return validate_capability_absence(
            self.status if status is None else status,
            self.ownership if ownership is None else ownership,
            self.dispositions if dispositions is None else dispositions,
            self.placement if placement is None else placement,
        )

    def test_current_boundary_and_capability_absence_pass(self) -> None:
        self.assertEqual(validate_current_boundary(), [])
        self.assertEqual(self.validate(), [])

    def test_mcp_implementation_and_module_are_rejected(self) -> None:
        for key, value in (("implemented", True), ("modules", ["mcp-adapter"])):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.ownership)
                mcp = next(item for item in changed["layers"] if item["id"] == "mcp")
                mcp[key] = value
                self.assertIn(
                    "capability-absence: MCP must remain unimplemented with no modules",
                    self.validate(ownership=changed),
                )

    def test_parser_feature_activation_is_rejected(self) -> None:
        changed = copy.deepcopy(self.placement)
        changed["package_features"]["default_enabled"] = ["parser-pdf"]
        self.assertIn(
            "capability-absence: parser/OCR package features were promoted",
            self.validate(placement=changed),
        )

    def test_ocr_promotion_is_rejected(self) -> None:
        changed = copy.deepcopy(self.placement)
        changed["package_features"]["ocr_status"] = "enabled"
        self.assertIn(
            "capability-absence: parser/OCR package features were promoted",
            self.validate(placement=changed),
        )

    def test_every_placement_implementation_claim_is_rejected(self) -> None:
        for key, value in self.placement["product_truth"].items():
            if key == "placement_decided":
                continue
            with self.subTest(key=key):
                changed = copy.deepcopy(self.placement)
                changed["product_truth"][key] = True
                self.assertIn(
                    f"capability-absence: placement product truth widened: {key}",
                    self.validate(placement=changed),
                )

    def test_every_dependency_capability_claim_is_rejected(self) -> None:
        for key in self.dispositions["product_truth"]:
            with self.subTest(key=key):
                changed = copy.deepcopy(self.dispositions)
                changed["product_truth"][key] = True
                self.assertIn(
                    f"capability-absence: dependency product truth widened: {key}",
                    self.validate(dispositions=changed),
                )

    def test_current_product_enabled_model_is_rejected(self) -> None:
        changed = copy.deepcopy(self.status)
        changed["current_product"]["enabled_models"] = ["candidate"]
        self.assertIn(
            "current-versus-planned-truth: current product enabled_models differs from bound truth",
            self.validate(status=changed),
        )

    def test_current_product_platform_or_package_claim_is_rejected(self) -> None:
        for key, value in (
            ("supported_platforms", ["fedora-x86_64"]),
            ("released_packages", ["agentmage-1.0.0"]),
        ):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.status)
                changed["current_product"][key] = value
                self.assertIn(
                    f"current-versus-planned-truth: current product {key} differs from bound truth",
                    self.validate(status=changed),
                )

    def test_current_product_workflow_regression_or_gate_claim_is_rejected(self) -> None:
        for key, value in (
            ("integrated_user_workflow", False),
            ("release_gate_status", "pass"),
        ):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.status)
                changed["current_product"][key] = value
                self.assertIn(
                    f"current-versus-planned-truth: current product {key} differs from bound truth",
                    self.validate(status=changed),
                )

    def test_current_product_workflow_must_retain_bound_evidence(self) -> None:
        changed = copy.deepcopy(self.status)
        changed["current_product"]["integrated_workflow"]["evidence_path"] = "unbound.json"
        self.assertIn(
            "current-versus-planned-truth: current product integrated_workflow differs from bound truth",
            self.validate(status=changed),
        )

    def test_gate_covers_the_exact_six_required_check_classes(self) -> None:
        self.assertEqual(
            [item["id"] for item in CHECKS],
            [
                "schema-mutation",
                "dependency-direction",
                "forbidden-duplicate-owner",
                "api-surface",
                "capability-absence",
                "current-versus-planned-truth",
            ],
        )

    def test_report_refuses_result_or_source_mutation(self) -> None:
        result = expected_report()
        result["checks"][0]["result"] = "not-run"
        self.assertTrue(validate_report(result))
        source = expected_report()
        source["sources"][0]["sha256"] = "0" * 64
        self.assertTrue(validate_report(source))

    def test_report_refuses_product_or_external_evidence_overclaim(self) -> None:
        for key in (
            "capability_enabled",
            "native_platform_evidence",
            "external_review_complete",
            "platform_support_claimed",
            "release_readiness",
        ):
            with self.subTest(key=key):
                changed = expected_report()
                changed["product_truth"][key] = True
                self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()

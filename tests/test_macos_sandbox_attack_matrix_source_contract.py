from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.macos_sandbox_attack_matrix_source_contract import (
    ROOT,
    SOURCE_PATHS,
    UPSTREAM_REPORT_SHA256,
    build_report,
    validate_sources,
)


class MacOSSandboxAttackMatrixSourceContractTests(unittest.TestCase):
    """Five closed tests for the S-008-ST01 source-only matrix contract."""

    def copy_inputs(self, destination: Path) -> None:
        for relative in SOURCE_PATHS:
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / relative, target)

    def mutate(self, relative: str, old: str, new: str) -> list[str]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / relative
            text = path.read_text(encoding="utf-8")
            self.assertIn(old, text)
            path.write_text(text.replace(old, new, 1), encoding="utf-8")
            return validate_sources(root)

    def test_source_contract_is_closed_and_blocked(self) -> None:
        self.assertEqual(validate_sources(), [])
        report = build_report()
        self.assertEqual(report["status"], "prepared-source-only-blocked-macos")
        self.assertEqual(report["contract"]["component_count"], 4)
        self.assertEqual(report["contract"]["attack_class_count"], 9)
        self.assertEqual(report["contract"]["attack_case_count"], 36)
        self.assertEqual(report["contract"]["accepted_unauthorized_case_count"], 0)
        self.assertTrue(all(value is False for value in report["execution"].values()))
        self.assertTrue(all(value is False for value in report["claims"].values()))
        self.assertEqual(len(report["remaining_blockers"]), 5)
        self.assertEqual(len(SOURCE_PATHS), 8)

    def test_exact_component_attack_and_zero_guard_closure_is_required(self) -> None:
        swift = (
            "platforms/macos/Tests/AgentMageMacOSPlatformTests/"
            "MacOSSandboxAttackMatrixTests.swift"
        )
        failures = self.mutate(swift, 'case kernelHost = "kernel_host"', "case kernelHost")
        self.assertTrue(any("component" in item for item in failures))
        failures = self.mutate(swift, 'case ambientHome = "ambient-home"', "case ambientHome")
        self.assertTrue(any("attack class" in item for item in failures))
        failures = self.mutate(
            swift,
            "guard result.unauthorizedAccessCount == 0 else",
            "guard result.unauthorizedAccessCount <= 1 else",
        )
        self.assertTrue(any("zero-result guard" in item for item in failures))

    def test_matrix_order_identity_and_mutation_closure_is_required(self) -> None:
        swift = (
            "platforms/macos/Tests/AgentMageMacOSPlatformTests/"
            "MacOSSandboxAttackMatrixTests.swift"
        )
        for old in (
            "SandboxAttackComponent.allCases.flatMap",
            "result.caseIdentifier == attack.identifier",
            "UnsafeAttackMutation.allCases.count == 11",
        ):
            failures = self.mutate(swift, old, "removed-campaign-term")
            self.assertTrue(any("campaign missing" in item for item in failures))

    def test_upstream_evidence_contract_drift_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.copy_inputs(root)
            path = root / (
                "artifacts/sprints/sprint-8/story-8.1/"
                "macos-sandbox-attack-results-source-contract.json"
            )
            value = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(
                value["contract"]["attack_case_count"],
                36,
            )
            value["contract"]["attack_case_count"] = 35
            path.write_text(json.dumps(value), encoding="utf-8")
            failures = validate_sources(root)
            self.assertTrue(any("digest changed" in item for item in failures))
            self.assertTrue(any("matrix closure changed" in item for item in failures))
            self.assertEqual(len(UPSTREAM_REPORT_SHA256), 64)

    def test_evidence_verifier_and_nonpromotion_procedure_are_required(self) -> None:
        failures = self.mutate(
            "scripts/macos_sandbox_attack_results.py",
            '"native_operations_executed_by_ingestor": False',
            '"native_operations_executed_by_ingestor": True',
        )
        self.assertTrue(any("evidence verifier" in item for item in failures))
        failures = self.mutate(
            "packaging/macos/SANDBOX-ATTACK-MATRIX.md",
            "No native hostile-access campaign",
            "Native hostile-access campaign",
        )
        self.assertTrue(any("procedure missing" in item for item in failures))


if __name__ == "__main__":
    unittest.main()

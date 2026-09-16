from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from scripts.status_model import (
    ROOT,
    load_json,
    load_status_model,
    transition_is_legal,
    validate_status_model,
    _validate_demo_native_evidence,
    DEMO_ACCEPTANCE_REPORTS,
    DEMO_BROWSER_CASES,
)


class StatusModelTests(unittest.TestCase):
    def setUp(self) -> None:
        self.model = load_status_model()
        self.matrix = load_json(ROOT / "architecture" / "language-build-matrix.json")
        contract = self.model["documentation_contract"]
        self.documents = {
            path: (ROOT / path).read_text(encoding="utf-8")
            for path in contract["documents"]
        }

    def validate(self, model: dict, matrix: dict | None = None) -> list[str]:
        return validate_status_model(
            model,
            matrix=self.matrix if matrix is None else matrix,
            documents=self.documents,
        )

    def test_canonical_status_model_passes(self) -> None:
        self.assertEqual(self.validate(self.model), [])

    def test_demo_scope_cannot_promote_production_or_enable_compaction(self) -> None:
        mutated = copy.deepcopy(self.model)
        demo = mutated["demo_milestones"][0]
        demo["automatic_compaction_enabled"] = True
        demo["production_qualification"] = True
        failures = self.validate(mutated)
        self.assertTrue(any("automatic_compaction_enabled" in item for item in failures))
        self.assertTrue(any("production_qualification" in item for item in failures))

    def test_demo_selected_model_binding_cannot_drift(self) -> None:
        mutated = copy.deepcopy(self.model)
        mutated["demo_milestones"][0]["model_sha256"] = "0" * 64
        self.assertTrue(any("selected-model binding" in item for item in self.validate(mutated)))

    def test_demo_native_state_requires_actual_complete_acceptance(self) -> None:
        mutated = copy.deepcopy(self.model)
        demo = mutated["demo_milestones"][0]
        demo["verification_status"] = "native-tested"
        demo["acceptance_status"] = "pending-real-browser-offline-restart"
        self.assertTrue(any("verified-local-demo acceptance" in item for item in self.validate(mutated)))
        with tempfile.TemporaryDirectory() as temporary:
            failures = []
            _validate_demo_native_evidence(Path(temporary), failures)
        self.assertEqual(len(failures), len(DEMO_ACCEPTANCE_REPORTS))

    def test_demo_native_report_guard_rejects_failed_cases_omitted_bindings_and_stale_inputs(self) -> None:
        # Synthetic unit fixtures exercise the validator, never demo acceptance.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            inputs = ["scripts/demo.py", "scripts/demo_smoke.py", "scripts/demo_offline_smoke.py",
                      "scripts/demo_browser_smoke.mjs", "shells/host/src/demo_documents.rs",
                      "demo/model.json", "demo/web/app.js", "supply-chain/sbom.cdx.json",
                      "supply-chain/dependency-provenance.json", "supply-chain/dependency-hashes.sha256"]
            bindings = {}
            for relative in inputs:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("unit fixture", encoding="utf-8")
                bindings[relative] = hashlib.sha256(path.read_bytes()).hexdigest()
            output = root / "docs" / "verification"
            output.mkdir(parents=True)
            for name, record_type in DEMO_ACCEPTANCE_REPORTS.items():
                report = {"record_type": record_type, "passed": True, "source_sha256": bindings,
                          "network": {"external_connect_blocked": True},
                          "real_application_result": {"answer": "unit fact", "citations": ["unit source"]},
                          "acceptance_reports": ["docs/verification/demo-browser-acceptance.json",
                                                 "docs/verification/demo-offline-acceptance.json",
                                                 "docs/verification/demo-restarted-browser-acceptance.json"],
                          "final_running_pid": 123}
                if "browser" in name:
                    report["tests"] = [{"name": case, "passed": True} for case in sorted(DEMO_BROWSER_CASES)]
                    report["real_model_interactions"] = [
                        {"answer": "unit fact", "citations": ["unit source"], "insufficient_evidence": False},
                        {"answer": "unit abstention", "citations": [], "insufficient_evidence": True}]
                (output / name).write_text(json.dumps(report), encoding="utf-8")
            failures = []
            _validate_demo_native_evidence(root, failures)
            self.assertEqual(failures, [])
            path = output / "demo-browser-acceptance.json"
            report = json.loads(path.read_text())
            report["tests"][0]["passed"] = False
            report["source_sha256"].pop("scripts/demo.py")
            path.write_text(json.dumps(report), encoding="utf-8")
            (root / "demo" / "model.json").write_text("changed unit fixture", encoding="utf-8")
            failures = []
            _validate_demo_native_evidence(root, failures)
            self.assertTrue(any("failed or missing cases" in item for item in failures))
            self.assertTrue(any("source binding coverage" in item for item in failures))
            self.assertTrue(any("input is stale" in item for item in failures))

    def test_unknown_status_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.model)
        mutated["components"][0]["lifecycle_status"] = "mostly-done"
        self.assertTrue(self.validate(mutated))

    def test_lifecycle_promotion_cannot_skip_a_state(self) -> None:
        self.assertTrue(
            transition_is_legal(self.model, "lifecycle", "scaffolded", "implemented")
        )
        self.assertFalse(
            transition_is_legal(self.model, "lifecycle", "scaffolded", "integrated")
        )

    def test_required_evidence_kind_cannot_be_removed(self) -> None:
        mutated = copy.deepcopy(self.model)
        kernel = next(item for item in mutated["components"] if item["id"] == "kernel")
        kernel["evidence_basis"] = [
            item for item in kernel["evidence_basis"] if item["kind"] != "source"
        ]
        failures = self.validate(mutated)
        self.assertTrue(any("requires source evidence" in item for item in failures))

    def test_scaffold_cannot_claim_supported_release(self) -> None:
        mutated = copy.deepcopy(self.model)
        mutated["components"][0]["support_status"] = "supported"
        self.assertTrue(
            any("verified shipment" in item for item in self.validate(mutated))
        )

    def test_rejected_model_cannot_be_enabled(self) -> None:
        mutated = copy.deepcopy(self.model)
        mutated["models"][0]["enabled"] = True
        self.assertTrue(any("must remain disabled" in item for item in self.validate(mutated)))

    def test_future_candidate_does_not_require_a_gemma_only_status_set(self) -> None:
        mutated = copy.deepcopy(self.model)
        candidate = copy.deepcopy(mutated["models"][0])
        candidate.update(
            {
                "id": "eligible-candidate",
                "verification_status": "not-run",
                "disposition_status": "blocked",
                "evidence_basis": [
                    {"kind": "source", "path": "MODEL-PROVENANCE-POLICY.md"},
                    {
                        "kind": "blocker",
                        "path": "docs/decisions/0027-muse-first-model-neutral-runtime-and-evaluation.md",
                    },
                    {"kind": "status-assessment", "path": "ENGINEERING-AUDIT-REMEDIATION.md"},
                ],
            }
        )
        mutated["models"].append(candidate)

        self.assertEqual(self.validate(mutated), [])

    def test_historical_gemma_candidate_cannot_be_deleted(self) -> None:
        mutated = copy.deepcopy(self.model)
        mutated["models"] = [
            item for item in mutated["models"] if item["id"] != "gemma-4-e4b-it"
        ]

        self.assertTrue(
            any("preserve evaluated historical candidates" in item for item in self.validate(mutated))
        )

    def test_current_product_cannot_claim_an_enabled_model(self) -> None:
        mutated = copy.deepcopy(self.model)
        mutated["current_product"]["enabled_models"] = ["gemma-4-e4b-it"]
        self.assertTrue(any("enabled_models" in item for item in self.validate(mutated)))

    def test_current_integrated_workflow_is_bound_to_story_22_5(self) -> None:
        current = self.model["current_product"]
        self.assertTrue(current["integrated_user_workflow"])
        self.assertEqual(
            current["integrated_workflow"]["evidence_path"],
            "artifacts/sprints/sprint-22/story-22.5/vertical-slice-report.json",
        )
        mutated = copy.deepcopy(self.model)
        mutated["current_product"]["integrated_user_workflow"] = False
        self.assertTrue(any("integrated_user_workflow" in item for item in self.validate(mutated)))

    def test_windows_cannot_be_promoted_without_later_evidence(self) -> None:
        mutated = copy.deepcopy(self.model)
        windows = next(
            item for item in mutated["platforms"] if item["id"] == "windows-x86_64"
        )
        windows["lifecycle_status"] = "implemented"
        self.assertTrue(any("windows-x86_64" in item for item in self.validate(mutated)))

    def test_scope_counts_cannot_change(self) -> None:
        mutated = copy.deepcopy(self.model)
        mutated["scope_control"]["accepted_sprints"] = 168
        self.assertTrue(any("must equal 169" in item for item in self.validate(mutated)))

    def test_scope_freeze_cannot_be_reactivated_silently(self) -> None:
        mutated = copy.deepcopy(self.model)
        mutated["scope_control"]["new_capability_families_frozen"] = True
        self.assertTrue(
            any("new_capability_families_frozen" in item for item in self.validate(mutated))
        )

    def test_document_status_marker_is_required(self) -> None:
        documents = copy.deepcopy(self.documents)
        documents["README.md"] = documents["README.md"].replace(
            "Current enabled models: none.", "Current enabled models: one."
        )
        failures = validate_status_model(
            self.model, matrix=self.matrix, documents=documents
        )
        self.assertTrue(any("README.md" in item for item in failures))

    def test_matrix_status_reference_cannot_drift(self) -> None:
        mutated = copy.deepcopy(self.matrix)
        mutated["product_components"][0]["status_ref"] = "elsewhere"
        self.assertTrue(
            any(
                "stale status reference" in item
                for item in self.validate(self.model, mutated)
            )
        )

    def test_reference_contract_cannot_change_silently(self) -> None:
        mutated = copy.deepcopy(self.model)
        mutated["reference_contract"] = "unversioned"
        self.assertTrue(
            any("reference contract" in item for item in self.validate(mutated))
        )


if __name__ == "__main__":
    unittest.main()

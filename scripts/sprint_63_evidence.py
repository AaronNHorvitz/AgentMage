#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 63 evidence."""

from __future__ import annotations

from pathlib import Path
from typing import Final

from scripts.sprint_evidence_recorder import SprintEvidenceDefinition, main


ROOT: Final = Path(__file__).resolve().parents[1]
SOURCE_PATHS: Final = (
    "Cargo.toml", "Cargo.lock", "capabilities/knowledge/Cargo.toml",
    "capabilities/knowledge/src/json_data.rs",
    "capabilities/knowledge/src/reconciliation_workbook.rs",
    "capabilities/knowledge/src/spreadsheet_ooxml.rs",
    "capabilities/knowledge/src/tabular.rs", "capabilities/knowledge/src/lib.rs",
    "schemas/runtime/structured-json-document.schema.json",
    "schemas/runtime/structured-json-redaction.schema.json",
    "schemas/runtime/structured-json-comparison.schema.json",
    "schemas/runtime/generated-reconciliation-workbook.schema.json",
    "schemas/runtime/spreadsheet-verification-report.schema.json",
    "scripts/validate_planning_schemas.mjs", "tests/test_planning_schemas.mjs",
    "supply-chain/dependency-hashes.sha256", "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json",
    "docs/architecture/structured-data-method-registry.md",
    "docs/architecture/json-reconciliation-and-workbook-output.md",
    "docs/guides/json-and-reconciliation-review.md",
    "docs/verification/sprint-63-reconciliation-corpus.json",
    "docs/verification/sprint-63-reconciliation-dependency-manifest.json",
    "docs/verification/sprint-63-local-results.md",
    "scripts/reconciliation_contract.py", "tests/test_reconciliation_contract.py",
    "scripts/sprint_evidence_recorder.py", "scripts/sprint_63_evidence.py",
    "tests/test_sprint_63_evidence.py",
)
COMMANDS: Final = (
    ("json-unit", ("cargo", "test", "-p", "agentmage-capability-knowledge", "json_data::tests", "--lib", "--locked")),
    ("reconciliation-unit", ("cargo", "test", "-p", "agentmage-capability-knowledge", "reconciliation_workbook::tests", "--lib", "--locked")),
    ("spreadsheet-reopen-unit", ("cargo", "test", "-p", "agentmage-capability-knowledge", "spreadsheet_ooxml::tests", "--lib", "--locked")),
    ("reconciliation-runtime-schemas", ("node", "--test", "tests/test_planning_schemas.mjs")),
    ("reconciliation-review-corpus", ("python3", "-m", "unittest", "tests.test_reconciliation_contract")),
    ("reconciliation-strict-clippy", ("cargo", "clippy", "-p", "agentmage-capability-knowledge", "--all-targets", "--locked", "--", "-D", "warnings")),
    ("reconciliation-format", ("cargo", "fmt", "--all", "--", "--check")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_63_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:5])
SECURITY_REQUIREMENTS: Final = (
    "SR-DAT-001", "SR-DAT-002", "SR-DAT-003", "SR-SUP-008",
    "SR-TST-002", "SR-TST-004", "SR-TST-006", "SR-CIV-008",
)
IMPLEMENTED: Final = {
    "runtime_schema_count": 5, "json_rust_fixture_count": 5,
    "workbook_rust_fixture_count": 4, "review_corpus_case_count": 109,
    "implemented_method_count": 6, "strict_duplicate_key_json_parsing": True,
    "closed_json_schema_validation": True, "canonical_json_redaction": True,
    "hash_only_json_comparison": True, "deterministic_reconciliation_xlsx": True,
    "generated_xlsx_direct_reopen": True, "closed_formula_policy": True,
    "formula_error_scan": True, "synthetic_native_verifier_semantics": True,
    "network_access_capability": False, "filesystem_mutation_capability": False,
    "formula_execution_capability": False, "native_office_runtime_admitted": False,
    "numeric_tolerance_method": False, "financial_rounding_method": False,
    "many_to_many_allocation_method": False, "stale_data_policy": False,
    "cross_platform_acceptance": False, "accessibility_acceptance": False,
    "independent_review": False, "manual_fuzzing": False,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINT-62-BLOCKED", "owner": "63.1"},
    {"code": "NUMERIC-TOLERANCE-METHOD-UNIMPLEMENTED", "owner": "63.1.3.2"},
    {"code": "FINANCIAL-ROUNDING-METHOD-UNIMPLEMENTED", "owner": "63.1.3.2"},
    {"code": "MANY-TO-MANY-ALLOCATION-UNIMPLEMENTED", "owner": "63.1.3.2"},
    {"code": "STALE-DATA-POLICY-UNIMPLEMENTED", "owner": "63.1.3.2"},
    {"code": "ARBITRARY-PRECISION-DECIMAL-METHOD-UNIMPLEMENTED", "owner": "63.1.3.2"},
    {"code": "NATIVE-OFFICE-RECALCULATION-ABSENT", "owner": "63.1.1.4"},
    {"code": "FEDORA-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "63.1.3.4"},
    {"code": "UBUNTU-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "63.1.3.4"},
    {"code": "WINDOWS-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "63.1.3.4"},
    {"code": "MACOS-NATIVE-VISUAL-EVIDENCE-ABSENT", "owner": "63.1.3.4"},
    {"code": "ACCESSIBILITY-ACCEPTANCE-ABSENT", "owner": "63.1.3.5"},
    {"code": "INDEPENDENT-BOUNDARY-REVIEW-ABSENT", "owner": "63.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "review_corpus_case_count": 109, "accepted_network_effect_count": 0,
    "accepted_execution_effect_count": 0, "accepted_filesystem_effect_count": 0,
    "canonical_json_verified": True, "json_redaction_and_comparison_verified": True,
    "deterministic_xlsx_generation_verified": True, "direct_reopen_verified": True,
    "synthetic_verifier_semantics_verified": True, "native_office_runtime_admitted": False,
    "native_office_platform_count": 0, "advanced_reconciliation_methods_complete": False,
    "cross_platform_acceptance": False, "accessibility_acceptance": False,
    "independent_review": False, "manual_fuzzing": False, "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_63_contract_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprint_62_closed": False, "advanced_reconciliation_methods_complete": False,
    "native_recalculation_complete": False, "native_visual_complete": False,
    "cross_platform_acceptance_passed": False, "accessibility_acceptance_passed": False,
    "independent_review_present": False, "manual_fuzzing_complete": False,
    "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=63, root=ROOT, output="artifacts/sprints/sprint-63/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(FOCUSED_COMMANDS[:3]),
    security_requirement_ids=SECURITY_REQUIREMENTS, implemented_contracts=IMPLEMENTED,
    verification_evidence=VERIFICATION, blockers=BLOCKERS, summary=SUMMARY,
)


if __name__ == "__main__":
    raise SystemExit(main(DEFINITION))

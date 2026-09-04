#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 68 evidence."""

from __future__ import annotations
from pathlib import Path
from typing import Final

try:
    from scripts.sprint_evidence_recorder import SprintEvidenceDefinition, main
except ModuleNotFoundError:
    from sprint_evidence_recorder import SprintEvidenceDefinition, main

ROOT: Final = Path(__file__).resolve().parents[1]
SOURCE_PATHS: Final = (
    "Cargo.toml", "Cargo.lock", "capabilities/knowledge/Cargo.toml",
    "capabilities/knowledge/src/database_adapter.rs", "capabilities/knowledge/src/lib.rs",
    "schemas/runtime/structured-database-receipt.schema.json",
    "schemas/runtime/examples/structured-database-receipt.valid.json",
    "scripts/validate_planning_schemas.mjs", "tests/test_planning_schemas.mjs",
    "supply-chain/dependency-hashes.sha256", "supply-chain/dependency-provenance.json", "supply-chain/sbom.cdx.json",
    "docs/architecture/bounded-local-structured-database.md", "docs/guides/local-database-review.md",
    "docs/verification/sprint-68-database-corpus.json", "docs/verification/sprint-68-database-dependency-manifest.json",
    "docs/verification/sprint-68-local-results.md", "scripts/local_database_contract.py",
    "tests/test_local_database_contract.py", "scripts/sprint_evidence_recorder.py",
    "scripts/sprint_68_evidence.py", "tests/test_sprint_68_evidence.py",
)
COMMANDS: Final = (
    ("local-database-unit", ("cargo", "test", "-p", "agentmage-capability-knowledge", "database_adapter::tests", "--lib", "--locked")),
    ("local-database-contract", ("python3", "-m", "unittest", "tests.test_local_database_contract")),
    ("runtime-schema-contract", ("npm", "run", "-s", "schemas:test")),
    ("local-database-strict-clippy", ("cargo", "clippy", "-p", "agentmage-capability-knowledge", "--all-targets", "--locked", "--", "-D", "warnings")),
    ("local-database-format", ("cargo", "fmt", "--all", "--", "--check")),
    ("supply-chain-currentness", ("python3", "scripts/supply_chain.py")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_68_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:3])
SECURITY_REQUIREMENTS: Final = ("SR-ACC-002", "SR-ACC-006", "SR-DAT-001", "SR-DAT-002", "SR-DAT-003", "SR-TST-002", "SR-TST-004", "SR-TST-006")
IMPLEMENTED: Final = {
    "database_rust_fixture_count": 4, "database_corpus_case_count": 58,
    "fixed_parameterized_templates": True, "transactional_migration": True,
    "separate_schema_row_fixture_permissions": True, "sqlite_query_only": True,
    "typed_null_precision_projection": True, "redaction_before_result": True,
    "row_column_byte_limits": True, "cancellation": True, "source_freshness_receipts": True,
    "report_scope_authorization": True, "live_database_access": False, "raw_sql_access": False,
    "external_credentials": False, "filesystem_path_access": False, "network_access": False,
    "native_parity": False, "independent_review": False, "manual_fuzzing": False,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINT-67-BLOCKED", "owner": "68.1"},
    {"code": "NATIVE-DATABASE-PARITY-ABSENT", "owner": "68.1.3.5"},
    {"code": "NATIVE-LOCK-TIMEOUT-CORRUPTION-CAMPAIGN-ABSENT", "owner": "68.1.3.5"},
    {"code": "INDEPENDENT-BOUNDARY-REVIEW-ABSENT", "owner": "68.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "database_corpus_case_count": 58, "accepted_network_effect_count": 0,
    "accepted_filesystem_effect_count": 0, "live_connector_count": 0,
    "external_credential_count": 0, "raw_sql_entry_point_count": 0,
    "permission_separation_verified": True, "parameter_binding_verified": True,
    "type_redaction_limit_receipt_verified": True, "native_platform_count": 0,
    "independent_review": False, "manual_fuzzing": False, "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_68_contract_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprint_67_closed": False, "native_parity_complete": False,
    "native_failure_campaign_complete": False, "independent_review_present": False,
    "manual_fuzzing_complete": False, "live_access_enabled": False, "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=68, root=ROOT, output="artifacts/sprints/sprint-68/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]), security_requirement_ids=SECURITY_REQUIREMENTS,
    implemented_contracts=IMPLEMENTED, verification_evidence=VERIFICATION,
    blockers=BLOCKERS, summary=SUMMARY,
)

if __name__ == "__main__": raise SystemExit(main(DEFINITION))

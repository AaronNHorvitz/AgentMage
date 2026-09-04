#!/usr/bin/env python3
"""Build truthful Sprint 80 local MCP identity evidence."""

from pathlib import Path
from typing import Final

try:
    from scripts.sprint_evidence_recorder import SprintEvidenceDefinition, main
except ModuleNotFoundError:
    from sprint_evidence_recorder import SprintEvidenceDefinition, main


ROOT: Final = Path(__file__).resolve().parents[1]
SOURCE_PATHS: Final = (
    "Cargo.toml", "Cargo.lock", "kernel/contracts/Cargo.toml", "kernel/contracts/src/lib.rs",
    "kernel/contracts/src/mcp.rs", "kernel/engine/Cargo.toml", "kernel/engine/src/lib.rs",
    "kernel/engine/src/mcp_registry.rs", "kernel/engine/src/tooling.rs",
    "docs/verification/shared-runtime-mcp-workflow-local-results.md",
    "scripts/mcp_identity_contract.py", "tests/test_mcp_identity_contract.py",
    "supply-chain/dependency-hashes.sha256", "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json", "scripts/sprint_evidence_recorder.py",
    "scripts/sprint_80_evidence.py", "tests/test_sprint_80_evidence.py",
)
COMMANDS: Final = (
    ("mcp-identity-rust-contract", ("cargo", "test", "-p", "agentmage-kernel-engine", "sprint_80_", "--lib")),
    ("mcp-identity-artifact-contract", ("python3", "-m", "unittest", "tests.test_mcp_identity_contract")),
    ("runtime-schema-contract", ("npm", "run", "-s", "schemas:test")),
    ("supply-chain-currentness", ("python3", "scripts/supply_chain.py")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_80_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:2])
IMPLEMENTED: Final = {
    "record_type_count": 18, "transport_kind_count": 5, "focused_rust_test_count": 4,
    "allowed_operation_count": 1, "native_tool_prerequisite_count": 0,
    "launched_server_count": 0, "remote_connection_count": 0,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINT-79-BLOCKED", "owner": "80.1"},
    {"code": "NATIVE-MCP-SECURITY-AND-INDEPENDENT-REVIEW-ABSENT", "owner": "80.1.3.4"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "rust_test_count": 4, "manifest_identity_bound": True,
    "native_tools_remain_independent": True, "live_server_exercised": False,
    "native_security_evidence_complete": False, "independent_review": False,
    "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_80_contract_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprint_79_closed": False, "live_mcp_transport_complete": False,
    "native_security_evidence_complete": False, "independent_review_present": False,
    "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=80, root=ROOT, output="artifacts/sprints/sprint-80/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(FOCUSED_COMMANDS[:1]),
    security_requirement_ids=(
        "SR-GOV-010", "SR-ACC-001", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004",
        "SR-ACC-005", "SR-ACC-006", "SR-ACC-007", "SR-ACC-008", "SR-AI-005",
        "SR-TST-002", "SR-TST-004", "SR-TST-006",
    ),
    implemented_contracts=IMPLEMENTED, verification_evidence=VERIFICATION,
    blockers=BLOCKERS, summary=SUMMARY,
)


if __name__ == "__main__":
    raise SystemExit(main(DEFINITION))

#!/usr/bin/env python3
"""Build truthful Sprint 81 local MCP mediation evidence."""

from pathlib import Path
from typing import Final

try:
    from scripts.sprint_evidence_recorder import SprintEvidenceDefinition, main
except ModuleNotFoundError:
    from sprint_evidence_recorder import SprintEvidenceDefinition, main


ROOT: Final = Path(__file__).resolve().parents[1]
SOURCE_PATHS: Final = (
    "Cargo.toml", "Cargo.lock", "kernel/contracts/src/mcp.rs",
    "kernel/engine/src/mcp_registry.rs", "kernel/engine/src/mcp_gateway.rs",
    "kernel/engine/src/tooling.rs", "capabilities/read-only/src/artifact.rs",
    "shells/host/Cargo.toml", "shells/host/src/lib.rs",
    "shells/host/src/runtime_tools.rs", "shells/host/src/mcp_artifact_adapter.rs",
    "docs/guides/mcp-artifact-tools.md",
    "docs/verification/shared-runtime-mcp-workflow-local-results.md",
    "docs/verification/sprint-81-mcp-adversarial-corpus.json",
    "artifacts/sprints/sprint-80/local-evidence-report.json",
    "scripts/mcp_artifact_adapter_contract.py",
    "tests/test_mcp_artifact_adapter_contract.py",
    "supply-chain/dependency-hashes.sha256", "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json", "scripts/sprint_evidence_recorder.py",
    "scripts/sprint_81_evidence.py", "tests/test_sprint_81_evidence.py",
)
COMMANDS: Final = (
    ("mcp-gateway-rust-contract", ("cargo", "test", "-p", "agentmage-kernel-engine", "mcp_", "--lib")),
    ("mcp-artifact-rust-contract", ("cargo", "test", "-p", "agentmage-host", "mcp_artifact_adapter::tests", "--lib")),
    ("mcp-artifact-contract", ("python3", "-m", "unittest", "tests.test_mcp_artifact_adapter_contract")),
    ("runtime-schema-contract", ("npm", "run", "-s", "schemas:test")),
    ("supply-chain-currentness", ("python3", "scripts/supply_chain.py")),
    ("product-ci-contract", ("python3", "scripts/product_ci.py", "--check")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_81_evidence")),
)
FOCUSED_COMMANDS: Final = tuple(item[0] for item in COMMANDS[:3])
IMPLEMENTED: Final = {
    "gateway_rust_test_count": 7, "artifact_adapter_rust_test_count": 3,
    "artifact_tool_mapping_count": 7, "adversarial_case_count": 32,
    "allowed_operation_count": 1, "effect_executor_count": 0,
    "live_server_count": 0, "native_parity_campaign_count": 0,
}
BLOCKERS: Final = (
    {"code": "UPSTREAM-SPRINT-80-BLOCKED", "owner": "81"},
    {"code": "NATIVE-MCP-PARITY-ISOLATION-AND-REVIEW-ABSENT", "owner": "81.1.3.2-81.2.3.2"},
)
VERIFICATION: Final = {
    "focused_local_contracts": True, "focused_blocking_skip_count": 0,
    "rust_test_count": 10, "adversarial_case_count": 32,
    "native_dispatcher_revalidation": True, "single_use_grant_preserved": True,
    "native_server_execution": False, "native_cleanup_complete": False,
    "native_parity_complete": False, "independent_review": False,
    "sprint_gate_closed": False,
}
SUMMARY: Final = {
    "local_sprint_81_contract_passed": True, "sprint_status": "BLOCKED",
    "upstream_sprint_80_closed": False, "native_mcp_campaign_complete": False,
    "native_artifact_parity_complete": False, "independent_review_present": False,
    "release_approval": False,
}
DEFINITION: Final = SprintEvidenceDefinition(
    sprint=81, root=ROOT, output="artifacts/sprints/sprint-81/local-evidence-report.json",
    source_paths=SOURCE_PATHS, commands=COMMANDS, focused_commands=FOCUSED_COMMANDS,
    rust_focused_commands=frozenset(FOCUSED_COMMANDS[:2]),
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

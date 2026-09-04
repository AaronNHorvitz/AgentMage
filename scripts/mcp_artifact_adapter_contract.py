#!/usr/bin/env python3
"""Validate Sprint 81 MCP mediation and optional artifact exposure."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ADAPTER = ROOT / "shells/host/src/mcp_artifact_adapter.rs"
GATEWAY = ROOT / "kernel/engine/src/mcp_gateway.rs"
CORPUS = ROOT / "docs/verification/sprint-81-mcp-adversarial-corpus.json"
GUIDE = ROOT / "docs/guides/mcp-artifact-tools.md"
REQUIRED_ADAPTER = (
    "ArtifactToolKind::ALL", "mcp.agentmage.", "artifact_tool_definition",
    "artifact_tool_kind", "McpGatewaySession",
    "ToolRegistry", "ToolDispatcher", "PreGrantDispatchDisposition::GrantRequired",
    "GrantOperation::WorkspaceRead", "required_grant.single_use",
    "McpResponseClass::UntrustedStructuredData", "StateChange::NotChanged",
)
REQUIRED_GATEWAY = (
    "verify_mcp_request", "verify_mcp_response", "McpReceiptKind::Request",
    "McpReceiptKind::Response", "McpReceiptKind::Cancelled",
    "McpReceiptKind::Disconnected", "descendants_terminated",
)
FORBIDDEN_ADAPTER = (
    "std::fs", "std::process", "std::net", "Command::new", "TcpStream", "UnixStream",
    "reqwest::", "dispatch_artifact(", "ArtifactBackend",
)
EXPECTED_CATEGORIES = {
    "identity": 5, "protocol": 6, "authority": 7, "response": 5,
    "lifecycle": 5, "artifact_mapping": 4,
}


def validate() -> list[str]:
    failures: list[str] = []
    adapter = ADAPTER.read_text(encoding="utf-8")
    production = adapter.split("#[cfg(test)]", 1)[0]
    gateway = GATEWAY.read_text(encoding="utf-8")
    guide = GUIDE.read_text(encoding="utf-8")
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
    for token in REQUIRED_ADAPTER:
        if token not in production:
            failures.append(f"MCP artifact adapter boundary absent: {token}")
    for token in REQUIRED_GATEWAY:
        if token not in gateway:
            failures.append(f"MCP gateway boundary absent: {token}")
    for token in FORBIDDEN_ADAPTER:
        if token in production:
            failures.append(f"MCP artifact adapter acquired authority: {token}")
    cases = corpus.get("cases", [])
    if corpus.get("case_count") != 32 or len(cases) != 32 or len(set(cases)) != 32:
        failures.append("MCP adversarial corpus count drifted")
    if corpus.get("categories") != EXPECTED_CATEGORIES or sum(EXPECTED_CATEGORIES.values()) != 32:
        failures.append("MCP adversarial corpus categories drifted")
    if (
        corpus.get("local_contract_passed") is not True
        or corpus.get("native_malicious_server_executed") is not False
        or corpus.get("native_cleanup_complete") is not False
        or corpus.get("independent_review_present") is not False
        or corpus.get("substitution_set") != []
    ):
        failures.append("MCP adversarial corpus truth state drifted")
    if "MCP cannot recover bytes it was not given" not in guide:
        failures.append("MCP missing-byte limitation is absent")
    if adapter.count("fn sprint_81_") != 3:
        failures.append("Sprint 81 artifact adapter test inventory drifted")
    return failures


if __name__ == "__main__":
    errors = validate()
    if errors:
        print("\n".join(errors))
        raise SystemExit(1)
    print("validated 7 MCP artifact mappings and 32 adversarial cases")

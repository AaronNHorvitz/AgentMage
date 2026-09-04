#!/usr/bin/env python3
"""Validate the inert Sprint 80 MCP identity and manifest boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CONTRACT = ROOT / "kernel/contracts/src/mcp.rs"
REGISTRY = ROOT / "kernel/engine/src/mcp_registry.rs"

CONTRACT_TYPES = (
    "McpTransportKind", "McpTransport", "McpLimits", "McpResponseClass",
    "McpToolManifest", "McpResourceManifest", "McpPromptManifest", "McpManifest",
    "McpConnection", "McpDiscovery", "McpRequestKind", "McpRequest", "McpTerminalState",
    "McpResponse", "McpError", "McpCancellation", "McpDisconnect", "McpReceipt",
)
TRANSPORTS = (
    "InProcess", "LocalProcessStdio", "LocalSocket", "LoopbackTcp", "RemoteHttps",
)
REGISTRY_BOUNDARIES = (
    "McpProcessObservation", "McpManifestRegistry", "seal_mcp_manifest",
    "verify_mcp_manifest", "admit_mcp_connection", "verify_mcp_connection",
    "mcp_discovery", "WriteCapabilityDenied", "ToolShadowDenied", "IdentityDenied",
    "GrantOperation::WorkspaceRead", "register_tools_into", "descendants_contained",
)
FORBIDDEN_EXECUTORS = (
    "std::process", "std::net", "Command::new", "TcpStream", "UnixStream", "reqwest::",
)


def validate() -> list[str]:
    """Return every contract drift without executing an MCP server."""

    failures: list[str] = []
    contract = CONTRACT.read_text(encoding="utf-8")
    registry = REGISTRY.read_text(encoding="utf-8")
    production = registry.split("#[cfg(test)]", 1)[0]
    for token in (*CONTRACT_TYPES, *TRANSPORTS):
        if token not in contract:
            failures.append(f"MCP contract token absent: {token}")
    for token in REGISTRY_BOUNDARIES:
        if token not in production:
            failures.append(f"MCP registry boundary absent: {token}")
    for token in FORBIDDEN_EXECUTORS:
        if token in production:
            failures.append(f"MCP identity layer acquired an executor: {token}")
    if "manifest.requested_operations != [GrantOperation::WorkspaceRead]" not in production:
        failures.append("requested operation admission is not visibly enforced")
    if registry.count("fn sprint_80_") != 4:
        failures.append("Sprint 80 focused test inventory drifted")
    return failures


if __name__ == "__main__":
    errors = validate()
    if errors:
        print("\n".join(errors))
        raise SystemExit(1)
    print("validated 18 MCP record types, 5 transports, and 4 focused identity cases")

#!/usr/bin/env python3
"""Validate and retain the source-only macOS bridge and App Group socket contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-8/story-8.1/macos-bridge-source-contract.json"
SOURCE_PATHS = (
    "platforms/macos/Sources/AgentMageMacOSPlatform/AppGroupSocketBoundary.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSBridgeProtocol.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSBridgeProtocolTests.swift",
    "scripts/macos_bridge_source_contract.py",
    "tests/test_macos_bridge_source_contract.py",
)
BRIDGE_FAILURES = (
    "macos.ipc.invalid-identity",
    "macos.ipc.invalid-launch-material",
    "macos.ipc.version-mismatch",
    "macos.ipc.challenge-mismatch",
    "macos.ipc.bridge-identity-mismatch",
    "macos.ipc.authentication-failed",
    "macos.ipc.replay",
    "macos.ipc.malformed-frame",
    "macos.ipc.resource-limit-exceeded",
)
SOCKET_FAILURES = (
    "macos.ipc.socket.outside-app-group",
    "macos.ipc.socket.unsafe-parent-type",
    "macos.ipc.socket.wrong-parent-owner",
    "macos.ipc.socket.wrong-parent-mode",
    "macos.ipc.socket.preexisting-path",
    "macos.ipc.socket.wrong-socket-type",
    "macos.ipc.socket.wrong-socket-owner",
    "macos.ipc.socket.wrong-socket-mode",
)


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    result = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return result if binary else result.decode().strip()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_sources(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    for relative in SOURCE_PATHS:
        if not (root / relative).is_file():
            failures.append(f"missing source input: {relative}")
    if failures:
        return failures
    protocol = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSBridgeProtocol.swift"
    ).read_text(encoding="utf-8")
    socket = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/AppGroupSocketBoundary.swift"
    ).read_text(encoding="utf-8")
    tests = (
        root
        / "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSBridgeProtocolTests.swift"
    ).read_text(encoding="utf-8")

    required = (
        "agentMageMacOSIPCProtocolVersion: UInt32 = 1",
        "agentMageMacOSHandshakeBytes = 68",
        "agentMageMacOSMaximumRequestBytes = 64 * 1024",
        "agentMageMacOSMaximumResponseBytes = 4 * 1024 * 1024",
        'Data("agentmage-macos-ipc-auth-v1\\0".utf8)',
        "HMAC<SHA256>.authenticationCode",
        "private var consumed = false",
        "constantTimeEqual",
    )
    for term in required:
        if term not in protocol:
            failures.append(f"bridge protocol missing required term: {term}")
    for refusal in BRIDGE_FAILURES:
        if protocol.count(f'"{refusal}"') != 1:
            failures.append(f"bridge refusal is not exact: {refusal}")
    for refusal in SOCKET_FAILURES:
        if socket.count(f'"{refusal}"') != 1:
            failures.append(f"socket refusal is not exact: {refusal}")
    for term in (
        'agentMageMacOSSocketRelativePath = "Library/Application Support/AgentMage/ipc/host.sock"',
        "observation.parentMode == 0o700",
        "observation.socketMode == 0o600",
        "observation.insideExpectedAppGroupContainer",
        "observation.socketPathAbsentBeforeBind",
        "observation.socketIsUnixDomainSocket",
    ):
        if term not in socket:
            failures.append(f"App Group socket boundary missing required term: {term}")
    if "cases.count == AppGroupSocketFailure.allCases.count" not in tests:
        failures.append("socket mutation matrix is not closed over every refusal")
    if tests.count("cases.append((item, .") != len(SOCKET_FAILURES):
        failures.append("socket mutation matrix does not independently cover each refusal")
    if "audit-token" in protocol.casefold() or "designated-requirement" in protocol.casefold():
        failures.append("8.1.1.2 source improperly claims 8.1.1.3 peer verification")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed macOS bridge source changed: {relative}")
    return revision, tree


def build_report(
    root: Path = ROOT, *, source_revision: str = "HEAD"
) -> dict[str, Any]:
    failures = validate_sources(root)
    if failures:
        raise ValueError("; ".join(failures))
    revision, tree = resolve_revision(source_revision, root)
    return {
        "schema_version": 1,
        "record_type": "macos-bridge-source-contract",
        "task_id": "8.1.1.2",
        "status": "partial-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "protocol_version": 1,
            "handshake_bytes": 68,
            "request_maximum_bytes": 64 * 1024,
            "response_maximum_bytes": 4 * 1024 * 1024,
            "bridge_refusal_count": len(BRIDGE_FAILURES),
            "socket_refusal_count": len(SOCKET_FAILURES),
            "socket_parent_mode": "0700",
            "socket_mode": "0600",
            "socket_relative_path": "Library/Application Support/AgentMage/ipc/host.sock",
            "replay_consumed_atomically": True,
        },
        "execution": {
            "swift_build_performed": False,
            "swift_tests_performed": False,
            "apple_silicon_execution_performed": False,
            "signed_bridge_executed": False,
            "app_group_socket_bound": False,
            "native_handshake_performed": False,
        },
        "claims": {
            "task_complete": False,
            "audit_token_verified": False,
            "designated_requirement_verified": False,
            "macos_support": False,
            "package_or_release": False,
        },
        "remaining_blockers": [
            "Apple Silicon Swift build and tests have not run.",
            "A signed native bridge executable is not yet integrated with the Visual Studio Code shell.",
            "A native App Group Unix socket has not been bound or inspected.",
            "Audit-token and designated-requirement verification remain task 8.1.1.3.",
            "Physical MacBook Pro M5 qualification has not run.",
        ],
    }


def canonical_bytes(report: dict[str, Any]) -> bytes:
    return (json.dumps(report, indent=2, sort_keys=True) + "\n").encode()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            revision = arguments.source_revision
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained report has no source revision")
        expected = canonical_bytes(build_report(source_revision=revision))
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS bridge source contract failed: {error}")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_bytes(expected)
    elif REPORT_PATH.read_bytes() != expected:
        print("macOS bridge source report is missing or stale")
        return 1
    print("macOS bridge source contract passed without native or release promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

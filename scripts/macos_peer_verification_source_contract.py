#!/usr/bin/env python3
"""Validate and retain source-only macOS audit-token peer verification."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-peer-verification-source-contract.json"
)
SOURCE_PATHS = (
    "platforms/macos/README.md",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSBridgeProtocol.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSPeerVerification.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSBridgeProtocolTests.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSPeerVerificationTests.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/PlatformBoundaryTests.swift",
    "release/platform-manifests/macos/v1/contract-fixture.json",
    "scripts/macos_peer_verification_source_contract.py",
    "tests/test_macos_peer_verification_source_contract.py",
)
PEER_FAILURES = (
    "macos.ipc.peer.invalid-expectation",
    "macos.ipc.peer.audit-token-missing",
    "macos.ipc.peer.audit-token-size-mismatch",
    "macos.ipc.peer.invalid-process",
    "macos.ipc.peer.wrong-process",
    "macos.ipc.peer.wrong-effective-user",
    "macos.ipc.peer.wrong-effective-group",
    "macos.ipc.peer.invalid-code-signature",
    "macos.ipc.peer.designated-requirement-mismatch",
    "macos.ipc.peer.wrong-bundle-identifier",
    "macos.ipc.peer.wrong-team-identifier",
    "macos.ipc.peer.app-sandbox-missing",
    "macos.ipc.peer.wrong-app-group",
    "macos.ipc.peer.entitlement-closure-mismatch",
)
OBSERVATION_FAILURES = (
    "macos.ipc.peer-observation.invalid-socket",
    "macos.ipc.peer-observation.cannot-read-audit-token",
    "macos.ipc.peer-observation.malformed-audit-token",
    "macos.ipc.peer-observation.cannot-read-peer-identity",
    "macos.ipc.peer-observation.cannot-resolve-peer-code",
    "macos.ipc.peer-observation.cannot-compile-requirement",
    "macos.ipc.peer-observation.cannot-read-signing-information",
)


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_sources(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    for relative in SOURCE_PATHS:
        if not (root / relative).is_file():
            failures.append(f"missing source input: {relative}")
    if failures:
        return failures

    peer = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSPeerVerification.swift"
    ).read_text(encoding="utf-8")
    protocol = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSBridgeProtocol.swift"
    ).read_text(encoding="utf-8")
    tests = (
        root
        / "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSPeerVerificationTests.swift"
    ).read_text(encoding="utf-8")
    boundary = (
        root
        / "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift"
    ).read_text(encoding="utf-8")
    readme = (root / "platforms/macos/README.md").read_text(encoding="utf-8")
    manifest = json.loads(
        (root / "release/platform-manifests/macos/v1/contract-fixture.json").read_text(
            encoding="utf-8"
        )
    )

    required_terms = (
        "LOCAL_PEERTOKEN",
        "LOCAL_PEERPID",
        "getpeereid",
        "kSecGuestAttributeAudit",
        "SecCodeCopyGuestWithAttributes",
        "SecRequirementCreateWithString",
        "SecCodeCheckValidity",
        "kSecCodeInfoEntitlementsDict",
        "kSecCodeInfoIdentifier",
        "kSecCodeInfoTeamIdentifier",
        "agentMageMacOSAuditTokenBytes",
        "public final class MacOSBridgeSessionAdmission",
        "try authenticator.authenticate(verifiedPeer: verifiedPeer, frame: frame)",
    )
    for term in required_terms:
        if term not in peer:
            failures.append(f"peer verification source missing required term: {term}")
    if "public final class MacOSBridgeAuthenticator" in protocol:
        failures.append("unverified callers can reach the raw host authenticator")
    if "verifiedPeer: VerifiedMacOSBridgePeer" not in protocol:
        failures.append("launch authenticator is not restricted to a verified peer")

    peer_enum = peer.split("public enum MacOSPeerFailure", 1)[1].split(
        "public struct VerifiedMacOSBridgePeer", 1
    )[0]
    observation_enum = peer.split("public enum MacOSPeerObservationError", 1)[1].split(
        "public enum SecurityFrameworkMacOSPeerObserver", 1
    )[0]
    for refusal in PEER_FAILURES:
        if peer_enum.count(f'"{refusal}"') != 1:
            failures.append(f"peer refusal is not exact: {refusal}")
    for refusal in OBSERVATION_FAILURES:
        if observation_enum.count(f'"{refusal}"') != 1:
            failures.append(f"peer observation refusal is not exact: {refusal}")
    if "cases.count + 1 == MacOSPeerFailure.allCases.count" not in tests:
        failures.append("peer mutation matrix is not closed over every refusal")
    if tests.count("cases.append((item, .") != len(PEER_FAILURES) - 1:
        failures.append("peer mutation matrix does not independently cover each refusal")
    for term in (
        "MacOSPeerFailure.wrongProcessIdentifier",
        "MacOSBridgeFailure.replay",
        "MacOSBridgeFailure.challengeMismatch",
    ):
        if term not in tests:
            failures.append(f"composed peer/challenge test missing: {term}")
    if 'peerVerificationSourceStatus = "implemented-source-unverified"' not in boundary:
        failures.append("peer verification source status is not explicitly unverified")
    for statement in (
        "no native socket observation, signed peer,",
        "Native compilation, signing, execution, and support remain `BLOCKED-MACOS`",
    ):
        if statement not in readme:
            failures.append(f"README missing peer non-promotion statement: {statement}")

    identity = manifest["code_identity"]
    bridge_bundle = identity["bundle_identifiers"]["vscode_bridge"]
    team_id = identity["team_id"]
    exact_requirement = (
        f"anchor apple generic and identifier {bridge_bundle} "
        f"and certificate leaf[subject.OU] = {team_id}"
    )
    if identity["designated_requirements"]["vscode_bridge"] != exact_requirement:
        failures.append("release manifest bridge requirement is not exact")
    if identity["app_group_identifier"] != "group.com.example.agentmage.contractfixture":
        failures.append("peer source fixture App Group is not visibly synthetic")
    if "com.example" not in bridge_bundle or team_id != "AAAAAAAAAA":
        failures.append("peer source fixture signing identity is not visibly synthetic")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed macOS peer source changed: {relative}")
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
        "record_type": "macos-peer-verification-source-contract",
        "task_id": "8.1.1.3",
        "status": "partial-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "peer_refusal_count": len(PEER_FAILURES),
            "observation_refusal_count": len(OBSERVATION_FAILURES),
            "audit_token_socket_option": "LOCAL_PEERTOKEN",
            "peer_pid_socket_option": "LOCAL_PEERPID",
            "effective_peer_credentials": ["uid", "gid"],
            "live_code_lookup_attribute": "kSecGuestAttributeAudit",
            "designated_requirement_exactly_release_bound": True,
            "verified_peer_required_before_challenge_consumption": True,
            "fresh_challenge_and_replay_checks_composed": True,
        },
        "execution": {
            "swift_build_performed": False,
            "swift_tests_performed": False,
            "apple_silicon_execution_performed": False,
            "native_socket_observed": False,
            "signed_peer_verified": False,
            "native_handshake_performed": False,
        },
        "claims": {
            "task_complete": False,
            "audit_token_verified_on_macos": False,
            "designated_requirement_verified_on_macos": False,
            "macos_support": False,
            "package_or_release": False,
        },
        "remaining_blockers": [
            "Apple Silicon Swift 6 build and tests have not run.",
            "No accepted native App Group socket has supplied a LOCAL_PEERTOKEN observation.",
            "No signed bridge has satisfied the release-designated requirement at runtime.",
            "The bridge and host executables are not integrated or signed.",
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
        print(f"macOS peer verification source contract failed: {error}")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_bytes(expected)
    elif REPORT_PATH.read_bytes() != expected:
        print("macOS peer verification source report is missing or stale")
        return 1
    print("macOS peer verification source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

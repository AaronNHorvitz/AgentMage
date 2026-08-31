#!/usr/bin/env python3
"""Validate and retain the source-only macOS Metal inference-service boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import plistlib
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-metal-inference-source-contract.json"
)
SOURCE_PATHS = (
    "platforms/macos/Configuration/MetalInferenceService.contract-fixture.entitlements",
    "platforms/macos/README.md",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSMetalInferenceProtocol.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSMetalInferenceService.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/PlatformBoundary.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSMetalInferenceTests.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/PlatformBoundaryTests.swift",
    "release/platform-manifests/macos/v1/contract-fixture.json",
    "model-profiles/runtimes/llama-cpp-b10333-macos-arm64.json",
    "scripts/macos_metal_inference_source_contract.py",
    "tests/test_macos_metal_inference_source_contract.py",
)
METAL_INFERENCE_FAILURES = (
    "macos.metal-inference.malformed-envelope",
    "macos.metal-inference.unsupported-schema",
    "macos.metal-inference.invalid-identity",
    "macos.metal-inference.invalid-profile",
    "macos.metal-inference.invalid-manifest-digest",
    "macos.metal-inference.invalid-artifact-digest",
    "macos.metal-inference.invalid-runtime-digest",
    "macos.metal-inference.invalid-codec-digest",
    "macos.metal-inference.invalid-input",
    "macos.metal-inference.input-limit-exceeded",
    "macos.metal-inference.input-digest-mismatch",
    "macos.metal-inference.invalid-context",
    "macos.metal-inference.invalid-output-limit",
    "macos.metal-inference.invalid-sampling",
    "macos.metal-inference.invalid-deadline",
    "macos.metal-inference.replay",
    "macos.metal-inference.capacity-exceeded",
    "macos.metal-inference.service-identity-rejected",
    "macos.metal-inference.host-identity-rejected",
    "macos.metal-inference.connection-rejected",
    "macos.metal-inference.resource-limit-failed",
    "macos.metal-inference.metal-unavailable",
    "macos.metal-inference.model-descriptor-rejected",
    "macos.metal-inference.model-digest-mismatch",
    "macos.metal-inference.profile-substitution",
    "macos.metal-inference.executor-failed",
    "macos.metal-inference.timeout",
    "macos.metal-inference.cancelled",
    "macos.metal-inference.result-limit-exceeded",
    "macos.metal-inference.result-encoding-failed",
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

    protocol = (root / SOURCE_PATHS[2]).read_text(encoding="utf-8")
    service = (root / SOURCE_PATHS[3]).read_text(encoding="utf-8")
    boundary = (root / SOURCE_PATHS[4]).read_text(encoding="utf-8")
    tests = (root / SOURCE_PATHS[5]).read_text(encoding="utf-8")
    readme = (root / "platforms/macos/README.md").read_text(encoding="utf-8")
    manifest = json.loads(
        (root / "release/platform-manifests/macos/v1/contract-fixture.json")
        .read_text(encoding="utf-8")
    )
    runtime = json.loads(
        (root / "model-profiles/runtimes/llama-cpp-b10333-macos-arm64.json")
        .read_text(encoding="utf-8")
    )

    protocol_terms = (
        "Set(dictionary.keys) == keys",
        "agentMageMacOSMetalInputMaximumBytes = 2 * 1024 * 1024",
        "agentMageMacOSMetalResultMaximumBytes = 4 * 1024 * 1024",
        "agentMageMacOSMetalMaximumContextTokens: UInt32 = 32_768",
        "agentMageMacOSMetalMaximumOutputTokens: UInt32 = 4_096",
        "agentMageMacOSMetalMaximumDeadlineMilliseconds: UInt64 = 120_000",
        "agentMageMacOSMetalMaximumModelBytes: UInt64 = 64 * 1024 * 1024 * 1024",
        "agentMageMacOSMetalMaximumRequestIdentities = 1_024",
        "metalSHA256(request.inputData) == request.inputSHA256",
        "seen.insert(requestIdentifier)",
        "throw MacOSMetalInferenceFailure.replay",
    )
    for term in protocol_terms:
        if term not in protocol:
            failures.append(f"Metal protocol missing required term: {term}")
    request_struct = protocol.split(
        "public struct MacOSMetalInferenceRequest", 1
    )[1].split("public struct MacOSMetalVerifiedProfile", 1)[0]
    for prohibited in (
        "workspace", "bookmark", "tool", "grant", "credential", "environment",
        "network", "endpoint", "URL",
    ):
        if prohibited.lower() in request_struct.lower():
            failures.append(f"Metal request contains prohibited authority: {prohibited}")

    service_terms = (
        "NSXPCListener.service()",
        "NSXPCInterface(",
        "MacOSMetalInferenceXPCProtocol.self",
        "SecCodeCopySelf",
        "SecCodeCopyGuestWithAttributes",
        "kSecGuestAttributePid",
        "SecRequirementCreateWithString",
        "connection.processIdentifier",
        "connection.effectiveUserIdentifier",
        "connection.effectiveGroupIdentifier",
        "MTLCreateSystemDefaultDevice()",
        "modelFile: FileHandle",
        "let duplicated = dup(fileHandle.fileDescriptor)",
        "flags & O_ACCMODE",
        "observation.accessMode == O_RDONLY",
        "(status.st_mode & S_IFMT) == S_IFREG",
        "status.st_uid == geteuid()",
        "status.st_nlink",
        "pread(",
        "observedDigest == expected.artifactSHA256",
        "guard verified.profile == expectedProfile",
        "profile == candidate",
        "readOnlyModelFileDescriptor: model.fileDescriptor",
        "cancellationRequested:",
        "RLIMIT_CORE", "RLIMIT_NOFILE", "RLIMIT_NPROC", "RLIMIT_CPU",
        "RLIMIT_AS", "RLIMIT_FSIZE",
        "_exit(124)", "_exit(125)",
    )
    for term in service_terms:
        if term not in service:
            failures.append(f"Metal service missing required term: {term}")
    for prohibited in (
        "URLSession", "NWConnection", "SecItemCopyMatching", "NSOpenPanel",
        ".withSecurityScope", "workspaceRootFileDescriptor",
    ):
        if prohibited in service:
            failures.append(f"Metal service contains prohibited authority: {prohibited}")
    exact_counts = {
        "NSXPCListener.service()": 1,
        "MTLCreateSystemDefaultDevice()": 1,
        "_exit(124)": 1,
        "_exit(125)": 2,
    }
    for term, count in exact_counts.items():
        if service.count(term) != count:
            failures.append(f"Metal service term count changed: {term}")

    failure_enum = protocol.split(
        "public enum MacOSMetalInferenceFailure", 1
    )[1].split("public struct MacOSMetalInferenceRequest", 1)[0]
    for refusal in METAL_INFERENCE_FAILURES:
        if failure_enum.count(f'"{refusal}"') != 1:
            failures.append(f"Metal refusal is not exact: {refusal}")
    if "MacOSMetalInferenceFailure.allCases.count == 30" not in tests:
        failures.append("Metal failure taxonomy count is not tested")
    if "#expect(cases.count == 14)" not in tests:
        failures.append("Metal request mutation matrix is not closed")
    if tests.count("cases.append((item, .") != 14:
        failures.append("Metal request mutations are incomplete")
    if 'metalInferenceSourceStatus = "implemented-source-unverified"' not in boundary:
        failures.append("Metal source status is not explicitly unverified")

    entitlement_path = root / SOURCE_PATHS[0]
    entitlements = plistlib.loads(entitlement_path.read_bytes())
    expected_entitlements = {"com.apple.security.app-sandbox": True}
    if entitlements != expected_entitlements:
        failures.append("Metal entitlement closure is not the exact one-key sandbox")
    if manifest["entitlements"]["metal_inference_service"] != list(
        expected_entitlements
    ):
        failures.append("release manifest Metal entitlement closure changed")
    for prohibited in (
        "com.apple.security.network.client",
        "com.apple.security.network.server",
        "com.apple.security.application-groups",
        "com.apple.security.files.bookmarks.app-scope",
        "com.apple.security.files.user-selected.read-only",
        "com.apple.security.files.user-selected.read-write",
        "keychain-access-groups",
        "com.apple.security.cs.allow-jit",
        "com.apple.security.cs.disable-library-validation",
    ):
        if prohibited in entitlements:
            failures.append(f"Metal entitlement grants prohibited authority: {prohibited}")

    if runtime.get("adapter_id") != "macos-native-metal":
        failures.append("macOS runtime candidate is not the native Metal adapter")
    if runtime.get("decision", {}).get("execution_status") != "NOT_RUN":
        failures.append("macOS runtime candidate incorrectly claims execution")
    if runtime.get("decision", {}).get("release_approval") is not False:
        failures.append("macOS runtime candidate incorrectly claims release approval")
    for statement in (
        "already-open file handle",
        "It deliberately has no workspace path or bookmark, tool, grant,",
        "The service has only `com.apple.security.app-sandbox`",
        "no signed inference target,",
        "Native compilation, signing, execution, and support remain `BLOCKED-MACOS`",
    ):
        if statement not in readme:
            failures.append(f"README missing Metal non-promotion statement: {statement}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed macOS Metal source changed: {relative}")
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
        "record_type": "macos-metal-inference-source-contract",
        "task_id": "8.1.1.6",
        "status": "partial-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "failure_count": len(METAL_INFERENCE_FAILURES),
            "xpc_connections_per_process": 1,
            "profiles_per_process": 1,
            "concurrent_inferences": 1,
            "retained_request_identities": 1_024,
            "input_maximum_bytes": 2 * 1024 * 1024,
            "result_maximum_bytes": 4 * 1024 * 1024,
            "context_maximum_tokens": 32_768,
            "output_maximum_tokens": 4_096,
            "wall_deadline_milliseconds": 120_000,
            "model_maximum_bytes": 64 * 1024 * 1024 * 1024,
            "model_authority": "verified-read-only-file-descriptor-only",
            "service_entitlements": ["com.apple.security.app-sandbox"],
            "network_entitlement": False,
            "workspace_entitlement": False,
            "bookmark_entitlement": False,
            "app_group_entitlement": False,
            "keychain_entitlement": False,
            "jit_entitlement": False,
            "library_validation_exception": False,
        },
        "execution": {
            "swift_build_performed": False,
            "swift_tests_performed": False,
            "apple_silicon_execution_performed": False,
            "signed_xpc_service_executed": False,
            "native_file_handle_transfer_performed": False,
            "metal_device_observed": False,
            "llama_cpp_loaded": False,
            "gguf_inference_performed": False,
            "app_sandbox_observed": False,
            "native_attack_campaign_executed": False,
            "native_lifecycle_campaign_executed": False,
        },
        "claims": {
            "task_complete": False,
            "workspace_authority": False,
            "tool_authority": False,
            "grant_authority": False,
            "credential_authority": False,
            "network_authority": False,
            "macos_support": False,
            "package_or_release": False,
        },
        "remaining_blockers": [
            "Apple Silicon Swift 6 build and tests have not run.",
            "No signed embedded inference XPC service target or release-derived identity exists.",
            "No native XPC host connection has transferred a read-only model file handle.",
            "The signed llama.cpp Metal runtime is not composed with the executor boundary.",
            "No manifest-pinned installed GGUF has been loaded or used for inference.",
            "Native App Sandbox, resource, network, attack, cancellation, crash, and cleanup campaigns have not run.",
            "Independent critical-boundary review has not been retained.",
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
        print(f"macOS Metal inference source contract failed: {error}")
        return 1
    if arguments.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_bytes(expected)
    elif REPORT_PATH.read_bytes() != expected:
        print("macOS Metal inference source report is missing or stale")
        return 1
    print("macOS Metal inference source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

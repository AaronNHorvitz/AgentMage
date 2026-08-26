#!/usr/bin/env python3
"""Validate the accepted AgentMage language and build-system decision."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
MATRIX_PATH = ROOT / "architecture" / "language-build-matrix.json"

EXPECTED_COMPONENTS = {
    "kernel": (
        "rust",
        "cargo",
        {"macos-arm64", "fedora-x86_64", "ubuntu-x86_64", "windows-x86_64"},
    ),
    "linux-platform-adapter": ("rust", "cargo", {"fedora-x86_64", "ubuntu-x86_64"}),
    "macos-platform-adapter": ("swift", "swiftpm-xcode", {"macos-arm64"}),
    "windows-platform-adapter": ("rust", "cargo", {"windows-x86_64"}),
    "vscode-extension": (
        "typescript",
        "npm",
        {"macos-arm64", "fedora-x86_64", "ubuntu-x86_64", "windows-x86_64"},
    ),
    "build-orchestrator": (
        "rust",
        "cargo-xtask",
        {"macos-arm64", "fedora-x86_64", "ubuntu-x86_64", "windows-x86_64"},
    ),
}
EXPECTED_TARGETS = {
    "macos-arm64": "aarch64-apple-darwin",
    "fedora-x86_64": "x86_64-unknown-linux-gnu",
    "ubuntu-x86_64": "x86_64-unknown-linux-gnu",
    "windows-x86_64": "x86_64-pc-windows-msvc",
}
PROHIBITED_EXTENSION_AUTHORITY = {
    "workspace-read",
    "workspace-write",
    "git",
    "model-runtime",
    "tool-execution",
    "grant-minting",
    "secret-store",
}
# Story 1.2 narrows the extension's read surface to references the user supplied with
# the current participant request. The permission is narrower than a workspace read:
# every ambient capability below stays prohibited, and `workspace-read` itself remains
# in PROHIBITED_EXTENSION_AUTHORITY.
REQUEST_REFERENCE_KINDS = [
    "request-attached-file",
    "request-attached-selection",
    "request-attached-uri",
    "request-attached-editor-context",
]
REQUEST_REFERENCE_EXACT_VALUES = {
    "permitted_source": "chat-participant-request-references",
    "permitted_api_channel": "stable",
    "scope": "current-request-only",
    "resolution_authority": "rust-host",
    "unresolvable_reference_disposition": "declared-unsupported-or-unavailable",
}
# Each of these must remain false. Setting any one of them to true would turn a
# request-bound resolution back into an ambient workspace read.
REQUEST_REFERENCE_PROHIBITED_FLAGS = [
    "ambient_workspace_read_permitted",
    "ambient_enumeration_permitted",
    "independent_path_selection_permitted",
    "background_indexing_permitted",
    "policy_bypassing_read_permitted",
    "reference_persistence_permitted",
    "proposed_api_required",
]


def load_matrix(path: Path = MATRIX_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _unique_by_id(records: Any, label: str, failures: list[str]) -> dict[str, Any]:
    if not isinstance(records, list):
        failures.append(f"{label} must be an array")
        return {}

    indexed: dict[str, Any] = {}
    for record in records:
        if not isinstance(record, dict) or not isinstance(record.get("id"), str):
            failures.append(f"every {label} entry must have a string id")
            continue
        record_id = record["id"]
        if record_id in indexed:
            failures.append(f"duplicate {label} id: {record_id}")
        indexed[record_id] = record
    return indexed


def _validate_request_reference_contract(contract: Any) -> list[str]:
    """Validate the narrowed request-bound reference permission.

    The extension may resolve only references the user supplied with the current
    AgentMage participant request, through stable APIs. Ambient enumeration, arbitrary
    path selection, background indexing, policy-bypassing reads, and persistence beyond
    the request stay prohibited, so this permission can never widen into a general
    workspace read.
    """
    failures: list[str] = []
    if not isinstance(contract, dict):
        return ["vscode_contract.request_reference_contract must be an object"]

    for key, expected in sorted(REQUEST_REFERENCE_EXACT_VALUES.items()):
        if contract.get(key) != expected:
            failures.append(f"request reference {key} must equal {expected}")

    if contract.get("permitted_reference_kinds") != REQUEST_REFERENCE_KINDS:
        failures.append("request reference kinds must be the exact closed current-request set")

    for flag in REQUEST_REFERENCE_PROHIBITED_FLAGS:
        if contract.get(flag) is not False:
            failures.append(f"request reference {flag} must remain false")

    unknown = set(contract) - set(REQUEST_REFERENCE_EXACT_VALUES) - set(
        REQUEST_REFERENCE_PROHIBITED_FLAGS
    ) - {"permitted_reference_kinds"}
    if unknown:
        failures.append(
            "unknown request reference contract keys: " + ", ".join(sorted(unknown))
        )
    return failures


def validate_matrix(matrix: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(matrix, dict):
        return ["language/build matrix must be an object"]

    if matrix.get("schema_version") != 2:
        failures.append("schema_version must equal 2")
    if matrix.get("decision_id") != "ADR-0004":
        failures.append("decision_id must equal ADR-0004")
    if matrix.get("amending_decision_id") != "ADR-0012":
        failures.append("amending_decision_id must equal ADR-0012")
    if matrix.get("planning_decision_ids") != ["ADR-0043", "ADR-0044", "ADR-0045"]:
        failures.append("planning decision identities must equal ADR-0043, ADR-0044, and ADR-0045")
    if matrix.get("status") != "accepted":
        failures.append("architecture decision must be accepted")

    components = _unique_by_id(matrix.get("product_components"), "component", failures)
    if set(components) != set(EXPECTED_COMPONENTS):
        failures.append("product component set does not match accepted architecture")
    for component_id, (language, build_system, platforms) in EXPECTED_COMPONENTS.items():
        component = components.get(component_id, {})
        if component.get("language") != language:
            failures.append(f"{component_id} language must be {language}")
        if component.get("build_system") != build_system:
            failures.append(f"{component_id} build system must be {build_system}")
        if set(component.get("platforms", [])) != platforms:
            failures.append(f"{component_id} platform set does not match accepted scope")
        if "shipped" in component:
            failures.append(f"{component_id} must use status_ref instead of shipped")
        expected_ref = f"architecture/status-model.json#component={component_id}"
        if component.get("status_ref") != expected_ref:
            failures.append(f"{component_id} must reference its canonical current status")

    targets = _unique_by_id(matrix.get("platform_targets"), "platform target", failures)
    if set(targets) != set(EXPECTED_TARGETS):
        failures.append("platform target set does not match accepted architecture")
    for target_id, expected_target in EXPECTED_TARGETS.items():
        target = targets.get(target_id, {})
        if target.get("rust_target") != expected_target:
            failures.append(f"{target_id} Rust target does not match accepted architecture")
        if "implementation_status" in target or "verification_status" in target:
            failures.append(f"{target_id} must obtain current status through status_ref")
        expected_ref = f"architecture/status-model.json#platform={target_id}"
        if target.get("status_ref") != expected_ref:
            failures.append(f"{target_id} must reference its canonical current status")

    build = matrix.get("build_contract")
    if not isinstance(build, dict):
        failures.append("build_contract must be an object")
    else:
        if build.get("rust_edition") != "2024" or build.get("cargo_resolver") != "3":
            failures.append("Rust edition 2024 must use Cargo resolver 3")
        expected_locks = {"Cargo.lock", "package-lock.json", "Package.resolved"}
        if set(build.get("lockfiles_committed", [])) != expected_locks:
            failures.append("all selected build-system lockfiles must be committed")
        if build.get("end_user_ambient_toolchains") != []:
            failures.append("end users must not require ambient development toolchains")

    tooling = matrix.get("non_product_tooling")
    if not isinstance(tooling, list):
        failures.append("non_product_tooling must be an array")
    else:
        python_records = [item for item in tooling if item.get("language") == "python"]
        if len(python_records) != 1 or python_records[0].get("end_user_dependency") is not False:
            failures.append("Python must remain non-product tooling with no end-user dependency")

    vscode = matrix.get("vscode_contract")
    if not isinstance(vscode, dict):
        failures.append("vscode_contract must be an object")
    else:
        if vscode.get("api") != "vscode.lm.registerLanguageModelChatProvider":
            failures.append("the stable VS Code language-model provider API must be selected")
        if vscode.get("contribution_point") != "contributes.languageModelChatProviders":
            failures.append("the VS Code provider contribution point must be selected")
        if vscode.get("api_channel") != "stable" or vscode.get("proposed_api_allowed") is not False:
            failures.append("proposed VS Code APIs must remain disabled")
        if vscode.get("canonical_reliable_interface") != "agentmage-verified-chat":
            failures.append("Verified Chat must remain the canonical reliable interface")
        if vscode.get("native_compatibility_interfaces") != [
            "chat-participant",
            "language-model-chat-provider",
        ]:
            failures.append("native VS Code compatibility interface set is incomplete")
        if vscode.get("native_semantic_gap_disclosure_required") is not True:
            failures.append("native VS Code semantic gaps must be disclosed")
        if vscode.get("webview_state_authoritative") is not False:
            failures.append("the Verified Chat webview cannot own runtime state")
        if vscode.get("rust_host_state_authoritative") is not True:
            failures.append("the Rust host must own Verified Chat runtime state")
        prohibited = set(vscode.get("extension_prohibited_authority", []))
        missing = PROHIBITED_EXTENSION_AUTHORITY - prohibited
        if missing:
            failures.append(
                "VS Code extension missing prohibited authority: " + ", ".join(sorted(missing))
            )
        if "request-bound-reference-resolution" not in vscode.get("extension_authority", []):
            failures.append(
                "the VS Code extension must declare request-bound-reference-resolution authority"
            )
        failures.extend(_validate_request_reference_contract(vscode.get("request_reference_contract")))

    runtime = matrix.get("planned_runtime_contract")
    expected_runtime = {
        "language": "rust",
        "strict_local_complete_target": True,
        "remote_inference_optional": True,
        "automatic_fallback": False,
        "profile_classes": [
            "strict_local",
            "local_network_private",
            "remote_private",
            "remote_managed",
        ],
        "status_ref": "architecture/status-model.json#component=engineering-runtime",
    }
    if runtime != expected_runtime:
        failures.append("planned Engineering Runtime contract is incomplete or changed")

    return failures


def main() -> int:
    try:
        matrix = load_matrix()
    except (OSError, json.JSONDecodeError) as error:
        print(f"architecture decision validation failed: {error}", file=sys.stderr)
        return 1

    failures = validate_matrix(matrix)
    if failures:
        for failure in failures:
            print(f"architecture decision validation failed: {failure}", file=sys.stderr)
        return 1

    print("architecture decision validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

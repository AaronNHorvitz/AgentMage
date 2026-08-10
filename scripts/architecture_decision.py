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
    "kernel": ("rust", "cargo"),
    "linux-platform-adapter": ("rust", "cargo"),
    "macos-platform-adapter": ("swift", "swiftpm-xcode"),
    "vscode-extension": ("typescript", "npm"),
    "build-orchestrator": ("rust", "cargo-xtask"),
}
EXPECTED_TARGETS = {
    "macos-arm64": ("aarch64-apple-darwin", "blocked-macos", "blocked-macos"),
    "fedora-x86_64": ("x86_64-unknown-linux-gnu", "selected", "not-run"),
    "ubuntu-x86_64": ("x86_64-unknown-linux-gnu", "selected", "not-run"),
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


def validate_matrix(matrix: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(matrix, dict):
        return ["language/build matrix must be an object"]

    if matrix.get("schema_version") != 1:
        failures.append("schema_version must equal 1")
    if matrix.get("decision_id") != "ADR-0004":
        failures.append("decision_id must equal ADR-0004")
    if matrix.get("status") != "accepted":
        failures.append("architecture decision must be accepted")

    components = _unique_by_id(matrix.get("product_components"), "component", failures)
    if set(components) != set(EXPECTED_COMPONENTS):
        failures.append("product component set does not match ADR-0004")
    for component_id, (language, build_system) in EXPECTED_COMPONENTS.items():
        component = components.get(component_id, {})
        if component.get("language") != language:
            failures.append(f"{component_id} language must be {language}")
        if component.get("build_system") != build_system:
            failures.append(f"{component_id} build system must be {build_system}")
        if not isinstance(component.get("platforms"), list) or not component.get("platforms"):
            failures.append(f"{component_id} must declare at least one platform")

    targets = _unique_by_id(matrix.get("platform_targets"), "platform target", failures)
    if set(targets) != set(EXPECTED_TARGETS):
        failures.append("platform target set does not match ADR-0004")
    for target_id, expected in EXPECTED_TARGETS.items():
        target = targets.get(target_id, {})
        actual = (
            target.get("rust_target"),
            target.get("implementation_status"),
            target.get("verification_status"),
        )
        if actual != expected:
            failures.append(f"{target_id} status or Rust target does not match ADR-0004")

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
        prohibited = set(vscode.get("extension_prohibited_authority", []))
        missing = PROHIBITED_EXTENSION_AUTHORITY - prohibited
        if missing:
            failures.append(
                "VS Code extension missing prohibited authority: " + ", ".join(sorted(missing))
            )

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

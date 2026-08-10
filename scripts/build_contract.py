#!/usr/bin/env python3
"""Validate AgentMage manifests, locks, toolchains, and development commands."""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
CONTRACT_PATH = ROOT / "architecture" / "build-contract.json"

EXPECTED_TOOLCHAINS = {
    "rust": "1.95.0",
    "rust_edition": "2024",
    "cargo_resolver": "3",
    "node": "24.15.0",
    "npm": "11.12.1",
    "typescript": "5.9.3",
    "swift_tools": "6.0",
}
EXPECTED_MEMBERS = {
    "capabilities/read-only",
    "kernel/contracts",
    "kernel/engine",
    "platforms/linux",
    "release/xtask",
    "shells/host",
}
EXPECTED_CARGO_PACKAGES = {
    "capabilities/read-only": (
        "agentmage-capability-read-only",
        {"agentmage-kernel-contracts"},
    ),
    "kernel/contracts": (
        "agentmage-kernel-contracts",
        {"serde", "serde_json"},
    ),
    "kernel/engine": (
        "agentmage-kernel-engine",
        {
            "agentmage-kernel-contracts",
            "ed25519-dalek",
            "serde",
            "serde_json",
            "sha2",
        },
    ),
    "platforms/linux": ("agentmage-platform-linux", {"agentmage-kernel-contracts"}),
    "release/xtask": ("agentmage-xtask", set()),
    "shells/host": (
        "agentmage-host",
        {
            "agentmage-capability-read-only",
            "agentmage-kernel-contracts",
            "agentmage-kernel-engine",
            "agentmage-platform-linux",
        },
    ),
}
EXPECTED_TYPESCRIPT_DEPS = {
    "@eslint/js": "10.0.1",
    "@types/node": "26.2.0",
    "@types/vscode": "1.125.0",
    "eslint": "10.8.1",
    "prettier": "3.9.6",
    "typescript": "5.9.3",
    "typescript-eslint": "8.66.0",
}
EXPECTED_COMMANDS = {
    "build": "npm run product:build",
    "format": "npm run product:format",
    "format_check": "npm run product:format-check",
    "lint": "npm run product:lint",
    "test": "npm run product:test",
    "shared_linux_check": "npm run product:check",
    "macos_build": "npm run product:macos:build",
    "macos_test": "npm run product:macos:test",
}
EXPECTED_SCRIPTS = {
    "product:build": (
        "cargo build --workspace --all-targets --locked && "
        "npm run build --workspace @agentmage/vscode-shell"
    ),
    "product:format": (
        "cargo fmt --all && npm run format --workspace @agentmage/vscode-shell"
    ),
    "product:format-check": (
        "cargo fmt --all -- --check && "
        "npm run format:check --workspace @agentmage/vscode-shell"
    ),
    "product:lint": (
        "cargo clippy --workspace --all-targets --locked -- -D warnings && "
        "npm run lint --workspace @agentmage/vscode-shell"
    ),
    "product:test": (
        "cargo test --workspace --locked && "
        "npm run test --workspace @agentmage/vscode-shell"
    ),
    "product:check": (
        "npm run product:format-check && npm run product:lint && "
        "npm run product:build && npm run product:test"
    ),
    "product:macos:build": "swift build --package-path platforms/macos",
    "product:macos:test": "swift test --package-path platforms/macos",
}
EXPECTED_MACOS_STATUS = {
    "shared_linux_commands": "verified-local",
    "macos_manifest": "interface-only",
    "macos_commands": "blocked-macos",
    "macos_implementation": "blocked-macos",
    "macos_verification": "blocked-macos",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_toml(path: Path) -> dict[str, Any]:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def load_contract(path: Path = CONTRACT_PATH) -> dict[str, Any]:
    return read_json(path)


def _all_cargo_dependencies(manifest: dict[str, Any]) -> set[str]:
    dependencies = set(manifest.get("dependencies", {}))
    for target in manifest.get("target", {}).values():
        if isinstance(target, dict):
            dependencies.update(target.get("dependencies", {}))
    return dependencies


def validate_contract(contract: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(contract, dict):
        return ["build contract must be an object"]
    if contract.get("schema_version") != 1:
        failures.append("schema_version must equal 1")
    if contract.get("decision_id") != "ADR-0004":
        failures.append("decision_id must equal ADR-0004")
    if contract.get("status") != "configured":
        failures.append("build contract status must be configured")
    if contract.get("toolchains") != EXPECTED_TOOLCHAINS:
        failures.append("toolchains do not match the pinned build contract")
    if contract.get("commands") != EXPECTED_COMMANDS:
        failures.append("development commands do not match the build contract")
    if contract.get("platform_status") != EXPECTED_MACOS_STATUS:
        failures.append("platform status must retain the blocked macOS lane")
    if contract.get("end_user_ambient_toolchains") != []:
        failures.append("end users must not require ambient development toolchains")

    required_files = contract.get("required_files")
    if not isinstance(required_files, list) or len(required_files) != len(set(required_files)):
        failures.append("required_files must be a unique array")
        required_files = []
    for relative in required_files:
        if not isinstance(relative, str) or not (root / relative).is_file():
            failures.append(f"missing required build file: {relative}")

    expected_locks = {"Cargo.lock", "package-lock.json", "platforms/macos/Package.resolved"}
    if set(contract.get("lockfiles", [])) != expected_locks:
        failures.append("lockfile set does not match selected build systems")

    try:
        workspace = read_toml(root / "Cargo.toml")
        cargo_lock = read_toml(root / "Cargo.lock")
        cargo_external_catalog = read_json(
            root / "supply-chain/cargo-external-catalog.json"
        )
        root_package = read_json(root / "package.json")
        npm_lock = read_json(root / "package-lock.json")
        swift_lock = read_json(root / "platforms/macos/Package.resolved")
        swift_manifest = (root / "platforms/macos/Package.swift").read_text(encoding="utf-8")
        rust_toolchain = read_toml(root / "rust-toolchain.toml")
    except (OSError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        failures.append(f"cannot parse build input: {error}")
        return failures

    workspace_table = workspace.get("workspace", {})
    workspace_package = workspace_table.get("package", {})
    if set(workspace_table.get("members", [])) != EXPECTED_MEMBERS:
        failures.append("Cargo workspace member set does not match the build contract")
    if workspace_table.get("resolver") != "3":
        failures.append("Cargo resolver must equal 3")
    if workspace_package.get("edition") != "2024":
        failures.append("Cargo workspace edition must equal 2024")
    if workspace_package.get("rust-version") != "1.95.0":
        failures.append("Cargo rust-version must equal 1.95.0")
    if workspace_package.get("license") != "Apache-2.0":
        failures.append("Cargo workspace license must be Apache-2.0")
    if workspace_package.get("publish") is not False:
        failures.append("bootstrap Cargo packages must not be publishable")

    for path, (expected_name, expected_dependencies) in EXPECTED_CARGO_PACKAGES.items():
        try:
            manifest = read_toml(root / path / "Cargo.toml")
        except (OSError, tomllib.TOMLDecodeError) as error:
            failures.append(f"cannot parse {path}/Cargo.toml: {error}")
            continue
        if manifest.get("package", {}).get("name") != expected_name:
            failures.append(f"{path} Cargo package name does not match the build contract")
        actual_dependencies = _all_cargo_dependencies(manifest)
        if actual_dependencies != expected_dependencies:
            failures.append(f"{path} Cargo dependencies do not match the accepted graph")
        if any(section in manifest for section in ("build-dependencies", "dev-dependencies")):
            failures.append(f"{path} contains an undeclared Cargo dependency class")

    lock_packages = cargo_lock.get("package", [])
    workspace_names = {value[0] for value in EXPECTED_CARGO_PACKAGES.values()}
    catalog_keys = {
        (item.get("name"), item.get("version"))
        for item in cargo_external_catalog.get("packages", [])
        if isinstance(item, dict)
    }
    lock_external_keys = {
        (item.get("name"), item.get("version"))
        for item in lock_packages
        if item.get("source") is not None
    }
    if {item.get("name") for item in lock_packages if item.get("source") is None} != workspace_names:
        failures.append("Cargo.lock package closure does not match workspace packages")
    if cargo_external_catalog.get("schema_version") != 1 or catalog_keys != lock_external_keys:
        failures.append("Cargo external catalog does not match the locked package closure")
    for item in lock_packages:
        if item.get("source") is None:
            if "checksum" in item:
                failures.append("workspace Cargo package unexpectedly has a checksum")
        elif (
            item.get("source") != "registry+https://github.com/rust-lang/crates.io-index"
            or not isinstance(item.get("checksum"), str)
        ):
            failures.append("Cargo.lock contains an unapproved external package")

    if root_package.get("packageManager") != "npm@11.12.1":
        failures.append("root package manager must be pinned to npm 11.12.1")
    if root_package.get("workspaces") != ["shells/vscode"]:
        failures.append("root npm workspace set does not match the build contract")
    scripts = root_package.get("scripts", {})
    for name, expected in EXPECTED_SCRIPTS.items():
        if scripts.get(name) != expected:
            failures.append(f"root npm script {name} does not match the build contract")

    try:
        extension_package = read_json(root / "shells/vscode/package.json")
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot parse shells/vscode/package.json: {error}")
        extension_package = {}
    if extension_package.get("devDependencies") != EXPECTED_TYPESCRIPT_DEPS:
        failures.append("VS Code shell dependencies must be exact and complete")
    if extension_package.get("engines", {}).get("vscode") != "^1.125.0":
        failures.append("VS Code engine range must begin at stable 1.125.0")

    locked_root = npm_lock.get("packages", {}).get("", {})
    locked_extension = npm_lock.get("packages", {}).get("shells/vscode", {})
    if locked_root.get("workspaces") != ["shells/vscode"]:
        failures.append("package-lock.json is missing the declared npm workspace")
    if locked_extension.get("devDependencies") != EXPECTED_TYPESCRIPT_DEPS:
        failures.append("package-lock.json does not pin the VS Code shell dependency set")

    if swift_lock != {"pins": [], "version": 2}:
        failures.append("Swift lock must contain no undeclared package pins")
    required_swift_terms = (
        "// swift-tools-version: 6.0",
        'name: "AgentMageMacOSPlatform"',
        ".macOS(.v15)",
        'path: "Sources/AgentMageMacOSPlatform"',
    )
    for term in required_swift_terms:
        if term not in swift_manifest:
            failures.append(f"Swift manifest missing required declaration: {term}")
    if rust_toolchain.get("toolchain") != {
        "channel": "1.95.0",
        "components": ["clippy", "rustfmt"],
        "profile": "minimal",
    }:
        failures.append("rust-toolchain.toml does not match the pinned toolchain")
    return failures


def main() -> int:
    try:
        contract = load_contract()
    except (OSError, json.JSONDecodeError) as error:
        print(f"build contract validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_contract(contract)
    if failures:
        for failure in failures:
            print(f"build contract validation failed: {failure}", file=sys.stderr)
        return 1
    print("build contract validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Enforce separation among AgentMage dependency classes."""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
CLASSES_PATH = ROOT / "architecture" / "dependency-classes.json"
CARGO_MANIFESTS = (
    "capabilities/read-only/Cargo.toml",
    "kernel/contracts/Cargo.toml",
    "kernel/engine/Cargo.toml",
    "platforms/linux/Cargo.toml",
    "release/xtask/Cargo.toml",
    "shells/host/Cargo.toml",
)
EXPECTED_INTERNAL_CARGO = {
    "agentmage-capability-read-only",
    "agentmage-kernel-contracts",
    "agentmage-kernel-engine",
    "agentmage-platform-linux",
}
EXPECTED_EXTERNAL_CARGO = {
    "ed25519-dalek",
    "rustix",
    "serde",
    "serde_json",
    "sha2",
    "unicode-normalization",
}
EXPECTED_ROOT_NPM = {
    "@mermaid-js/mermaid-cli",
    "ajv",
    "ajv-formats",
    "markdownlint-cli2",
}
EXPECTED_VSCODE_DEV = {
    "@eslint/js",
    "@types/node",
    "@types/vscode",
    "eslint",
    "prettier",
    "typescript",
    "typescript-eslint",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_toml(path: Path) -> dict[str, Any]:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def load_classes(path: Path = CLASSES_PATH) -> dict[str, Any]:
    return read_json(path)


def _cargo_dependency_sections(manifest: dict[str, Any]) -> tuple[set[str], set[str], set[str]]:
    production = set(manifest.get("dependencies", {}))
    development = set(manifest.get("dev-dependencies", {}))
    build = set(manifest.get("build-dependencies", {}))
    for target in manifest.get("target", {}).values():
        if not isinstance(target, dict):
            continue
        production.update(target.get("dependencies", {}))
        development.update(target.get("dev-dependencies", {}))
        build.update(target.get("build-dependencies", {}))
    return production, development, build


def validate_classes(record: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(record, dict):
        return ["dependency class record must be an object"]
    if record.get("schema_version") != 1:
        failures.append("schema_version must equal 1")
    if record.get("decision_id") != "ADR-0004":
        failures.append("decision_id must equal ADR-0004")
    if record.get("status") != "enforced":
        failures.append("dependency classes status must be enforced")

    production = record.get("production")
    development = record.get("development")
    packaging = record.get("platform_packaging")
    optional = record.get("optional_later_capabilities")
    if not all(isinstance(item, dict) for item in (production, development, packaging, optional)):
        return [*failures, "every dependency class must be an object"]

    if set(production.get("cargo_internal_path_packages", [])) != EXPECTED_INTERNAL_CARGO:
        failures.append("production Cargo path-package class does not match manifests")
    if set(production.get("cargo_external_packages", [])) != EXPECTED_EXTERNAL_CARGO:
        failures.append("production external Cargo class does not match manifests")
    for key in ("vscode_extension_packages", "swift_packages"):
        if production.get(key) != []:
            failures.append(f"undeclared production dependency class must be empty: {key}")
    if set(development.get("root_npm_packages", [])) != EXPECTED_ROOT_NPM:
        failures.append("root npm development class does not match package.json")
    if set(development.get("vscode_extension_npm_packages", [])) != EXPECTED_VSCODE_DEV:
        failures.append("VS Code npm development class does not match package.json")
    if development.get("cargo_packages") != []:
        failures.append("Cargo development dependency class must be empty")
    if packaging != {
        "linux_packages": [],
        "macos_packages": [],
        "may_enter_product_manifests": False,
    }:
        failures.append("platform packaging dependencies must remain isolated and empty")
    if optional != {
        "packages": [],
        "enabled": False,
        "included_in_default_build": False,
        "included_in_release": False,
    }:
        failures.append("later-capability dependencies must remain absent and disabled")

    try:
        root_package = read_json(root / "package.json")
        extension_package = read_json(root / "shells/vscode/package.json")
        npm_lock = read_json(root / "package-lock.json")
        swift_manifest = (root / "platforms/macos/Package.swift").read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot parse dependency manifest: {error}")
        return failures

    if set(root_package.get("devDependencies", {})) != EXPECTED_ROOT_NPM:
        failures.append("root npm development dependencies drifted")
    for section in ("dependencies", "optionalDependencies", "bundledDependencies"):
        if root_package.get(section) not in (None, {}, []):
            failures.append(f"root package contains prohibited {section}")
    if root_package.get("private") is not True:
        failures.append("root development package must remain private")

    if set(extension_package.get("devDependencies", {})) != EXPECTED_VSCODE_DEV:
        failures.append("VS Code shell development dependencies drifted")
    for section in ("dependencies", "optionalDependencies", "bundledDependencies"):
        if extension_package.get(section) not in (None, {}, []):
            failures.append(f"VS Code shell contains undeclared {section}")

    locked_root = npm_lock.get("packages", {}).get("", {})
    locked_extension = npm_lock.get("packages", {}).get("shells/vscode", {})
    if set(locked_root.get("devDependencies", {})) != EXPECTED_ROOT_NPM:
        failures.append("root npm lock development class drifted")
    if set(locked_extension.get("devDependencies", {})) != EXPECTED_VSCODE_DEV:
        failures.append("VS Code npm lock development class drifted")

    cargo_production: set[str] = set()
    for relative in CARGO_MANIFESTS:
        try:
            manifest = read_toml(root / relative)
        except (OSError, tomllib.TOMLDecodeError) as error:
            failures.append(f"cannot parse {relative}: {error}")
            continue
        production_dependencies, development_dependencies, build_dependencies = (
            _cargo_dependency_sections(manifest)
        )
        cargo_production.update(production_dependencies)
        if development_dependencies:
            failures.append(f"{relative} contains undeclared Cargo development dependencies")
        if build_dependencies:
            failures.append(f"{relative} contains undeclared Cargo build dependencies")
    if cargo_production != EXPECTED_INTERNAL_CARGO | EXPECTED_EXTERNAL_CARGO:
        failures.append("Cargo production dependency class drifted")

    if ".package(" in swift_manifest:
        failures.append("Swift manifest contains an undeclared external dependency")
    return failures


def main() -> int:
    try:
        record = load_classes()
    except (OSError, json.JSONDecodeError) as error:
        print(f"dependency class validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_classes(record)
    if failures:
        for failure in failures:
            print(f"dependency class validation failed: {failure}", file=sys.stderr)
        return 1
    print("dependency class validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

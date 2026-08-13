#!/usr/bin/env python3
"""Fail closed when normal product source gains an undeclared network path."""

from __future__ import annotations

import hashlib
import json
import re
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "security" / "strict-local-source-policy.json"
SOURCE_SUFFIXES = {".css", ".html", ".js", ".json", ".mjs", ".rs", ".ts"}
URI = re.compile(r"(?:https?|wss?)://[^\s\"'<>`)]+", re.IGNORECASE)
TOP_LEVEL_KEYS = {
    "allowed_external_uris",
    "allowed_first_party_build_scripts",
    "approved_cargo_manifests",
    "approved_cargo_packages",
    "denied_npm_runtime_packages",
    "denied_rust_packages",
    "exact_source_fragments",
    "scan_roots",
    "schema_version",
    "symbol_rules",
    "vscode_manifest_profile",
}


class StrictLocalSourceAuditError(ValueError):
    """Raised when the strict-local source policy or source tree is invalid."""


def load_policy(path: Path = POLICY_PATH) -> dict[str, Any]:
    try:
        policy = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as failure:
        raise StrictLocalSourceAuditError("strict-local source policy is unreadable") from failure
    if not isinstance(policy, dict) or set(policy) != TOP_LEVEL_KEYS:
        raise StrictLocalSourceAuditError("strict-local source policy fields are not closed")
    if policy.get("schema_version") != 1:
        raise StrictLocalSourceAuditError("strict-local source policy version is unsupported")
    build_scripts = policy.get("allowed_first_party_build_scripts")
    if (
        not isinstance(build_scripts, list)
        or any(not isinstance(item, str) or not item for item in build_scripts)
        or build_scripts != sorted(set(build_scripts))
    ):
        raise StrictLocalSourceAuditError("strict-local build-script policy is invalid")
    manifests = policy.get("approved_cargo_manifests")
    if (
        not isinstance(manifests, dict)
        or not manifests
        or list(manifests) != sorted(manifests)
        or any(
            not isinstance(path, str)
            or not path.endswith("Cargo.toml")
            or re.fullmatch(r"[0-9a-f]{64}", digest) is None
            for path, digest in manifests.items()
        )
    ):
        raise StrictLocalSourceAuditError("strict-local Cargo manifest policy is invalid")
    for key in (
        "approved_cargo_packages",
        "denied_npm_runtime_packages",
        "denied_rust_packages",
        "scan_roots",
    ):
        value = policy.get(key)
        if (
            not isinstance(value, list)
            or not value
            or any(not isinstance(item, str) or not item for item in value)
            or value != sorted(set(value))
        ):
            raise StrictLocalSourceAuditError(f"strict-local policy list is invalid: {key}")
    manifest = policy.get("vscode_manifest_profile")
    if not isinstance(manifest, dict) or set(manifest) != {
        "activation_events",
        "allowed_top_level_keys",
        "chat_provider_keys",
        "contribution_keys",
        "extension_kind",
        "main",
        "runtime_packages",
        "scripts",
    }:
        raise StrictLocalSourceAuditError("strict-local VS Code manifest profile is invalid")
    for key in (
        "activation_events",
        "allowed_top_level_keys",
        "chat_provider_keys",
        "contribution_keys",
        "extension_kind",
        "runtime_packages",
    ):
        value = manifest.get(key)
        if (
            not isinstance(value, list)
            or any(not isinstance(item, str) or not item for item in value)
            or value != sorted(set(value))
        ):
            raise StrictLocalSourceAuditError(
                f"strict-local VS Code manifest list is invalid: {key}"
            )
    if not isinstance(manifest.get("main"), str) or not manifest["main"]:
        raise StrictLocalSourceAuditError("strict-local VS Code entry point is invalid")
    scripts = manifest.get("scripts")
    if (
        not isinstance(scripts, dict)
        or not scripts
        or any(
            not isinstance(key, str)
            or not key
            or not isinstance(value, str)
            or not value
            for key, value in scripts.items()
        )
    ):
        raise StrictLocalSourceAuditError("strict-local VS Code scripts are invalid")
    rules = policy.get("symbol_rules")
    if not isinstance(rules, list) or not rules:
        raise StrictLocalSourceAuditError("strict-local symbol rules are absent")
    observed_ids: set[str] = set()
    for rule in rules:
        if not isinstance(rule, dict) or set(rule) != {"allowed_paths", "id", "pattern"}:
            raise StrictLocalSourceAuditError("strict-local symbol rule fields are invalid")
        identifier = rule.get("id")
        paths = rule.get("allowed_paths")
        pattern = rule.get("pattern")
        if (
            not isinstance(identifier, str)
            or not identifier
            or identifier in observed_ids
            or not isinstance(paths, list)
            or any(not isinstance(item, str) or not item for item in paths)
            or paths != sorted(set(paths))
            or not isinstance(pattern, str)
            or not pattern
        ):
            raise StrictLocalSourceAuditError("strict-local symbol rule value is invalid")
        try:
            re.compile(pattern)
        except re.error as failure:
            raise StrictLocalSourceAuditError(
                "strict-local symbol rule expression is invalid"
            ) from failure
        observed_ids.add(identifier)
    fragments = policy.get("exact_source_fragments")
    if not isinstance(fragments, list) or not fragments:
        raise StrictLocalSourceAuditError("strict-local exact source fragments are absent")
    fragment_ids: set[str] = set()
    for rule in fragments:
        if not isinstance(rule, dict) or set(rule) != {"count", "fragment", "id", "path"}:
            raise StrictLocalSourceAuditError("strict-local exact source fragment fields are invalid")
        identifier = rule.get("id")
        path = rule.get("path")
        fragment = rule.get("fragment")
        count = rule.get("count")
        if (
            not isinstance(identifier, str)
            or not identifier
            or identifier in fragment_ids
            or not isinstance(path, str)
            or not path
            or not isinstance(fragment, str)
            or not fragment
            or not isinstance(count, int)
            or isinstance(count, bool)
            or count < 1
        ):
            raise StrictLocalSourceAuditError("strict-local exact source fragment value is invalid")
        fragment_ids.add(identifier)
    if [rule["id"] for rule in fragments] != sorted(fragment_ids):
        raise StrictLocalSourceAuditError("strict-local exact source fragments are not sorted")
    allowed_uris = policy.get("allowed_external_uris")
    if not isinstance(allowed_uris, dict):
        raise StrictLocalSourceAuditError("strict-local URI allowances are invalid")
    for path_name, values in allowed_uris.items():
        if (
            not isinstance(path_name, str)
            or not path_name
            or not isinstance(values, list)
            or not values
            or any(not isinstance(item, str) or URI.fullmatch(item) is None for item in values)
            or values != sorted(set(values))
        ):
            raise StrictLocalSourceAuditError("strict-local URI allowance is invalid")
    return policy


def source_map(policy: dict[str, Any]) -> dict[str, str]:
    sources: dict[str, str] = {}
    for relative_root in policy["scan_roots"]:
        root = ROOT / relative_root
        if not root.is_dir():
            raise StrictLocalSourceAuditError(f"strict-local scan root is absent: {relative_root}")
        for path in sorted(root.rglob("*")):
            if path.is_file() and path.suffix in SOURCE_SUFFIXES:
                relative = path.relative_to(ROOT).as_posix()
                try:
                    sources[relative] = path.read_text(encoding="utf-8")
                except UnicodeError as failure:
                    raise StrictLocalSourceAuditError(
                        f"strict-local source is not UTF-8: {relative}"
                    ) from failure
    if not sources:
        raise StrictLocalSourceAuditError("strict-local source closure is empty")
    return sources


def production_source(path: str, content: str) -> str:
    """Exclude one terminal Rust unit-test module from product-source scans."""

    if not path.endswith(".rs"):
        return content
    test_modules = list(
        re.finditer(r"(?m)^#\[cfg\(test\)\]\s*\nmod\s+tests\s*\{", content)
    )
    return content[: test_modules[-1].start()] if test_modules else content


def scan_sources(policy: dict[str, Any], sources: dict[str, str]) -> list[str]:
    failures: list[str] = []
    allowed_uris = policy["allowed_external_uris"]
    observed_allowed: dict[str, set[str]] = {path: set() for path in allowed_uris}
    for path, content in sorted(sources.items()):
        for match in URI.finditer(content):
            uri = match.group(0)
            if uri not in allowed_uris.get(path, []):
                failures.append(f"undeclared external URI in product source: {path}")
            else:
                observed_allowed[path].add(uri)
        product_content = production_source(path, content)
        for rule in policy["symbol_rules"]:
            if re.search(rule["pattern"], product_content) and path not in rule["allowed_paths"]:
                failures.append(f"{rule['id']} found outside its closed allowlist: {path}")
    for path, expected in allowed_uris.items():
        if path not in sources:
            failures.append(f"URI allowance path is outside the source closure: {path}")
        elif observed_allowed[path] != set(expected):
            failures.append(f"URI allowance is stale or incomplete: {path}")
    for rule in policy["exact_source_fragments"]:
        content = sources.get(rule["path"])
        if content is None:
            failures.append(f"exact source fragment path is outside the source closure: {rule['id']}")
        elif content.count(rule["fragment"]) != rule["count"]:
            failures.append(f"exact source fragment count changed: {rule['id']}")
    return sorted(set(failures))


def cargo_runtime_packages() -> set[str]:
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    packages = lock.get("package")
    if not isinstance(packages, list):
        raise StrictLocalSourceAuditError("Cargo lock package closure is invalid")
    identities: set[str] = set()
    for package in packages:
        if not isinstance(package, dict):
            raise StrictLocalSourceAuditError("Cargo lock package entry is invalid")
        name = package.get("name")
        version = package.get("version")
        if not isinstance(name, str) or not name or not isinstance(version, str) or not version:
            raise StrictLocalSourceAuditError("Cargo lock package identity is invalid")
        identity = f"{name}@{version}"
        if identity in identities:
            raise StrictLocalSourceAuditError("Cargo lock package identity is duplicated")
        identities.add(identity)
    return identities


def npm_runtime_packages() -> set[str]:
    manifest = json.loads((ROOT / "shells/vscode/package.json").read_text(encoding="utf-8"))
    if not isinstance(manifest, dict):
        raise StrictLocalSourceAuditError("VS Code package manifest is invalid")
    runtime = manifest.get("dependencies", {})
    optional = manifest.get("optionalDependencies", {})
    if not isinstance(runtime, dict) or not isinstance(optional, dict):
        raise StrictLocalSourceAuditError("VS Code runtime dependency maps are invalid")
    names = set(runtime) | set(optional)
    if any(not isinstance(name, str) or not name for name in names):
        raise StrictLocalSourceAuditError("VS Code runtime package name is invalid")
    return names


def npm_lock_runtime_packages() -> set[str]:
    lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
    if not isinstance(lock, dict) or not isinstance(lock.get("packages"), dict):
        raise StrictLocalSourceAuditError("npm lock package closure is invalid")
    package = lock["packages"].get("shells/vscode")
    if not isinstance(package, dict):
        raise StrictLocalSourceAuditError("VS Code lock manifest is absent")
    runtime = package.get("dependencies", {})
    optional = package.get("optionalDependencies", {})
    if not isinstance(runtime, dict) or not isinstance(optional, dict):
        raise StrictLocalSourceAuditError("VS Code lock runtime maps are invalid")
    return set(runtime) | set(optional)


def audit_cargo_manifests(
    policy: dict[str, Any],
    content_overrides: dict[str, bytes] | None = None,
    observed_build_scripts: list[str] | None = None,
) -> list[str]:
    failures: list[str] = []
    overrides = {} if content_overrides is None else content_overrides
    for path, expected_sha256 in policy["approved_cargo_manifests"].items():
        try:
            content = overrides.get(path, (ROOT / path).read_bytes())
        except OSError as failure:
            raise StrictLocalSourceAuditError("approved Cargo manifest is unavailable") from failure
        if hashlib.sha256(content).hexdigest() != expected_sha256:
            failures.append(f"reviewed Cargo manifest changed: {path}")
    if observed_build_scripts is None:
        observed_build_scripts = sorted(
            path.relative_to(ROOT).as_posix()
            for manifest in policy["approved_cargo_manifests"]
            if manifest != "Cargo.toml"
            for path in [(ROOT / manifest).parent / "build.rs"]
            if path.is_file()
        )
    if observed_build_scripts != policy["allowed_first_party_build_scripts"]:
        failures.append("first-party Cargo build-script surface changed")
    return failures


def audit_vscode_manifest(
    policy: dict[str, Any],
    manifest: dict[str, Any] | None = None,
    locked_runtime: set[str] | None = None,
) -> list[str]:
    if manifest is None:
        manifest = json.loads(
            (ROOT / "shells/vscode/package.json").read_text(encoding="utf-8")
        )
    if not isinstance(manifest, dict):
        raise StrictLocalSourceAuditError("VS Code package manifest is invalid")
    profile = policy["vscode_manifest_profile"]
    failures: list[str] = []
    if sorted(manifest) != profile["allowed_top_level_keys"]:
        failures.append("VS Code manifest surface changed")
    if manifest.get("activationEvents") != profile["activation_events"]:
        failures.append("VS Code activation events changed")
    if manifest.get("extensionKind") != profile["extension_kind"]:
        failures.append("VS Code extension execution location changed")
    if manifest.get("main") != profile["main"]:
        failures.append("VS Code shipped entry point changed")
    if manifest.get("scripts") != profile["scripts"]:
        failures.append("VS Code package scripts changed")
    contributes = manifest.get("contributes")
    if not isinstance(contributes, dict) or sorted(contributes) != profile["contribution_keys"]:
        failures.append("VS Code contribution surface changed")
    else:
        providers = contributes.get("languageModelChatProviders")
        if (
            not isinstance(providers, list)
            or len(providers) != 1
            or not isinstance(providers[0], dict)
            or sorted(providers[0]) != profile["chat_provider_keys"]
            or any(not isinstance(value, str) or not value for value in providers[0].values())
        ):
            failures.append("VS Code chat-provider declaration changed")
    observed = npm_runtime_packages()
    locked = npm_lock_runtime_packages() if locked_runtime is None else locked_runtime
    if sorted(observed) != profile["runtime_packages"] or observed != locked:
        failures.append("VS Code runtime package closure changed")
    return failures


def audit(policy: dict[str, Any], sources: dict[str, str]) -> list[str]:
    failures = scan_sources(policy, sources)
    failures.extend(audit_cargo_manifests(policy))
    denied_rust = set(policy["denied_rust_packages"])
    observed_rust = cargo_runtime_packages()
    observed_rust_names = {identity.rsplit("@", 1)[0] for identity in observed_rust}
    if denied_rust & observed_rust_names:
        failures.append("denied network-capable Rust dependency is present")
    if observed_rust != set(policy["approved_cargo_packages"]):
        failures.append("reviewed Cargo package closure changed")
    denied_npm = set(policy["denied_npm_runtime_packages"])
    observed_npm = npm_runtime_packages()
    if denied_npm & observed_npm:
        failures.append("denied network-capable npm runtime dependency is present")
    if observed_npm:
        failures.append("VS Code shell has undeclared runtime dependencies")
    failures.extend(audit_vscode_manifest(policy))
    return sorted(set(failures))


def main() -> int:
    try:
        policy = load_policy()
        failures = audit(policy, source_map(policy))
    except (OSError, UnicodeError, json.JSONDecodeError, tomllib.TOMLDecodeError, StrictLocalSourceAuditError) as failure:
        print(f"strict-local source audit failed: {failure}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"strict-local source audit failed: {failure}", file=sys.stderr)
        return 1
    print("Strict-local source audit passed with zero undeclared network paths.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

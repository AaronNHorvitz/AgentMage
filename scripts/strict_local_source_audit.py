#!/usr/bin/env python3
"""Fail closed when normal product source gains an undeclared network path."""

from __future__ import annotations

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
    "denied_npm_runtime_packages",
    "denied_rust_packages",
    "scan_roots",
    "schema_version",
    "symbol_rules",
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
    for key in ("denied_npm_runtime_packages", "denied_rust_packages", "scan_roots"):
        value = policy.get(key)
        if (
            not isinstance(value, list)
            or not value
            or any(not isinstance(item, str) or not item for item in value)
            or value != sorted(set(value))
        ):
            raise StrictLocalSourceAuditError(f"strict-local policy list is invalid: {key}")
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
        for rule in policy["symbol_rules"]:
            if re.search(rule["pattern"], content) and path not in rule["allowed_paths"]:
                failures.append(f"{rule['id']} found outside its closed allowlist: {path}")
    for path, expected in allowed_uris.items():
        if path not in sources:
            failures.append(f"URI allowance path is outside the source closure: {path}")
        elif observed_allowed[path] != set(expected):
            failures.append(f"URI allowance is stale or incomplete: {path}")
    return sorted(set(failures))


def cargo_runtime_packages() -> set[str]:
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    packages = lock.get("package")
    if not isinstance(packages, list):
        raise StrictLocalSourceAuditError("Cargo lock package closure is invalid")
    names = {package.get("name") for package in packages if isinstance(package, dict)}
    if None in names or any(not isinstance(name, str) for name in names):
        raise StrictLocalSourceAuditError("Cargo lock package name is invalid")
    return names


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


def audit(policy: dict[str, Any], sources: dict[str, str]) -> list[str]:
    failures = scan_sources(policy, sources)
    denied_rust = set(policy["denied_rust_packages"])
    observed_rust = cargo_runtime_packages()
    if denied_rust & observed_rust:
        failures.append("denied network-capable Rust dependency is present")
    denied_npm = set(policy["denied_npm_runtime_packages"])
    observed_npm = npm_runtime_packages()
    if denied_npm & observed_npm:
        failures.append("denied network-capable npm runtime dependency is present")
    if observed_npm:
        failures.append("VS Code shell has undeclared runtime dependencies")
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

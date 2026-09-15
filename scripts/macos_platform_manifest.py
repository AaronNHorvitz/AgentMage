#!/usr/bin/env python3
"""Validate the macOS release-manifest field freeze fixture."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
FIELD_FREEZE_PATH = ROOT / "release/platform-manifests/v2/field-freeze/macos-aarch64.json"

TOP_LEVEL_FIELDS = {
    "schema_version",
    "record_type",
    "manifest_id",
    "status",
    "adapter_api_version",
    "platform_family",
    "architecture",
    "platform_build",
    "product_toolchain",
    "vscode",
    "team_id",
    "app_group",
    "bundle_ids",
    "designated_requirements",
    "entitlements",
    "helper_hashes",
    "package",
    "credential_values_present",
    "private_environment_values_present",
    "linux_evidence_substituted",
    "release_claim",
}

HELPERS = ("kernel_host", "vscode_bridge", "xpc_tool_helper", "metal_inference")

BUNDLE_IDS = {
    "kernel_host": "com.agentmage.kernel",
    "vscode_bridge": "com.agentmage.vscode-bridge",
    "xpc_tool_helper": "com.agentmage.xpc-tool-helper",
    "metal_inference": "com.agentmage.metal-inference",
}

HASH = re.compile(r"^[0-9a-f]{64}$")
TEAM_ID = re.compile(r"^[A-Z0-9]{10}$")
BUNDLE_ID = re.compile(r"^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)+$")
VSCODE_COMMIT = re.compile(r"^[0-9a-f]{40}$")

FORBIDDEN_KEYS = {
    "api_key",
    "home_directory",
    "hostname",
    "notarization_ticket",
    "password",
    "private_key",
    "secret",
    "signature",
    "signed_at",
    "signer_identity",
    "token",
    "username",
}


class MacosManifestError(ValueError):
    """Raised when the macOS field freeze is stale, malformed, or overclaims."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n").encode()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def nested_keys(value: Any) -> set[str]:
    keys: set[str] = set()
    if isinstance(value, dict):
        for key, item in value.items():
            keys.add(key)
            keys.update(nested_keys(item))
    elif isinstance(value, list):
        for item in value:
            keys.update(nested_keys(item))
    return keys


def validate_identity_object(
    value: Any, required: set[str], hash_key: str, failures: list[str], label: str
) -> None:
    if not isinstance(value, dict) or set(value) != required:
        failures.append(f"{label}: fields changed")
        return
    if not HASH.fullmatch(str(value.get(hash_key))):
        failures.append(f"{label}: {hash_key} is not a lowercase 64-character SHA-256")


def validate_field_freeze(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["field freeze must be an object"]
    if set(value) != TOP_LEVEL_FIELDS:
        return ["field freeze top-level fields changed"]

    if (
        value["schema_version"] != 2
        or value["record_type"] != "platform-release-manifest-field-freeze"
        or value["manifest_id"] != "agentmage-macos-aarch64-v2-field-freeze"
        or value["status"] != "frozen-field-specification"
        or value["adapter_api_version"] != 1
        or value["platform_family"] != "macos-apple-silicon"
        or value["architecture"] != "aarch64"
    ):
        failures.append("field freeze identity changed")

    validate_identity_object(
        value.get("platform_build"), {"id", "sha256"}, "sha256", failures, "platform_build"
    )
    validate_identity_object(
        value.get("product_toolchain"),
        {"id", "sha256"},
        "sha256",
        failures,
        "product_toolchain",
    )

    vscode = value.get("vscode")
    if not isinstance(vscode, dict) or set(vscode) != {"version", "commit", "sha256"}:
        failures.append("vscode: fields changed")
    else:
        if not isinstance(vscode.get("version"), str) or not vscode["version"]:
            failures.append("vscode: version is malformed")
        if not VSCODE_COMMIT.fullmatch(str(vscode.get("commit"))):
            failures.append("vscode: commit is not a 40-character lowercase hex hash")
        if not HASH.fullmatch(str(vscode.get("sha256"))):
            failures.append("vscode: sha256 is not a lowercase 64-character SHA-256")

    team_id = value.get("team_id")
    if not isinstance(team_id, str) or not TEAM_ID.fullmatch(team_id):
        failures.append("team_id: must be exactly ten uppercase ASCII alphanumerics")

    app_group = value.get("app_group")
    if (
        not isinstance(app_group, str)
        or not isinstance(team_id, str)
        or not TEAM_ID.fullmatch(team_id or "")
        or not app_group.startswith(f"{team_id}.")
        or not BUNDLE_ID.fullmatch(app_group[len(team_id) + 1 :] or "")
    ):
        failures.append("app_group: must be '<team_id>.<reverse-dns>'")

    bundle_ids = value.get("bundle_ids")
    if not isinstance(bundle_ids, dict) or tuple(sorted(bundle_ids)) != tuple(sorted(HELPERS)):
        failures.append("bundle_ids: helper set changed")
    else:
        for helper in HELPERS:
            actual = bundle_ids.get(helper)
            if actual != BUNDLE_IDS[helper] or not BUNDLE_ID.fullmatch(str(actual)):
                failures.append(f"bundle_ids: {helper} identifier changed")

    designated = value.get("designated_requirements")
    if not isinstance(designated, dict) or set(designated) != set(HELPERS):
        failures.append("designated_requirements: helper set changed")
    else:
        for helper in HELPERS:
            requirement = designated.get(helper)
            expected_identifier = f'identifier "{BUNDLE_IDS[helper]}"'
            expected_team = f'certificate leaf[subject.OU] = "{team_id}"'
            if (
                not isinstance(requirement, str)
                or expected_identifier not in requirement
                or "anchor apple generic" not in requirement
                or expected_team not in requirement
            ):
                failures.append(f"designated_requirements: {helper} does not bind bundle and team")

    entitlements = value.get("entitlements")
    if not isinstance(entitlements, dict) or set(entitlements) != set(HELPERS):
        failures.append("entitlements: helper set changed")
    else:
        for helper in HELPERS:
            entries = entitlements.get(helper)
            if (
                not isinstance(entries, list)
                or not entries
                or entries != sorted(set(entries))
                or any(not isinstance(entry, str) or not entry for entry in entries)
            ):
                failures.append(f"entitlements: {helper} list is not a sorted non-empty set")

    helper_hashes = value.get("helper_hashes")
    if not isinstance(helper_hashes, dict) or set(helper_hashes) != set(HELPERS):
        failures.append("helper_hashes: helper set changed")
    else:
        seen: set[str] = set()
        for helper in HELPERS:
            digest = helper_hashes.get(helper)
            if not HASH.fullmatch(str(digest)):
                failures.append(f"helper_hashes: {helper} is not a lowercase 64-character SHA-256")
            elif digest in seen:
                failures.append(f"helper_hashes: {helper} reuses another helper's digest")
            else:
                seen.add(digest)

    package = value.get("package")
    if not isinstance(package, dict) or set(package) != {"format", "sha256", "identity_class"}:
        failures.append("package: fields changed")
    else:
        if package.get("format") != "pkg":
            failures.append("package: format must be 'pkg'")
        if not HASH.fullmatch(str(package.get("sha256"))):
            failures.append("package: sha256 is not a lowercase 64-character SHA-256")
        if package.get("identity_class") != "synthetic-field-freeze-fixture":
            failures.append("package: identity_class must be 'synthetic-field-freeze-fixture'")

    if (
        value.get("credential_values_present") is not False
        or value.get("private_environment_values_present") is not False
        or value.get("linux_evidence_substituted") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("field freeze made an unsupported or private-data claim")

    if nested_keys(value) & FORBIDDEN_KEYS:
        failures.append("field freeze contains a forbidden private or signed-release field")

    return failures


def check_field_freeze(path: Path = FIELD_FREEZE_PATH) -> None:
    value = read_json(path)
    failures = validate_field_freeze(value)
    if failures:
        raise MacosManifestError("; ".join(failures))
    canonical_first = canonical_json(value)
    canonical_second = canonical_json(read_json(path))
    if canonical_first != canonical_second:
        raise MacosManifestError("field freeze canonicalization is not stable")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--path", default=str(FIELD_FREEZE_PATH))
    args = parser.parse_args()
    try:
        check_field_freeze(Path(args.path))
    except (OSError, UnicodeError, MacosManifestError, json.JSONDecodeError) as error:
        print(f"macOS field freeze failed: {error}", file=sys.stderr)
        return 1
    print("macOS release-manifest field freeze passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

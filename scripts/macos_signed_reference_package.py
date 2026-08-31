#!/usr/bin/env python3
"""Validate external signed macOS reference-package evidence or its source contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.macos_release_runner_source_contract import (  # noqa: E402
    COMPONENTS,
    EXPECTED_ENTITLEMENTS,
    POLICY_KEYS,
)


REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-signed-reference-package-source-contract.json"
)
SOURCE_PATHS = (
    "packaging/macos/REFERENCE-PACKAGE.md",
    "schemas/platform/macos-release-manifest.schema.json",
    "scripts/macos_signed_reference_package.py",
    "tests/test_macos_signed_reference_package.py",
)
MANIFEST_KEYS = {
    "schema_version",
    "record_type",
    "manifest_id",
    "status",
    "identity_class",
    "platform",
    "toolchain",
    "vscode",
    "code_identity",
    "entitlements",
    "component_hashes",
    "package",
    "credential_values_present",
    "private_environment_values_present",
    "macos_execution_performed",
    "release_claim",
}
TERMINAL_KEYS = {
    "schema_version",
    "record_type",
    "source_revision",
    "version",
    "macos_build",
    "xcode_build",
    "architecture",
    "package_sha256",
    "notary_status",
    "staple_valid",
    "gatekeeper_install_accepted",
    "candidate_install",
    "candidate_launch",
    "candidate_uninstall",
    "rollback_install",
    "rollback_launch",
    "credential_values_present",
    "release_claim",
}
SHA256 = re.compile(r"[0-9a-f]{64}")
SHA1 = re.compile(r"[0-9A-Fa-f]{40}")
REVISION = re.compile(r"[0-9a-f]{40}")
VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
PROHIBITED_KEYS = {
    "password",
    "private_key",
    "api_key",
    "apple_id",
    "issuer_id",
    "auth_token",
    "authorization",
    "environment",
}


def valid_sha256(value: Any) -> bool:
    return isinstance(value, str) and SHA256.fullmatch(value) is not None and value != "0" * 64


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def read_closed_json(path: Path, expected: set[str], maximum: int) -> tuple[Any, list[str]]:
    failures: list[str] = []
    try:
        metadata = path.lstat()
        if (
            not stat.S_ISREG(metadata.st_mode)
            or metadata.st_nlink != 1
            or metadata.st_uid != os.geteuid()
            or metadata.st_mode & 0o022
            or metadata.st_size > maximum
        ):
            return None, [f"unsafe evidence file: {path.name}"]
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeError) as error:
        return None, [f"unreadable evidence file {path.name}: {error}"]
    if not isinstance(value, dict) or set(value) != expected:
        failures.append(f"closed evidence fields changed: {path.name}")
    return value, failures


def contains_prohibited_key(value: Any) -> bool:
    if isinstance(value, dict):
        return any(
            str(key).lower() in PROHIBITED_KEYS or contains_prohibited_key(item)
            for key, item in value.items()
        )
    if isinstance(value, list):
        return any(contains_prohibited_key(item) for item in value)
    return False


def validate_release_policy(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict) or set(value) != POLICY_KEYS:
        return ["release policy field closure changed"]
    expected = {
        "schema_version": 1,
        "record_type": "macos-release-runner-policy",
        "status": "release-approved",
        "architecture": "arm64",
        "notary_keychain_profile": "agentmage-release-notary-v1",
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "authorized-candidate",
    }
    for key, item in expected.items():
        if value.get(key) != item:
            failures.append(f"release policy disposition invalid: {key}")
    if not REVISION.fullmatch(str(value.get("source_revision", ""))):
        failures.append("release policy source revision invalid")
    if not VERSION.fullmatch(str(value.get("version", ""))):
        failures.append("release policy version invalid")
    team_id = value.get("team_id")
    if not isinstance(team_id, str) or not re.fullmatch(r"[A-Z0-9]{10}", team_id) or team_id == "AAAAAAAAAA":
        failures.append("release policy Team ID is not release-derived")
    for key in ("application_identity_sha1", "installer_identity_sha1"):
        if not SHA1.fullmatch(str(value.get(key, ""))):
            failures.append(f"release policy certificate fingerprint invalid: {key}")
    if value.get("application_identity_sha1") == value.get("installer_identity_sha1"):
        failures.append("release policy certificate classes collide")
    bundles = value.get("bundle_identifiers")
    if not isinstance(bundles, dict) or set(bundles) != set(COMPONENTS):
        failures.append("release policy component closure changed")
    else:
        bundle_values = list(bundles.values())
        if len(set(bundle_values)) != len(COMPONENTS) or any(
            not isinstance(item, str)
            or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.-]{7,191}", item)
            or ".example." in item
            for item in bundle_values
        ):
            failures.append("release policy bundle identities are not release-derived")
    if value.get("entitlements") != EXPECTED_ENTITLEMENTS:
        failures.append("release policy entitlement closure changed")
    app_group = value.get("app_group_identifier")
    if not isinstance(app_group, str) or not app_group.startswith("group.") or ".example." in app_group:
        failures.append("release policy App Group is not release-derived")
    keychain_group = value.get("keychain_access_group")
    if not isinstance(keychain_group, str) or not isinstance(team_id, str) or not keychain_group.startswith(f"{team_id}."):
        failures.append("release policy Keychain group is not Team-bound")
    if not valid_sha256(value.get("previous_package_sha256")):
        failures.append("release policy prior-package digest invalid")
    if not isinstance(value.get("previous_package_path"), str) or not value["previous_package_path"].startswith("/"):
        failures.append("release policy prior-package path invalid")
    if contains_prohibited_key(value):
        failures.append("release policy contains credential or environment material")
    return failures


def validate_manifest(value: Any, policy: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict) or set(value) != MANIFEST_KEYS:
        return ["release manifest field closure changed"]
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "macos-release-manifest"
        or value.get("status") != "signed-release"
        or value.get("identity_class") != "release-derived"
        or value.get("credential_values_present") is not False
        or value.get("private_environment_values_present") is not False
        or value.get("macos_execution_performed") is not True
        or value.get("release_claim") != "signed-package-candidate"
    ):
        failures.append("release manifest disposition is invalid")
    if not isinstance(value.get("manifest_id"), str) or not re.fullmatch(
        r"[a-z0-9][a-z0-9._-]{7,127}", value["manifest_id"]
    ):
        failures.append("release manifest identifier is invalid")
    platform = value.get("platform", {})
    if (
        not isinstance(platform, dict)
        or set(platform) != {"family", "minimum_macos_version", "tested_macos_build", "architecture"}
        or platform.get("family") != "macos"
        or platform.get("architecture") != "arm64"
        or not re.fullmatch(r"[0-9]{1,2}\.[0-9]{1,2}(?:\.[0-9]{1,2})?", str(platform.get("minimum_macos_version", "")))
        or set(platform.get("tested_macos_build", {})) != {"id", "sha256"}
        or platform.get("tested_macos_build", {}).get("id") != policy.get("expected_macos_build")
        or not valid_sha256(platform.get("tested_macos_build", {}).get("sha256"))
    ):
        failures.append("release manifest platform identity is invalid")
    code_identity = value.get("code_identity", {})
    if (
        not isinstance(code_identity, dict)
        or set(code_identity)
        != {"team_id", "bundle_identifiers", "app_group_identifier", "designated_requirements"}
        or code_identity.get("team_id") != policy.get("team_id")
        or code_identity.get("bundle_identifiers") != policy.get("bundle_identifiers")
        or code_identity.get("app_group_identifier") != policy.get("app_group_identifier")
    ):
        failures.append("release manifest code identity does not match policy")
    requirements = code_identity.get("designated_requirements", {})
    bundles = policy.get("bundle_identifiers", {})
    if not isinstance(requirements, dict) or set(requirements) != set(COMPONENTS):
        failures.append("release manifest designated-requirement closure changed")
    else:
        for component in COMPONENTS:
            requirement = requirements.get(component)
            if (
                not isinstance(requirement, str)
                or f"identifier {bundles.get(component)}" not in requirement
                or f"certificate leaf[subject.OU] = {policy.get('team_id')}" not in requirement
            ):
                failures.append(f"release manifest designated requirement invalid: {component}")
    if value.get("entitlements") != policy.get("entitlements"):
        failures.append("release manifest entitlements do not match policy")
    hashes = value.get("component_hashes", {})
    if not isinstance(hashes, dict) or set(hashes) != set(COMPONENTS):
        failures.append("release manifest component hash closure changed")
    else:
        values = list(hashes.values())
        if any(not valid_sha256(item) for item in values) or len(set(values)) != len(values):
            failures.append("release manifest component hashes are invalid or collide")
    package = value.get("package", {})
    if set(package) != {"format", "sha256"} or package.get("format") != "pkg" or not valid_sha256(package.get("sha256")):
        failures.append("release manifest package identity is invalid")
    toolchain = value.get("toolchain", {})
    apple_sdk = toolchain.get("apple_sdk", {}) if isinstance(toolchain, dict) else {}
    swift = toolchain.get("swift_toolchain", {}) if isinstance(toolchain, dict) else {}
    if (
        not isinstance(toolchain, dict)
        or set(toolchain) != {"apple_sdk", "swift_toolchain"}
        or set(apple_sdk) != {"id", "sha256"}
        or not isinstance(apple_sdk.get("id"), str)
        or not valid_sha256(apple_sdk.get("sha256"))
        or set(swift) != {"version", "build", "sha256"}
        or not re.fullmatch(r"[0-9]+\.[0-9]+(?:\.[0-9]+)?", str(swift.get("version", "")))
        or not isinstance(swift.get("build"), str)
        or not valid_sha256(swift.get("sha256"))
    ):
        failures.append("release manifest toolchain identity is invalid")
    vscode = value.get("vscode", {})
    if (
        not isinstance(vscode, dict)
        or set(vscode) != {"version", "commit", "sha256"}
        or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", str(vscode.get("version", "")))
        or not REVISION.fullmatch(str(vscode.get("commit", "")))
        or not valid_sha256(vscode.get("sha256"))
    ):
        failures.append("release manifest Visual Studio Code identity is invalid")
    if contains_prohibited_key(value):
        failures.append("release manifest contains credential or environment material")
    return failures


def validate_terminal(value: Any, policy: dict[str, Any], package_sha256: str) -> list[str]:
    if not isinstance(value, dict) or set(value) != TERMINAL_KEYS:
        return ["release terminal field closure changed"]
    failures: list[str] = []
    expected = {
        "schema_version": 1,
        "record_type": "macos-release-runner-terminal",
        "source_revision": policy.get("source_revision"),
        "version": policy.get("version"),
        "macos_build": policy.get("expected_macos_build"),
        "xcode_build": policy.get("expected_xcode_build"),
        "architecture": "arm64",
        "package_sha256": package_sha256,
        "notary_status": "Accepted",
        "staple_valid": True,
        "gatekeeper_install_accepted": True,
        "candidate_install": True,
        "candidate_launch": True,
        "candidate_uninstall": True,
        "rollback_install": True,
        "rollback_launch": True,
        "credential_values_present": False,
        "release_claim": "signed-package-candidate",
    }
    for key, item in expected.items():
        if value.get(key) != item:
            failures.append(f"release terminal mismatch: {key}")
    if contains_prohibited_key(value):
        failures.append("release terminal contains credential or environment material")
    return failures


def validate_reference_bundle(
    policy_path: Path, manifest_path: Path, package_path: Path, terminal_path: Path
) -> tuple[list[str], dict[str, Any] | None]:
    failures: list[str] = []
    policy, found = read_closed_json(policy_path, POLICY_KEYS, 1024 * 1024)
    failures.extend(found)
    manifest, found = read_closed_json(manifest_path, MANIFEST_KEYS, 4 * 1024 * 1024)
    failures.extend(found)
    terminal, found = read_closed_json(terminal_path, TERMINAL_KEYS, 1024 * 1024)
    failures.extend(found)
    try:
        metadata = package_path.lstat()
        if (
            not stat.S_ISREG(metadata.st_mode)
            or metadata.st_nlink != 1
            or metadata.st_uid != os.geteuid()
            or metadata.st_mode & 0o022
            or metadata.st_size <= 0
            or metadata.st_size > 16 * 1024 * 1024 * 1024
        ):
            failures.append("unsafe reference package file")
    except OSError:
        failures.append("reference package unavailable")
    if failures or not isinstance(policy, dict) or not isinstance(manifest, dict) or not isinstance(terminal, dict):
        return failures, None
    failures.extend(validate_release_policy(policy))
    failures.extend(validate_manifest(manifest, policy))
    previous_path = Path(policy.get("previous_package_path", ""))
    try:
        previous_metadata = previous_path.lstat()
        if (
            not stat.S_ISREG(previous_metadata.st_mode)
            or previous_metadata.st_nlink != 1
            or previous_metadata.st_uid != os.geteuid()
            or previous_metadata.st_mode & 0o022
            or sha256_file(previous_path) != policy.get("previous_package_sha256")
        ):
            failures.append("prior rollback package identity is invalid")
    except OSError:
        failures.append("prior rollback package is unavailable")
    package_sha256 = sha256_file(package_path)
    if manifest.get("package", {}).get("sha256") != package_sha256:
        failures.append("reference package digest does not match manifest")
    failures.extend(validate_terminal(terminal, policy, package_sha256))
    if failures:
        return failures, None
    result = {
        "schema_version": 1,
        "record_type": "macos-signed-reference-package-evidence",
        "status": "accepted-candidate-evidence",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": package_sha256,
        "package_bytes": package_path.stat().st_size,
        "component_hashes": manifest["component_hashes"],
        "input_hashes": {
            "policy": sha256_file(policy_path),
            "manifest": sha256_file(manifest_path),
            "terminal": sha256_file(terminal_path),
        },
        "credential_values_present": False,
        "private_environment_values_present": False,
        "native_signature_reverified_by_ingestor": False,
        "release_claim": "signed-package-candidate",
        "macos_support_claim": False,
    }
    return [], result


def validate_sources(root: Path = ROOT) -> list[str]:
    failures = [
        f"missing source input: {relative}"
        for relative in SOURCE_PATHS
        if not (root / relative).is_file()
    ]
    if failures:
        return failures
    source = (root / "scripts/macos_signed_reference_package.py").read_text(encoding="utf-8")
    tests = (root / "tests/test_macos_signed_reference_package.py").read_text(encoding="utf-8")
    documentation = (root / "packaging/macos/REFERENCE-PACKAGE.md").read_text(encoding="utf-8")
    for term in (
        "read_closed_json(policy_path, POLICY_KEYS",
        "read_closed_json(manifest_path, MANIFEST_KEYS",
        "read_closed_json(terminal_path, TERMINAL_KEYS",
        "metadata.st_nlink != 1",
        "metadata.st_uid != os.geteuid()",
        "metadata.st_mode & 0o022",
        "manifest.get(\"package\", {}).get(\"sha256\") != package_sha256",
        "value.get(\"entitlements\") != policy.get(\"entitlements\")",
        "len(set(values)) != len(values)",
        '"native_signature_reverified_by_ingestor": False',
        '"macos_support_claim": False',
    ):
        if term not in source:
            failures.append(f"signed reference-package verifier missing term: {term}")
    for prohibited in (
        "subprocess.run([\"code" + "sign\"",
        "notary" + "tool submit",
        "cu" + "rl ",
        "reque" + "sts.",
        "--pass" + "word",
    ):
        if prohibited in source:
            failures.append(f"signed reference-package verifier contains prohibited effect: {prohibited}")
    if "Seven" not in tests or len(re.findall(r"^    def test_", tests, re.MULTILINE)) != 7:
        failures.append("signed reference-package mutation corpus is not seven closed tests")
    for term in (
        "does not\ncopy the package",
        "not reference-package evidence",
        "No\nsigned package",
        "`BLOCKED-MACOS`",
    ):
        if term not in documentation:
            failures.append(f"reference-package documentation missing statement: {term}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed signed reference-package source changed: {relative}")
    return revision, tree


def build_source_report(root: Path = ROOT, *, source_revision: str = "HEAD") -> dict[str, Any]:
    failures = validate_sources(root)
    if failures:
        raise ValueError("; ".join(failures))
    revision, tree = resolve_revision(source_revision, root)
    return {
        "schema_version": 1,
        "record_type": "macos-signed-reference-package-source-contract",
        "task_id": "8.1.2.1",
        "status": "prepared-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "input_count": 4,
            "component_count": 4,
            "policy_field_count": len(POLICY_KEYS),
            "manifest_field_count": len(MANIFEST_KEYS),
            "terminal_field_count": len(TERMINAL_KEYS),
            "maximum_package_bytes": 16 * 1024 * 1024 * 1024,
            "native_signature_reverification": False,
            "network_authority": False,
            "credential_authority": False,
            "package_copy_authority": False,
        },
        "execution": {
            "signed_package_ingested": False,
            "release_manifest_ingested": False,
            "component_hashes_verified": False,
            "terminal_lifecycle_record_ingested": False,
            "apple_silicon_execution_observed": False,
            "independent_review_performed": False,
        },
        "claims": {
            "task_complete": False,
            "signed_reference_package_exists": False,
            "release_manifest_exists": False,
            "release_candidate_exists": False,
            "macos_support": False,
        },
        "remaining_blockers": [
            "No externally signed, notarized, stapled reference package exists.",
            "No release-approved production policy or successful ceremony terminal record exists.",
            "No release-derived component/package manifest exists.",
            "No four-component hash set has been verified against an exact package.",
            "The ingestor does not replace native signature, notarization, stapling, or Gatekeeper evidence.",
            "No independent reviewer has retained the external evidence bundle.",
        ],
    }


def write_atomic(path: Path, value: Any) -> None:
    if not path.is_absolute() or path.exists() or path.is_symlink():
        raise ValueError("reference evidence output must be a new absolute path")
    parent = path.parent
    metadata = parent.lstat()
    if (
        not stat.S_ISDIR(metadata.st_mode)
        or metadata.st_uid != os.geteuid()
        or metadata.st_mode & 0o077
    ):
        raise ValueError("reference evidence parent must be an owner-only directory")
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-macos-reference-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(canonical_bytes(value))
            stream.flush()
            os.fsync(stream.fileno())
        temporary.chmod(0o600)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-source-contract", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--package", type=Path)
    parser.add_argument("--terminal", type=Path)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    live_values = (arguments.policy, arguments.manifest, arguments.package, arguments.terminal, arguments.output)
    try:
        if any(item is not None for item in live_values):
            if arguments.write_source_contract or any(item is None for item in live_values):
                raise ValueError("live reference verification requires exactly five paths")
            failures, result = validate_reference_bundle(
                arguments.policy, arguments.manifest, arguments.package, arguments.terminal
            )
            if failures or result is None:
                raise ValueError("; ".join(failures))
            write_atomic(arguments.output, result)
            print("macOS signed reference-package evidence accepted without support promotion")
            return 0
        if arguments.write_source_contract:
            report = build_source_report(source_revision=arguments.source_revision)
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_bytes(canonical_bytes(report))
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained source contract has no source revision")
            expected = canonical_bytes(build_source_report(source_revision=revision))
            if REPORT_PATH.read_bytes() != expected:
                raise ValueError("signed reference-package source contract is stale")
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS signed reference-package verification failed: {error}")
        return 1
    print("macOS signed reference-package source contract passed without package promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

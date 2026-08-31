#!/usr/bin/env python3
"""Assemble and verify content-free macOS IPC and bookmark receipts."""

from __future__ import annotations

import argparse
import json
import os
import re
import stat
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.macos_release_runner_source_contract import POLICY_KEYS  # noqa: E402
from scripts.macos_signed_reference_package import (  # noqa: E402
    MANIFEST_KEYS,
    TERMINAL_KEYS,
    canonical_bytes,
    contains_prohibited_key,
    read_closed_json,
    sha256_file,
    valid_sha256,
    validate_reference_bundle,
    write_atomic,
)

REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-ipc-bookmark-receipts-source-contract.json"
)
SOURCE_PATHS = (
    "packaging/macos/IPC-BOOKMARK-RECEIPTS.md",
    "scripts/macos_ipc_bookmark_receipts.py",
    "scripts/macos_signed_reference_package.py",
    "tests/test_macos_ipc_bookmark_receipts.py",
)

IPC_TRUE_FIELDS = (
    "live_audit_token_verified",
    "live_designated_requirement_verified",
    "live_signature_verified",
    "live_bundle_verified",
    "live_team_verified",
    "live_app_sandbox_verified",
    "live_app_group_verified",
    "peer_pid_verified",
    "peer_uid_verified",
    "peer_gid_verified",
    "launch_pid_verified",
    "fresh_launch_challenge_verified",
    "challenge_consumed_once",
    "replay_refused",
    "round_trip_verified",
)
BOOKMARK_TRUE_FIELDS = (
    "directory_only_selection",
    "alias_resolution_disabled",
    "keychain_inserted",
    "keychain_loaded",
    "bookmark_created",
    "bookmark_resolved",
    "resolution_without_ui",
    "resolution_without_mounting",
    "start_stop_access_balanced",
    "read_access_verified",
    "write_access_denied",
    "resource_move_resolved",
    "reboot_reopen_resolved",
    "stale_bookmark_observed",
    "stale_bookmark_refreshed",
    "bookmark_revoked",
    "post_revocation_denied",
)
COMMON_RAW_FIELDS = (
    "schema_version",
    "record_type",
    "status",
    "source_revision",
    "version",
    "macos_build",
    "xcode_build",
    "architecture",
    "package_sha256",
    "team_id",
)
IPC_DETAIL_FIELDS = (
    "host_bundle_identifier",
    "bridge_bundle_identifier",
    "app_group_identifier",
    "bridge_designated_requirement",
    "protocol_version",
    "handshake_bytes",
    "request_maximum_bytes",
    "response_maximum_bytes",
    "socket_parent_mode",
    "socket_mode",
    "peer_audit_identity_sha256",
    *IPC_TRUE_FIELDS,
)
BOOKMARK_DETAIL_FIELDS = (
    "host_bundle_identifier",
    "app_group_identifier",
    "keychain_access_group",
    "bookmark_identifier_sha256",
    "resource_identity_sha256",
    "volume_identity_sha256",
    "selection_count",
    "bookmark_scope",
    "bookmark_authority",
    *BOOKMARK_TRUE_FIELDS,
)
IPC_RAW_KEYS = set(
    (*COMMON_RAW_FIELDS, *IPC_DETAIL_FIELDS, "credential_values_present", "private_environment_values_present")
)
BOOKMARK_RAW_KEYS = set(
    (*COMMON_RAW_FIELDS, *BOOKMARK_DETAIL_FIELDS, "credential_values_present", "private_environment_values_present")
)
RECEIPTS_KEYS = {
    "schema_version",
    "record_type",
    "status",
    "source_revision",
    "version",
    "macos_build",
    "xcode_build",
    "architecture",
    "team_id",
    "package_sha256",
    "ipc_authentication",
    "bookmark_lifecycle",
    "raw_receipt_hashes",
    "credential_values_present",
    "private_environment_values_present",
    "release_claim",
}
RAW_HASH_KEYS = {"ipc_authentication", "bookmark_lifecycle"}
REVISION = re.compile(r"[0-9a-f]{40}")


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def safe_owner_only_root(path: Path) -> list[str]:
    try:
        metadata = path.lstat()
    except OSError:
        return ["native receipt directory unavailable"]
    if (
        not path.is_absolute()
        or not stat.S_ISDIR(metadata.st_mode)
        or metadata.st_uid != os.geteuid()
        or metadata.st_mode & 0o077
    ):
        return ["native receipt directory is not owner-only"]
    return []


def load_release_inputs(
    policy_path: Path,
    manifest_path: Path,
    terminal_path: Path,
) -> tuple[list[str], dict[str, Any] | None, dict[str, Any] | None, dict[str, Any] | None]:
    failures: list[str] = []
    policy, found = read_closed_json(policy_path, POLICY_KEYS, 1024 * 1024)
    failures.extend(found)
    manifest, found = read_closed_json(manifest_path, MANIFEST_KEYS, 4 * 1024 * 1024)
    failures.extend(found)
    terminal, found = read_closed_json(terminal_path, TERMINAL_KEYS, 1024 * 1024)
    failures.extend(found)
    return (
        failures,
        policy if isinstance(policy, dict) else None,
        manifest if isinstance(manifest, dict) else None,
        terminal if isinstance(terminal, dict) else None,
    )


def expected_common(
    record_type: str,
    policy: dict[str, Any],
    package_sha256: str,
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": record_type,
        "status": "passed",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "macos_build": policy["expected_macos_build"],
        "xcode_build": policy["expected_xcode_build"],
        "architecture": "arm64",
        "package_sha256": package_sha256,
        "team_id": policy["team_id"],
    }


def validate_ipc_raw(
    value: Any,
    policy: dict[str, Any],
    manifest: dict[str, Any],
    package_sha256: str,
) -> list[str]:
    if not isinstance(value, dict) or set(value) != IPC_RAW_KEYS:
        return ["IPC authentication receipt field closure changed"]
    failures: list[str] = []
    expected = expected_common("macos-ipc-authentication-receipt", policy, package_sha256)
    expected.update(
        {
            "host_bundle_identifier": policy["bundle_identifiers"]["kernel_host"],
            "bridge_bundle_identifier": policy["bundle_identifiers"]["vscode_bridge"],
            "app_group_identifier": policy["app_group_identifier"],
            "bridge_designated_requirement": manifest["code_identity"]["designated_requirements"]["vscode_bridge"],
            "protocol_version": 1,
            "handshake_bytes": 68,
            "request_maximum_bytes": 64 * 1024,
            "response_maximum_bytes": 4 * 1024 * 1024,
            "socket_parent_mode": "0700",
            "socket_mode": "0600",
            "credential_values_present": False,
            "private_environment_values_present": False,
        }
    )
    for key, expected_value in expected.items():
        if value.get(key) != expected_value:
            failures.append(f"IPC authentication receipt mismatch: {key}")
    for key in IPC_TRUE_FIELDS:
        if value.get(key) is not True:
            failures.append(f"IPC authentication disposition did not pass: {key}")
    if not valid_sha256(value.get("peer_audit_identity_sha256")):
        failures.append("IPC peer audit identity digest is invalid")
    if contains_prohibited_key(value):
        failures.append("IPC authentication receipt contains credential or environment material")
    return failures


def validate_bookmark_raw(
    value: Any,
    policy: dict[str, Any],
    package_sha256: str,
) -> list[str]:
    if not isinstance(value, dict) or set(value) != BOOKMARK_RAW_KEYS:
        return ["bookmark lifecycle receipt field closure changed"]
    failures: list[str] = []
    expected = expected_common("macos-bookmark-lifecycle-receipt", policy, package_sha256)
    expected.update(
        {
            "host_bundle_identifier": policy["bundle_identifiers"]["kernel_host"],
            "app_group_identifier": policy["app_group_identifier"],
            "keychain_access_group": policy["keychain_access_group"],
            "selection_count": 1,
            "bookmark_scope": "app-scoped",
            "bookmark_authority": "read-only",
            "credential_values_present": False,
            "private_environment_values_present": False,
        }
    )
    for key, expected_value in expected.items():
        if value.get(key) != expected_value:
            failures.append(f"bookmark lifecycle receipt mismatch: {key}")
    for key in BOOKMARK_TRUE_FIELDS:
        if value.get(key) is not True:
            failures.append(f"bookmark lifecycle disposition did not pass: {key}")
    identities = [value.get(key) for key in (
        "bookmark_identifier_sha256",
        "resource_identity_sha256",
        "volume_identity_sha256",
    )]
    if any(not valid_sha256(item) for item in identities) or len(set(identities)) != 3:
        failures.append("bookmark lifecycle identity digests are invalid or collide")
    if contains_prohibited_key(value):
        failures.append("bookmark lifecycle receipt contains credential or environment material")
    return failures


def assemble_receipts(
    policy_path: Path,
    manifest_path: Path,
    package_path: Path,
    terminal_path: Path,
    evidence_root: Path,
) -> tuple[list[str], dict[str, Any] | None]:
    failures, reference = validate_reference_bundle(
        policy_path, manifest_path, package_path, terminal_path
    )
    found, policy, manifest, _terminal = load_release_inputs(
        policy_path, manifest_path, terminal_path
    )
    failures.extend(found)
    failures.extend(safe_owner_only_root(evidence_root))
    if failures or reference is None or policy is None or manifest is None:
        return failures, None

    ipc_path = evidence_root / "ipc-authentication-receipt.json"
    bookmark_path = evidence_root / "bookmark-lifecycle-receipt.json"
    ipc, found = read_closed_json(ipc_path, IPC_RAW_KEYS, 1024 * 1024)
    failures.extend(found)
    bookmark, found = read_closed_json(bookmark_path, BOOKMARK_RAW_KEYS, 1024 * 1024)
    failures.extend(found)
    if not isinstance(ipc, dict) or not isinstance(bookmark, dict):
        return failures, None
    package_sha256 = reference["package_sha256"]
    failures.extend(validate_ipc_raw(ipc, policy, manifest, package_sha256))
    failures.extend(validate_bookmark_raw(bookmark, policy, package_sha256))
    if failures:
        return failures, None

    report = {
        "schema_version": 1,
        "record_type": "macos-ipc-bookmark-receipts",
        "status": "native-lifecycle-passed",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "macos_build": policy["expected_macos_build"],
        "xcode_build": policy["expected_xcode_build"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": package_sha256,
        "ipc_authentication": {key: ipc[key] for key in IPC_DETAIL_FIELDS},
        "bookmark_lifecycle": {key: bookmark[key] for key in BOOKMARK_DETAIL_FIELDS},
        "raw_receipt_hashes": {
            "ipc_authentication": sha256_file(ipc_path),
            "bookmark_lifecycle": sha256_file(bookmark_path),
        },
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
    }
    return [], report


def validate_receipts(
    policy_path: Path,
    manifest_path: Path,
    package_path: Path,
    terminal_path: Path,
    receipts_path: Path,
) -> tuple[list[str], dict[str, Any] | None]:
    failures, reference = validate_reference_bundle(
        policy_path, manifest_path, package_path, terminal_path
    )
    found, policy, manifest, _terminal = load_release_inputs(
        policy_path, manifest_path, terminal_path
    )
    failures.extend(found)
    receipts, found = read_closed_json(receipts_path, RECEIPTS_KEYS, 4 * 1024 * 1024)
    failures.extend(found)
    if failures or reference is None or policy is None or manifest is None or not isinstance(receipts, dict):
        return failures, None

    expected = {
        "schema_version": 1,
        "record_type": "macos-ipc-bookmark-receipts",
        "status": "native-lifecycle-passed",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "macos_build": policy["expected_macos_build"],
        "xcode_build": policy["expected_xcode_build"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": reference["package_sha256"],
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
    }
    for key, expected_value in expected.items():
        if receipts.get(key) != expected_value:
            failures.append(f"assembled native receipt mismatch: {key}")
    if contains_prohibited_key(receipts):
        failures.append("assembled native receipts contain credential or environment material")

    ipc = receipts.get("ipc_authentication")
    if not isinstance(ipc, dict) or set(ipc) != set(IPC_DETAIL_FIELDS):
        failures.append("assembled IPC receipt field closure changed")
    else:
        synthetic_raw = {
            **expected_common("macos-ipc-authentication-receipt", policy, reference["package_sha256"]),
            **ipc,
            "credential_values_present": False,
            "private_environment_values_present": False,
        }
        failures.extend(validate_ipc_raw(synthetic_raw, policy, manifest, reference["package_sha256"]))

    bookmark = receipts.get("bookmark_lifecycle")
    if not isinstance(bookmark, dict) or set(bookmark) != set(BOOKMARK_DETAIL_FIELDS):
        failures.append("assembled bookmark receipt field closure changed")
    else:
        synthetic_raw = {
            **expected_common("macos-bookmark-lifecycle-receipt", policy, reference["package_sha256"]),
            **bookmark,
            "credential_values_present": False,
            "private_environment_values_present": False,
        }
        failures.extend(validate_bookmark_raw(synthetic_raw, policy, reference["package_sha256"]))

    raw_hashes = receipts.get("raw_receipt_hashes")
    if (
        not isinstance(raw_hashes, dict)
        or set(raw_hashes) != RAW_HASH_KEYS
        or any(not valid_sha256(value) for value in raw_hashes.values())
        or len(set(raw_hashes.values())) != len(RAW_HASH_KEYS)
    ):
        failures.append("raw native receipt hashes are invalid or collide")
    if failures:
        return failures, None

    assert isinstance(raw_hashes, dict)
    result = {
        "schema_version": 1,
        "record_type": "macos-ipc-bookmark-receipts-evidence",
        "status": "accepted-candidate-evidence",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": reference["package_sha256"],
        "receipts_sha256": sha256_file(receipts_path),
        "raw_receipt_hashes": raw_hashes,
        "ipc_authentication_reconciled": True,
        "bookmark_lifecycle_reconciled": True,
        "native_operations_executed_by_ingestor": False,
        "credential_values_present": False,
        "private_environment_values_present": False,
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
    source = (root / "scripts/macos_ipc_bookmark_receipts.py").read_text(encoding="utf-8")
    tests = (root / "tests/test_macos_ipc_bookmark_receipts.py").read_text(encoding="utf-8")
    documentation = (root / "packaging/macos/IPC-BOOKMARK-RECEIPTS.md").read_text(encoding="utf-8")
    for term in (
        'evidence_root / "ipc-authentication-receipt.json"',
        'evidence_root / "bookmark-lifecycle-receipt.json"',
        '"bridge_designated_requirement": manifest["code_identity"]',
        'value.get("peer_audit_identity_sha256")',
        '"socket_parent_mode": "0700"',
        '"socket_mode": "0600"',
        '"bookmark_scope": "app-scoped"',
        '"bookmark_authority": "read-only"',
        '"post_revocation_denied"',
        '"native_operations_executed_by_ingestor": False',
        '"macos_support_claim": False',
    ):
        if term not in source:
            failures.append(f"IPC/bookmark receipt verifier missing term: {term}")
    for prohibited in (
        "socket." + "socket(",
        'subprocess.run(["secur' + 'ity"',
        "NSOpen" + "Panel(",
        "startAccessing" + "SecurityScopedResource(",
        "cu" + "rl ",
        "reque" + "sts.",
    ):
        if prohibited in source:
            failures.append(f"IPC/bookmark receipt verifier contains prohibited effect: {prohibited}")
    if "Seven" not in tests or len(re.findall(r"^    def test_", tests, re.MULTILINE)) != 7:
        failures.append("IPC/bookmark mutation corpus is not seven closed tests")
    for term in (
        "two content-free receipt families",
        "never the audit token itself",
        "neither invokes native IPC",
        "source evidence\nonly",
        "`BLOCKED-MACOS`",
    ):
        if term not in documentation:
            failures.append(f"IPC/bookmark receipt documentation missing statement: {term}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    if not REVISION.fullmatch(revision):
        raise ValueError("reviewed source revision is invalid")
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed IPC/bookmark receipt source changed: {relative}")
    return revision, tree


def build_source_report(root: Path = ROOT, *, source_revision: str = "HEAD") -> dict[str, Any]:
    failures = validate_sources(root)
    if failures:
        raise ValueError("; ".join(failures))
    revision, tree = resolve_revision(source_revision, root)
    return {
        "schema_version": 1,
        "record_type": "macos-ipc-bookmark-receipts-source-contract",
        "task_id": "8.1.2.3",
        "status": "prepared-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "receipt_family_count": 2,
            "ipc_required_disposition_count": len(IPC_TRUE_FIELDS),
            "bookmark_required_disposition_count": len(BOOKMARK_TRUE_FIELDS),
            "raw_audit_token_retained": False,
            "raw_bookmark_bytes_retained": False,
            "native_operation_authority": False,
            "network_authority": False,
            "credential_authority": False,
            "package_copy_authority": False,
            "workspace_authority": False,
            "support_promotion_authority": False,
        },
        "execution": {
            "native_ipc_receipt_observed": False,
            "signed_peer_authenticated": False,
            "native_bookmark_receipt_observed": False,
            "powerbox_selection_observed": False,
            "keychain_lifecycle_observed": False,
            "bookmark_lifecycle_observed": False,
            "independent_review_performed": False,
        },
        "claims": {
            "task_complete": False,
            "native_receipts_exist": False,
            "release_candidate_exists": False,
            "macos_support": False,
        },
        "remaining_blockers": [
            "No external signed host and bridge IPC authentication receipt exists.",
            "No live audit-token, designated-requirement, App Group, challenge, socket, or replay-refusal observation exists.",
            "No external Powerbox, Keychain, or security-scoped bookmark receipt exists.",
            "No native move, reboot reopen, stale refresh, revocation, or post-revocation denial receipt exists.",
            "No release-approved policy, signed package, release-derived manifest, or successful terminal record exists.",
            "No independent reviewer has reconciled and retained the two native receipt families.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-source-contract", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--assemble", action="store_true")
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--package", type=Path)
    parser.add_argument("--terminal", type=Path)
    parser.add_argument("--evidence-root", type=Path)
    parser.add_argument("--receipts-output", type=Path)
    parser.add_argument("--receipts", type=Path)
    parser.add_argument("--review-output", type=Path)
    arguments = parser.parse_args()
    try:
        if arguments.assemble:
            required = (
                arguments.policy,
                arguments.manifest,
                arguments.package,
                arguments.terminal,
                arguments.evidence_root,
                arguments.receipts_output,
            )
            if arguments.verify or arguments.write_source_contract or any(item is None for item in required):
                raise ValueError("receipt assembly requires exactly its six path inputs")
            failures, report = assemble_receipts(*required[:-1])  # type: ignore[arg-type]
            if failures or report is None:
                raise ValueError("; ".join(failures))
            write_atomic(arguments.receipts_output, report)  # type: ignore[arg-type]
            print("macOS IPC and bookmark receipts assembled without native effects")
            return 0
        if arguments.verify:
            required = (
                arguments.policy,
                arguments.manifest,
                arguments.package,
                arguments.terminal,
                arguments.receipts,
                arguments.review_output,
            )
            if arguments.write_source_contract or any(item is None for item in required):
                raise ValueError("receipt verification requires exactly its six path inputs")
            failures, result = validate_receipts(*required[:-1])  # type: ignore[arg-type]
            if failures or result is None:
                raise ValueError("; ".join(failures))
            write_atomic(arguments.review_output, result)  # type: ignore[arg-type]
            print("macOS IPC and bookmark receipts accepted without support promotion")
            return 0
        if any(
            item is not None
            for item in (
                arguments.policy,
                arguments.manifest,
                arguments.package,
                arguments.terminal,
                arguments.evidence_root,
                arguments.receipts_output,
                arguments.receipts,
                arguments.review_output,
            )
        ):
            raise ValueError("receipt path inputs require --assemble or --verify")
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
                raise ValueError("IPC/bookmark receipt source contract is stale")
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS IPC/bookmark receipt verification failed: {error}")
        return 1
    print("macOS IPC/bookmark receipt source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

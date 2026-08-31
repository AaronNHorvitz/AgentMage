#!/usr/bin/env python3
"""Verify content-free S-008-AT01 standard-user acceptance evidence."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.macos_release_runner_source_contract import POLICY_KEYS
from scripts.macos_signed_reference_package import (
    canonical_bytes,
    contains_prohibited_key,
    read_closed_json,
    sha256_file,
    validate_reference_bundle,
    write_atomic,
)

REPORT_PATH = ROOT / (
    "artifacts/sprints/sprint-8/story-8.1/"
    "macos-standard-user-acceptance-source-contract.json"
)
SOURCE_PATHS = (
    "packaging/macos/STANDARD-USER-ACCEPTANCE.md",
    "packaging/macos/release-runner.sh",
    "scripts/macos_standard_user_acceptance.py",
    "tests/test_macos_standard_user_acceptance.py",
    "artifacts/sprints/sprint-8/story-8.1/macos-release-runner-source-contract.json",
    "artifacts/sprints/sprint-8/story-8.1/macos-release-reports-source-contract.json",
)
PHASE_ORDER = (
    "build",
    "sign",
    "notarize",
    "staple",
    "gatekeeper-check",
    "install",
    "launch",
    "use",
    "remove",
)
PASS_FIELDS = (
    "build_passed",
    "sign_passed",
    "notarization_passed",
    "stapling_passed",
    "gatekeeper_passed",
    "install_passed",
    "launch_passed",
    "use_passed",
    "removal_passed",
)
ACCEPTANCE_KEYS = {
    "schema_version",
    "record_type",
    "source_revision",
    "version",
    "macos_build",
    "xcode_build",
    "architecture",
    "package_sha256",
    "identity_class",
    "real_effective_uid_match",
    "home_owner_match",
    "primary_gid_observed",
    "phase_order",
    *PASS_FIELDS,
    "zero_residue",
    "network_closed_before_installed_execution",
    "credential_values_present",
    "private_environment_values_present",
    "release_claim",
}


def git(*args: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *args], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def validate_acceptance(
    policy_path: Path,
    manifest_path: Path,
    package_path: Path,
    terminal_path: Path,
    acceptance_path: Path,
) -> tuple[list[str], dict[str, Any] | None]:
    failures, reference = validate_reference_bundle(
        policy_path, manifest_path, package_path, terminal_path
    )
    policy, found = read_closed_json(policy_path, POLICY_KEYS, 1024 * 1024)
    failures.extend(found)
    value, found = read_closed_json(acceptance_path, ACCEPTANCE_KEYS, 1024 * 1024)
    failures.extend(found)
    if (
        failures
        or reference is None
        or not isinstance(policy, dict)
        or not isinstance(value, dict)
    ):
        return failures, None
    expected = {
        "schema_version": 1,
        "record_type": "macos-standard-user-acceptance",
        "source_revision": reference["source_revision"],
        "version": reference["version"],
        "macos_build": policy["expected_macos_build"],
        "xcode_build": policy["expected_xcode_build"],
        "architecture": "arm64",
        "package_sha256": reference["package_sha256"],
        "identity_class": "non-admin-standard-user",
        "real_effective_uid_match": True,
        "home_owner_match": True,
        "primary_gid_observed": True,
        "phase_order": list(PHASE_ORDER),
        **{field: True for field in PASS_FIELDS},
        "zero_residue": True,
        "network_closed_before_installed_execution": True,
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
    }
    for key, expected_value in expected.items():
        if value.get(key) != expected_value:
            failures.append(f"standard-user acceptance mismatch: {key}")
    if contains_prohibited_key(value):
        failures.append("standard-user acceptance contains credential material")
    if failures:
        return failures, None
    return [], {
        "schema_version": 1,
        "record_type": "macos-standard-user-acceptance-evidence",
        "status": "accepted-candidate-evidence",
        "source_revision": reference["source_revision"],
        "version": reference["version"],
        "architecture": "arm64",
        "package_sha256": reference["package_sha256"],
        "acceptance_sha256": sha256_file(acceptance_path),
        "phase_count": len(PHASE_ORDER),
        "failed_phase_count": 0,
        "standard_user_identity_verified": True,
        "remaining_residue_count": 0,
        "native_operations_executed_by_verifier": False,
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
        "macos_support_claim": False,
    }


def validate_sources(root: Path = ROOT) -> list[str]:
    failures = [
        f"missing source input: {path}"
        for path in SOURCE_PATHS
        if not (root / path).is_file()
    ]
    if failures:
        return failures
    runner = (root / "packaging/macos/release-runner.sh").read_text(encoding="utf-8")
    source = (root / "scripts/macos_standard_user_acceptance.py").read_text(
        encoding="utf-8"
    )
    tests = (root / "tests/test_macos_standard_user_acceptance.py").read_text(
        encoding="utf-8"
    )
    docs = (root / "packaging/macos/STANDARD-USER-ACCEPTANCE.md").read_text(
        encoding="utf-8"
    )
    for term in (
        'fail "root-forbidden"',
        'fail "elevated-identity-forbidden"',
        'fail "administrator-group-forbidden"',
        'fail "home-owner-mismatch"',
        '"record_type":"macos-standard-user-acceptance"',
        '"identity_class":"non-admin-standard-user"',
        '"network_closed_before_installed_execution":true',
    ):
        if term not in runner:
            failures.append(f"standard-user release runner missing term: {term}")
    for term in (
        '"native_operations_executed_by_verifier": False',
        '"macos_support_claim": False',
    ):
        if term not in source:
            failures.append(f"standard-user verifier missing term: {term}")
    if "Seven closed tests" not in tests:
        failures.append("standard-user mutation suite is not closed")
    for term in ("nine ordered phases", "performs no build", "remains `BLOCKED-MACOS`"):
        if term not in docs:
            failures.append(f"standard-user procedure missing term: {term}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        if git("show", f"{revision}:{relative}", root=root, binary=True) != (
            root / relative
        ).read_bytes():
            raise ValueError(f"reviewed standard-user source changed: {relative}")
    return revision, tree


def build_source_report(
    root: Path = ROOT, *, source_revision: str = "HEAD"
) -> dict[str, Any]:
    failures = validate_sources(root)
    if failures:
        raise ValueError("; ".join(failures))
    revision, tree = resolve_revision(source_revision, root)
    return {
        "schema_version": 1,
        "record_type": "macos-standard-user-acceptance-source-contract",
        "task_id": "8.1.3.4",
        "status": "prepared-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": path, "sha256": sha256_file(root / path)} for path in SOURCE_PATHS
        ],
        "contract": {
            "acceptance_phase_count": len(PHASE_ORDER),
            "failed_phase_count": 0,
            "administrator_group_allowed": False,
            "elevated_identity_allowed": False,
            "residue_allowed": False,
            "native_operation_authority": False,
            "network_authority": False,
            "credential_authority": False,
            "support_promotion_authority": False,
        },
        "execution": {
            "standard_user_campaign_observed": False,
            "signed_package_executed": False,
            "protected_outputs_observed": False,
            "independent_review_performed": False,
        },
        "claims": {
            "task_complete": False,
            "native_acceptance_exists": False,
            "release_candidate_exists": False,
            "macos_support": False,
        },
        "remaining_blockers": [
            "No production policy, signed package, release manifest, or successful terminal record exists.",
            "No pinned physical Apple Silicon image has run the standard-user ceremony.",
            "No native nine-phase acceptance record or protected command output exists.",
            "No independent reviewer has reconciled the standard-user acceptance bundle.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-source-contract", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--package", type=Path)
    parser.add_argument("--terminal", type=Path)
    parser.add_argument("--acceptance", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        if args.verify:
            required = (
                args.policy,
                args.manifest,
                args.package,
                args.terminal,
                args.acceptance,
                args.output,
            )
            if args.write_source_contract or any(item is None for item in required):
                raise ValueError("standard-user verification requires six path inputs")
            failures, result = validate_acceptance(*required[:-1])  # type: ignore[arg-type]
            if failures or result is None:
                raise ValueError("; ".join(failures))
            write_atomic(args.output, result)  # type: ignore[arg-type]
            print("macOS standard-user acceptance verified without native effects")
            return 0
        if any(
            item is not None
            for item in (
                args.policy,
                args.manifest,
                args.package,
                args.terminal,
                args.acceptance,
                args.output,
            )
        ):
            raise ValueError("standard-user paths require --verify")
        if args.write_source_contract:
            report = build_source_report(source_revision=args.source_revision)
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_bytes(canonical_bytes(report))
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained standard-user contract has no revision")
            if REPORT_PATH.read_bytes() != canonical_bytes(
                build_source_report(source_revision=revision)
            ):
                raise ValueError("standard-user source contract is stale")
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS standard-user acceptance verification failed: {error}")
        return 1
    print("macOS standard-user source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

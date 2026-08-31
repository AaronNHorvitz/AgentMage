#!/usr/bin/env python3
"""Assemble and verify content-free macOS sandbox and attack evidence."""

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

from scripts.macos_release_runner_source_contract import (  # noqa: E402
    COMPONENTS,
    EXPECTED_ENTITLEMENTS,
    POLICY_KEYS,
)
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
    "macos-sandbox-attack-results-source-contract.json"
)
SOURCE_PATHS = (
    "packaging/macos/SANDBOX-ATTACK-RESULTS.md",
    "scripts/macos_sandbox_attack_results.py",
    "scripts/macos_signed_reference_package.py",
    "tests/test_macos_ipc_bookmark_receipts.py",
    "tests/test_macos_sandbox_attack_results.py",
)
ATTACK_CLASSES = (
    "ambient-home",
    "device",
    "process",
    "environment",
    "credential",
    "network",
    "workspace-write",
    "grant",
    "cross-user",
)
COMMON_FIELDS = (
    "source_revision",
    "version",
    "macos_build",
    "xcode_build",
    "architecture",
    "team_id",
    "package_sha256",
)
PROFILE_KEYS = {
    "component",
    "bundle_identifier",
    "designated_requirement",
    "component_sha256",
    "entitlement_keys",
    "app_sandbox_observed",
    "hardened_runtime_observed",
    "sandbox_profile_active",
    "network_entitlement_present",
    "workspace_write_entitlement_present",
    "temporary_exception_entitlement_present",
    "raw_report_sha256",
}
ATTACK_KEYS = {
    "case_id",
    "component",
    "attack_class",
    "status",
    "attempted",
    "unauthorized_access_count",
    "unauthorized_byte_count",
    "network_connection_count",
    "network_byte_count",
    "descendant_process_count",
    "residue_count",
    "canary_observed",
    "workspace_modified",
    "authority_broadened",
    "raw_report_sha256",
}
PROFILE_RECORD_KEYS = {
    "schema_version",
    "record_type",
    "status",
    *COMMON_FIELDS,
    "profiles",
    "credential_values_present",
    "private_environment_values_present",
}
ATTACK_RECORD_KEYS = {
    "schema_version",
    "record_type",
    "status",
    *COMMON_FIELDS,
    "attack_classes",
    "cases",
    "credential_values_present",
    "private_environment_values_present",
}
RESULTS_KEYS = {
    "schema_version",
    "record_type",
    "status",
    *COMMON_FIELDS,
    "profiles",
    "attacks",
    "raw_record_hashes",
    "credential_values_present",
    "private_environment_values_present",
    "release_claim",
}
ATTACKS_KEYS = {"attack_classes", "cases"}
RAW_RECORD_HASH_KEYS = {"profiles", "attacks"}
REVISION = re.compile(r"[0-9a-f]{40}")


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def safe_owner_only_directory(path: Path, label: str) -> list[str]:
    try:
        metadata = path.lstat()
    except OSError:
        return [f"{label} directory unavailable"]
    if (
        not path.is_absolute()
        or not stat.S_ISDIR(metadata.st_mode)
        or metadata.st_uid != os.geteuid()
        or metadata.st_mode & 0o077
    ):
        return [f"{label} directory is not owner-only"]
    return []


def safe_regular(path: Path, label: str, maximum: int = 16 * 1024 * 1024) -> list[str]:
    try:
        metadata = path.lstat()
    except OSError:
        return [f"{label} file unavailable: {path.name}"]
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_nlink != 1
        or metadata.st_uid != os.geteuid()
        or metadata.st_mode & 0o022
        or metadata.st_size > maximum
    ):
        return [f"unsafe {label} file: {path.name}"]
    return []


def exact_log_files(directory: Path, expected: tuple[str, ...], label: str) -> list[str]:
    failures = safe_owner_only_directory(directory, label)
    if failures:
        return failures
    try:
        actual = tuple(sorted(path.name for path in directory.iterdir()))
    except OSError:
        return [f"{label} log closure unavailable"]
    if actual != tuple(sorted(expected)):
        failures.append(f"{label} log closure changed")
        return failures
    for name in expected:
        failures.extend(safe_regular(directory / name, label))
    return failures


def load_release_inputs(
    policy_path: Path, manifest_path: Path, terminal_path: Path
) -> tuple[list[str], dict[str, Any] | None, dict[str, Any] | None]:
    failures: list[str] = []
    policy, found = read_closed_json(policy_path, POLICY_KEYS, 1024 * 1024)
    failures.extend(found)
    manifest, found = read_closed_json(manifest_path, MANIFEST_KEYS, 4 * 1024 * 1024)
    failures.extend(found)
    _terminal, found = read_closed_json(terminal_path, TERMINAL_KEYS, 1024 * 1024)
    failures.extend(found)
    return (
        failures,
        policy if isinstance(policy, dict) else None,
        manifest if isinstance(manifest, dict) else None,
    )


def common_expected(policy: dict[str, Any], package_sha256: str) -> dict[str, Any]:
    return {
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "macos_build": policy["expected_macos_build"],
        "xcode_build": policy["expected_xcode_build"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": package_sha256,
    }


def validate_common(value: dict[str, Any], expected: dict[str, Any], label: str) -> list[str]:
    return [
        f"{label} release binding mismatch: {key}"
        for key, expected_value in expected.items()
        if value.get(key) != expected_value
    ]


def validate_profiles(
    profiles: Any,
    policy: dict[str, Any],
    manifest: dict[str, Any],
    log_root: Path | None = None,
) -> list[str]:
    if not isinstance(profiles, list) or len(profiles) != len(COMPONENTS):
        return ["sandbox profile component closure changed"]
    failures: list[str] = []
    if [item.get("component") for item in profiles if isinstance(item, dict)] != list(COMPONENTS):
        failures.append("sandbox profile component order changed")
        return failures
    for component, item in zip(COMPONENTS, profiles, strict=True):
        if not isinstance(item, dict) or set(item) != PROFILE_KEYS:
            failures.append(f"sandbox profile fields changed: {component}")
            continue
        expected = {
            "component": component,
            "bundle_identifier": policy["bundle_identifiers"][component],
            "designated_requirement": manifest["code_identity"]["designated_requirements"][component],
            "component_sha256": manifest["component_hashes"][component],
            "entitlement_keys": EXPECTED_ENTITLEMENTS[component],
            "app_sandbox_observed": True,
            "hardened_runtime_observed": True,
            "sandbox_profile_active": True,
            "network_entitlement_present": False,
            "workspace_write_entitlement_present": False,
            "temporary_exception_entitlement_present": False,
        }
        for key, expected_value in expected.items():
            if item.get(key) != expected_value:
                failures.append(f"sandbox profile mismatch: {component}:{key}")
        if not valid_sha256(item.get("raw_report_sha256")):
            failures.append(f"sandbox profile raw digest invalid: {component}")
        elif log_root is not None and item["raw_report_sha256"] != sha256_file(log_root / f"{component}.log"):
            failures.append(f"sandbox profile raw digest mismatch: {component}")
    hashes = [item.get("raw_report_sha256") for item in profiles if isinstance(item, dict)]
    if len(hashes) == len(COMPONENTS) and len(set(hashes)) != len(COMPONENTS):
        failures.append("sandbox profile raw digests collide")
    return failures


def expected_attack_pairs() -> list[tuple[str, str]]:
    return [
        (component, attack_class)
        for component in COMPONENTS
        for attack_class in ATTACK_CLASSES
    ]


def attack_log_name(component: str, attack_class: str) -> str:
    return f"{component}-{attack_class}.log"


def validate_attacks(cases: Any, log_root: Path | None = None) -> list[str]:
    expected_pairs = expected_attack_pairs()
    if not isinstance(cases, list) or len(cases) != len(expected_pairs):
        return ["sandbox attack case closure changed"]
    failures: list[str] = []
    actual_pairs = [
        (item.get("component"), item.get("attack_class"))
        for item in cases
        if isinstance(item, dict)
    ]
    if actual_pairs != expected_pairs:
        failures.append("sandbox attack matrix order or uniqueness changed")
        return failures
    raw_hashes: list[Any] = []
    for (component, attack_class), item in zip(expected_pairs, cases, strict=True):
        if not isinstance(item, dict) or set(item) != ATTACK_KEYS:
            failures.append(f"sandbox attack fields changed: {component}:{attack_class}")
            continue
        expected = {
            "case_id": f"{component}:{attack_class}",
            "component": component,
            "attack_class": attack_class,
            "status": "passed-zero-unauthorized-access",
            "attempted": True,
            "unauthorized_access_count": 0,
            "unauthorized_byte_count": 0,
            "network_connection_count": 0,
            "network_byte_count": 0,
            "descendant_process_count": 0,
            "residue_count": 0,
            "canary_observed": False,
            "workspace_modified": False,
            "authority_broadened": False,
        }
        for key, expected_value in expected.items():
            if item.get(key) != expected_value:
                failures.append(f"sandbox attack mismatch: {component}:{attack_class}:{key}")
        raw_hash = item.get("raw_report_sha256")
        raw_hashes.append(raw_hash)
        if not valid_sha256(raw_hash):
            failures.append(f"sandbox attack raw digest invalid: {component}:{attack_class}")
        elif log_root is not None and raw_hash != sha256_file(log_root / attack_log_name(component, attack_class)):
            failures.append(f"sandbox attack raw digest mismatch: {component}:{attack_class}")
    if len(raw_hashes) == len(expected_pairs) and len(set(raw_hashes)) != len(expected_pairs):
        failures.append("sandbox attack raw digests collide")
    return failures


def validate_profile_record(
    value: Any,
    policy: dict[str, Any],
    manifest: dict[str, Any],
    package_sha256: str,
    log_root: Path,
) -> list[str]:
    if not isinstance(value, dict) or set(value) != PROFILE_RECORD_KEYS:
        return ["sandbox profile record field closure changed"]
    failures = validate_common(value, common_expected(policy, package_sha256), "sandbox profile")
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "macos-sandbox-profile-results"
        or value.get("status") != "passed"
        or value.get("credential_values_present") is not False
        or value.get("private_environment_values_present") is not False
    ):
        failures.append("sandbox profile record disposition is invalid")
    failures.extend(validate_profiles(value.get("profiles"), policy, manifest, log_root))
    if contains_prohibited_key(value):
        failures.append("sandbox profile record contains credential or environment material")
    return failures


def validate_attack_record(
    value: Any,
    policy: dict[str, Any],
    package_sha256: str,
    log_root: Path,
) -> list[str]:
    if not isinstance(value, dict) or set(value) != ATTACK_RECORD_KEYS:
        return ["sandbox attack record field closure changed"]
    failures = validate_common(value, common_expected(policy, package_sha256), "sandbox attack")
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "macos-sandbox-attack-results"
        or value.get("status") != "passed"
        or value.get("attack_classes") != list(ATTACK_CLASSES)
        or value.get("credential_values_present") is not False
        or value.get("private_environment_values_present") is not False
    ):
        failures.append("sandbox attack record disposition is invalid")
    failures.extend(validate_attacks(value.get("cases"), log_root))
    if contains_prohibited_key(value):
        failures.append("sandbox attack record contains credential or environment material")
    return failures


def assemble_results(
    policy_path: Path,
    manifest_path: Path,
    package_path: Path,
    terminal_path: Path,
    evidence_root: Path,
) -> tuple[list[str], dict[str, Any] | None]:
    failures, reference = validate_reference_bundle(
        policy_path, manifest_path, package_path, terminal_path
    )
    found, policy, manifest = load_release_inputs(policy_path, manifest_path, terminal_path)
    failures.extend(found)
    failures.extend(safe_owner_only_directory(evidence_root, "sandbox evidence"))
    if failures or reference is None or policy is None or manifest is None:
        return failures, None

    profile_log_root = evidence_root / "profile-logs"
    attack_log_root = evidence_root / "attack-logs"
    profile_names = tuple(f"{component}.log" for component in COMPONENTS)
    attack_names = tuple(attack_log_name(*pair) for pair in expected_attack_pairs())
    failures.extend(exact_log_files(profile_log_root, profile_names, "sandbox profile"))
    failures.extend(exact_log_files(attack_log_root, attack_names, "sandbox attack"))
    profile_path = evidence_root / "sandbox-profile-results.json"
    attack_path = evidence_root / "sandbox-attack-results.json"
    profiles, found = read_closed_json(profile_path, PROFILE_RECORD_KEYS, 4 * 1024 * 1024)
    failures.extend(found)
    attacks, found = read_closed_json(attack_path, ATTACK_RECORD_KEYS, 8 * 1024 * 1024)
    failures.extend(found)
    if failures or not isinstance(profiles, dict) or not isinstance(attacks, dict):
        return failures, None
    package_sha256 = reference["package_sha256"]
    failures.extend(validate_profile_record(profiles, policy, manifest, package_sha256, profile_log_root))
    failures.extend(validate_attack_record(attacks, policy, package_sha256, attack_log_root))
    if failures:
        return failures, None
    return [], {
        "schema_version": 1,
        "record_type": "macos-sandbox-profile-and-attack-results",
        "status": "native-sandbox-campaign-passed",
        **common_expected(policy, package_sha256),
        "profiles": profiles["profiles"],
        "attacks": {
            "attack_classes": attacks["attack_classes"],
            "cases": attacks["cases"],
        },
        "raw_record_hashes": {
            "profiles": sha256_file(profile_path),
            "attacks": sha256_file(attack_path),
        },
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
    }


def validate_results(
    policy_path: Path,
    manifest_path: Path,
    package_path: Path,
    terminal_path: Path,
    results_path: Path,
) -> tuple[list[str], dict[str, Any] | None]:
    failures, reference = validate_reference_bundle(
        policy_path, manifest_path, package_path, terminal_path
    )
    found, policy, manifest = load_release_inputs(policy_path, manifest_path, terminal_path)
    failures.extend(found)
    results, found = read_closed_json(results_path, RESULTS_KEYS, 16 * 1024 * 1024)
    failures.extend(found)
    if failures or reference is None or policy is None or manifest is None or not isinstance(results, dict):
        return failures, None
    expected = {
        "schema_version": 1,
        "record_type": "macos-sandbox-profile-and-attack-results",
        "status": "native-sandbox-campaign-passed",
        **common_expected(policy, reference["package_sha256"]),
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
    }
    for key, expected_value in expected.items():
        if results.get(key) != expected_value:
            failures.append(f"assembled sandbox evidence mismatch: {key}")
    failures.extend(validate_profiles(results.get("profiles"), policy, manifest))
    attacks = results.get("attacks")
    if not isinstance(attacks, dict) or set(attacks) != ATTACKS_KEYS:
        failures.append("assembled sandbox attack closure changed")
    else:
        if attacks.get("attack_classes") != list(ATTACK_CLASSES):
            failures.append("assembled sandbox attack class closure changed")
        failures.extend(validate_attacks(attacks.get("cases")))
    raw_hashes = results.get("raw_record_hashes")
    if (
        not isinstance(raw_hashes, dict)
        or set(raw_hashes) != RAW_RECORD_HASH_KEYS
        or any(not valid_sha256(value) for value in raw_hashes.values())
        or len(set(raw_hashes.values())) != len(RAW_RECORD_HASH_KEYS)
    ):
        failures.append("sandbox raw record hashes are invalid or collide")
    if contains_prohibited_key(results):
        failures.append("assembled sandbox evidence contains credential or environment material")
    if failures:
        return failures, None
    assert isinstance(raw_hashes, dict)
    return [], {
        "schema_version": 1,
        "record_type": "macos-sandbox-profile-and-attack-evidence",
        "status": "accepted-candidate-evidence",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": reference["package_sha256"],
        "results_sha256": sha256_file(results_path),
        "raw_record_hashes": raw_hashes,
        "profile_count": len(COMPONENTS),
        "attack_class_count": len(ATTACK_CLASSES),
        "attack_case_count": len(expected_attack_pairs()),
        "unauthorized_access_count": 0,
        "unauthorized_byte_count": 0,
        "native_operations_executed_by_ingestor": False,
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
        "macos_support_claim": False,
    }


def validate_sources(root: Path = ROOT) -> list[str]:
    failures = [
        f"missing source input: {relative}"
        for relative in SOURCE_PATHS
        if not (root / relative).is_file()
    ]
    if failures:
        return failures
    source = (root / "scripts/macos_sandbox_attack_results.py").read_text(encoding="utf-8")
    tests = (root / "tests/test_macos_sandbox_attack_results.py").read_text(encoding="utf-8")
    documentation = (root / "packaging/macos/SANDBOX-ATTACK-RESULTS.md").read_text(encoding="utf-8")
    for term in (
        '"sandbox_profile_active": True',
        '"network_entitlement_present": False',
        '"workspace_write_entitlement_present": False',
        '"temporary_exception_entitlement_present": False',
        '"passed-zero-unauthorized-access"',
        '"unauthorized_access_count": 0',
        '"network_connection_count": 0',
        '"authority_broadened": False',
        "expected_attack_pairs()",
        '"native_operations_executed_by_ingestor": False',
        '"macos_support_claim": False',
    ):
        if term not in source:
            failures.append(f"sandbox evidence verifier missing term: {term}")
    for prohibited in (
        "sandbox-" + "exec ",
        "subprocess.run([\"open\"",
        "socket." + "socket(",
        "cu" + "rl ",
        "reque" + "sts.",
    ):
        if prohibited in source:
            failures.append(f"sandbox evidence verifier contains prohibited effect: {prohibited}")
    if "Seven" not in tests or len(re.findall(r"^    def test_", tests, re.MULTILINE)) != 7:
        failures.append("sandbox evidence mutation corpus is not seven closed tests")
    for term in (
        "four native sandbox profiles",
        "closed 36-case hostile-access matrix",
        "never retains canary",
        "neither launches a native process",
        "source evidence\nonly",
        "`BLOCKED-MACOS`",
    ):
        if term not in documentation:
            failures.append(f"sandbox evidence documentation missing statement: {term}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    if not REVISION.fullmatch(revision):
        raise ValueError("reviewed sandbox evidence source revision is invalid")
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed sandbox evidence source changed: {relative}")
    return revision, tree


def build_source_report(root: Path = ROOT, *, source_revision: str = "HEAD") -> dict[str, Any]:
    failures = validate_sources(root)
    if failures:
        raise ValueError("; ".join(failures))
    revision, tree = resolve_revision(source_revision, root)
    return {
        "schema_version": 1,
        "record_type": "macos-sandbox-attack-results-source-contract",
        "task_id": "8.1.2.4",
        "status": "prepared-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "profile_count": len(COMPONENTS),
            "attack_class_count": len(ATTACK_CLASSES),
            "attack_case_count": len(expected_attack_pairs()),
            "raw_canary_values_retained": False,
            "raw_private_paths_retained": False,
            "native_operation_authority": False,
            "network_authority": False,
            "credential_authority": False,
            "package_copy_authority": False,
            "workspace_authority": False,
            "attack_authority": False,
            "support_promotion_authority": False,
        },
        "execution": {
            "native_profiles_observed": False,
            "native_attack_campaign_observed": False,
            "signed_components_executed": False,
            "hostile_access_cases_executed": False,
            "protected_logs_observed": False,
            "independent_review_performed": False,
        },
        "claims": {
            "task_complete": False,
            "native_sandbox_results_exist": False,
            "release_candidate_exists": False,
            "macos_support": False,
        },
        "remaining_blockers": [
            "No external active App Sandbox profile report exists for the four signed components.",
            "No external 36-case hostile-access campaign or protected raw logs exist.",
            "No native ambient-home, device, process, environment, credential, network, workspace-write, grant, or cross-user results exist.",
            "No native zero-access, zero-byte, zero-authority, and zero-residue reconciliation exists.",
            "No release-approved policy, signed package, release-derived manifest, or successful terminal record exists.",
            "No independent reviewer has reconciled and retained the sandbox evidence bundle.",
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
    parser.add_argument("--results-output", type=Path)
    parser.add_argument("--results", type=Path)
    parser.add_argument("--review-output", type=Path)
    arguments = parser.parse_args()
    try:
        if arguments.assemble:
            required = (
                arguments.policy, arguments.manifest, arguments.package,
                arguments.terminal, arguments.evidence_root, arguments.results_output,
            )
            if arguments.verify or arguments.write_source_contract or any(item is None for item in required):
                raise ValueError("sandbox assembly requires exactly its six path inputs")
            failures, report = assemble_results(*required[:-1])  # type: ignore[arg-type]
            if failures or report is None:
                raise ValueError("; ".join(failures))
            write_atomic(arguments.results_output, report)  # type: ignore[arg-type]
            print("macOS sandbox evidence assembled without native effects")
            return 0
        if arguments.verify:
            required = (
                arguments.policy, arguments.manifest, arguments.package,
                arguments.terminal, arguments.results, arguments.review_output,
            )
            if arguments.write_source_contract or any(item is None for item in required):
                raise ValueError("sandbox verification requires exactly its six path inputs")
            failures, result = validate_results(*required[:-1])  # type: ignore[arg-type]
            if failures or result is None:
                raise ValueError("; ".join(failures))
            write_atomic(arguments.review_output, result)  # type: ignore[arg-type]
            print("macOS sandbox evidence accepted without support promotion")
            return 0
        if any(
            item is not None
            for item in (
                arguments.policy, arguments.manifest, arguments.package,
                arguments.terminal, arguments.evidence_root, arguments.results_output,
                arguments.results, arguments.review_output,
            )
        ):
            raise ValueError("sandbox path inputs require --assemble or --verify")
        if arguments.write_source_contract:
            report = build_source_report(source_revision=arguments.source_revision)
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_bytes(canonical_bytes(report))
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained sandbox source contract has no source revision")
            expected = canonical_bytes(build_source_report(source_revision=revision))
            if REPORT_PATH.read_bytes() != expected:
                raise ValueError("sandbox evidence source contract is stale")
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS sandbox evidence verification failed: {error}")
        return 1
    print("macOS sandbox evidence source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Assemble and verify content-free S-008-RT01 recovery evidence."""

from __future__ import annotations

import argparse
import json
import os
import stat
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.macos_release_runner_source_contract import POLICY_KEYS
from scripts.macos_signed_reference_package import (
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
    "macos-lifecycle-recovery-results-source-contract.json"
)
SOURCE_PATHS = (
    "packaging/macos/LIFECYCLE-RECOVERY-RESULTS.md",
    (
        "platforms/macos/Tests/AgentMageMacOSPlatformTests/"
        "MacOSLifecycleRecoveryMatrixTests.swift"
    ),
    "scripts/macos_lifecycle_recovery_results.py",
    "scripts/macos_signed_reference_package.py",
    "tests/test_macos_ipc_bookmark_receipts.py",
    "tests/test_macos_lifecycle_recovery_results.py",
)
RECOVERY_SCENARIOS = (
    ("stale-bookmark", "refreshed-after-revalidation"),
    ("revoked-bookmark", "revoked-access-denied"),
    ("helper-crash", "helper-terminated-cleanly"),
    ("host-crash", "host-terminated-cleanly"),
    ("interrupted-install", "prior-package-preserved"),
    ("failed-launch", "prior-package-restored"),
    ("uninstall", "candidate-removed"),
    ("rollback", "prior-package-restored-and-launched"),
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
CASE_KEYS = {
    "scenario",
    "terminal_class",
    "status",
    "attempted",
    "cleanup_verified",
    "prior_valid_state_recovered",
    "descendant_process_count",
    "residue_count",
    "workspace_modified",
    "authority_broadened",
    "raw_report_sha256",
}
RAW_KEYS = {
    "schema_version",
    "record_type",
    "status",
    *COMMON_FIELDS,
    "scenario_order",
    "cases",
    "credential_values_present",
    "private_environment_values_present",
}
RESULTS_KEYS = {
    "schema_version",
    "record_type",
    "status",
    *COMMON_FIELDS,
    "cases",
    "raw_record_sha256",
    "credential_values_present",
    "private_environment_values_present",
    "release_claim",
}


def git(*arguments: str, root: Path = ROOT, binary: bool = False) -> str | bytes:
    output = subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True
    ).stdout
    return output if binary else output.decode().strip()


def scenario_names() -> tuple[str, ...]:
    return tuple(item[0] for item in RECOVERY_SCENARIOS)


def log_name(scenario: str) -> str:
    return f"{scenario}.log"


def safe_directory(path: Path, label: str) -> list[str]:
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


def safe_regular(path: Path, label: str) -> list[str]:
    try:
        metadata = path.lstat()
    except OSError:
        return [f"{label} file unavailable: {path.name}"]
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_nlink != 1
        or metadata.st_uid != os.geteuid()
        or metadata.st_mode & 0o022
        or metadata.st_size > 16 * 1024 * 1024
    ):
        return [f"unsafe {label} file: {path.name}"]
    return []


def exact_logs(directory: Path) -> list[str]:
    failures = safe_directory(directory, "lifecycle recovery log")
    if failures:
        return failures
    expected = tuple(sorted(log_name(name) for name in scenario_names()))
    try:
        actual = tuple(sorted(path.name for path in directory.iterdir()))
    except OSError:
        return ["lifecycle recovery log closure unavailable"]
    if actual != expected:
        return ["lifecycle recovery log closure changed"]
    for name in expected:
        failures.extend(safe_regular(directory / name, "lifecycle recovery log"))
    return failures


def load_policy(
    policy_path: Path, manifest_path: Path, terminal_path: Path
) -> tuple[list[str], dict[str, Any] | None]:
    failures: list[str] = []
    policy, found = read_closed_json(policy_path, POLICY_KEYS, 1024 * 1024)
    failures.extend(found)
    _manifest, found = read_closed_json(manifest_path, MANIFEST_KEYS, 4 * 1024 * 1024)
    failures.extend(found)
    _terminal, found = read_closed_json(terminal_path, TERMINAL_KEYS, 1024 * 1024)
    failures.extend(found)
    return failures, policy if isinstance(policy, dict) else None


def common(policy: dict[str, Any], package_sha256: str) -> dict[str, Any]:
    return {
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "macos_build": policy["expected_macos_build"],
        "xcode_build": policy["expected_xcode_build"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": package_sha256,
    }


def expected_case(scenario: str, terminal: str) -> dict[str, Any]:
    return {
        "scenario": scenario,
        "terminal_class": terminal,
        "status": "passed-cleanup-and-recovery",
        "attempted": True,
        "cleanup_verified": True,
        "prior_valid_state_recovered": True,
        "descendant_process_count": 0,
        "residue_count": 0,
        "workspace_modified": False,
        "authority_broadened": False,
    }


def validate_cases(cases: Any, log_root: Path | None = None) -> list[str]:
    if not isinstance(cases, list) or len(cases) != len(RECOVERY_SCENARIOS):
        return ["lifecycle recovery case closure changed"]
    if [item.get("scenario") for item in cases if isinstance(item, dict)] != list(
        scenario_names()
    ):
        return ["lifecycle recovery matrix order or uniqueness changed"]
    failures: list[str] = []
    hashes: list[Any] = []
    for (scenario, terminal), item in zip(RECOVERY_SCENARIOS, cases, strict=True):
        if not isinstance(item, dict) or set(item) != CASE_KEYS:
            failures.append(f"lifecycle recovery fields changed: {scenario}")
            continue
        for key, expected in expected_case(scenario, terminal).items():
            if item.get(key) != expected:
                failures.append(f"lifecycle recovery mismatch: {scenario}:{key}")
        raw_hash = item.get("raw_report_sha256")
        hashes.append(raw_hash)
        if not valid_sha256(raw_hash):
            failures.append(f"lifecycle recovery raw digest invalid: {scenario}")
        elif log_root is not None and raw_hash != sha256_file(
            log_root / log_name(scenario)
        ):
            failures.append(f"lifecycle recovery raw digest mismatch: {scenario}")
    if len(hashes) == len(RECOVERY_SCENARIOS) and len(set(hashes)) != len(hashes):
        failures.append("lifecycle recovery raw digests collide")
    return failures


def validate_raw(
    value: Any, policy: dict[str, Any], package_sha256: str, log_root: Path
) -> list[str]:
    if not isinstance(value, dict) or set(value) != RAW_KEYS:
        return ["lifecycle recovery record field closure changed"]
    failures = [
        f"lifecycle recovery release binding mismatch: {key}"
        for key, expected in common(policy, package_sha256).items()
        if value.get(key) != expected
    ]
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "macos-lifecycle-recovery-results"
        or value.get("status") != "passed"
        or value.get("scenario_order") != list(scenario_names())
        or value.get("credential_values_present") is not False
        or value.get("private_environment_values_present") is not False
    ):
        failures.append("lifecycle recovery record disposition is invalid")
    failures.extend(validate_cases(value.get("cases"), log_root))
    if contains_prohibited_key(value):
        failures.append("lifecycle recovery record contains credential material")
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
    found, policy = load_policy(policy_path, manifest_path, terminal_path)
    failures.extend(found)
    failures.extend(safe_directory(evidence_root, "lifecycle recovery evidence"))
    if failures or reference is None or policy is None:
        return failures, None
    log_root = evidence_root / "recovery-logs"
    failures.extend(exact_logs(log_root))
    raw_path = evidence_root / "lifecycle-recovery-results.json"
    raw, found = read_closed_json(raw_path, RAW_KEYS, 8 * 1024 * 1024)
    failures.extend(found)
    if failures or not isinstance(raw, dict):
        return failures, None
    failures.extend(validate_raw(raw, policy, reference["package_sha256"], log_root))
    if failures:
        return failures, None
    return [], {
        "schema_version": 1,
        "record_type": "macos-lifecycle-recovery-results",
        "status": "native-recovery-matrix-passed",
        **common(policy, reference["package_sha256"]),
        "cases": raw["cases"],
        "raw_record_sha256": sha256_file(raw_path),
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
    found, policy = load_policy(policy_path, manifest_path, terminal_path)
    failures.extend(found)
    results, found = read_closed_json(results_path, RESULTS_KEYS, 8 * 1024 * 1024)
    failures.extend(found)
    if failures or reference is None or policy is None or not isinstance(results, dict):
        return failures, None
    expected = {
        "schema_version": 1,
        "record_type": "macos-lifecycle-recovery-results",
        "status": "native-recovery-matrix-passed",
        **common(policy, reference["package_sha256"]),
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
    }
    for key, expected_value in expected.items():
        if results.get(key) != expected_value:
            failures.append(f"assembled lifecycle recovery mismatch: {key}")
    failures.extend(validate_cases(results.get("cases")))
    if not valid_sha256(results.get("raw_record_sha256")):
        failures.append("lifecycle recovery raw record digest is invalid")
    if contains_prohibited_key(results):
        failures.append("assembled lifecycle recovery contains credential material")
    if failures:
        return failures, None
    return [], {
        "schema_version": 1,
        "record_type": "macos-lifecycle-recovery-evidence",
        "status": "accepted-candidate-evidence",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": reference["package_sha256"],
        "results_sha256": sha256_file(results_path),
        "raw_record_sha256": results["raw_record_sha256"],
        "scenario_count": len(RECOVERY_SCENARIOS),
        "failed_cleanup_count": 0,
        "failed_prior_state_recovery_count": 0,
        "remaining_descendant_count": 0,
        "remaining_residue_count": 0,
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
    swift = (
        root
        / "platforms/macos/Tests/AgentMageMacOSPlatformTests/"
        "MacOSLifecycleRecoveryMatrixTests.swift"
    ).read_text(encoding="utf-8")
    source = (root / "scripts/macos_lifecycle_recovery_results.py").read_text(
        encoding="utf-8"
    )
    tests = (root / "tests/test_macos_lifecycle_recovery_results.py").read_text(
        encoding="utf-8"
    )
    documentation = (
        root / "packaging/macos/LIFECYCLE-RECOVERY-RESULTS.md"
    ).read_text(encoding="utf-8")
    for scenario, terminal in RECOVERY_SCENARIOS:
        if swift.count(f'"{scenario}"') != 1:
            failures.append(f"Swift lifecycle scenario is not exact: {scenario}")
        if swift.count(f'"{terminal}"') != 1:
            failures.append(f"Swift lifecycle terminal is not exact: {scenario}")
    for term in (
        "LifecycleRecoveryScenario.allCases.count == 8",
        "let result = try probe.exercise(scenario)",
        "result.scenario == scenario.rawValue",
        "result.terminalClass == scenario.expectedTerminalClass",
        "UnsafeRecoveryMutation.allCases.count == 9",
        '"cleanup_verified": True',
        '"prior_valid_state_recovered": True',
        '"native_operations_executed_by_ingestor": False',
        '"macos_support_claim": False',
    ):
        target = swift if term.startswith(("Lifecycle", "let ", "result.", "Unsafe")) else source
        if term not in target:
            failures.append(f"lifecycle recovery verifier missing term: {term}")
    if "Seven closed tests" not in tests:
        failures.append("lifecycle recovery mutation suite is not closed")
    for statement in (
        "exactly eight installed-candidate recovery",
        "No native recovery campaign",
        "has no bookmark, process, installer, filesystem, workspace",
    ):
        if statement not in documentation:
            failures.append(f"lifecycle recovery procedure missing statement: {statement}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed lifecycle recovery source changed: {relative}")
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
        "record_type": "macos-lifecycle-recovery-results-source-contract",
        "task_id": "8.1.3.3",
        "status": "prepared-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "scenario_count": len(RECOVERY_SCENARIOS),
            "unsafe_result_mutation_count": 9,
            "failed_cleanup_count": 0,
            "failed_prior_state_recovery_count": 0,
            "raw_private_values_retained": False,
            "native_operation_authority": False,
            "bookmark_authority": False,
            "process_authority": False,
            "installer_authority": False,
            "filesystem_authority": False,
            "workspace_authority": False,
            "credential_authority": False,
            "package_copy_authority": False,
            "fault_injection_authority": False,
            "support_promotion_authority": False,
        },
        "execution": {
            "swift_tests_performed": False,
            "native_recovery_campaign_observed": False,
            "signed_components_executed": False,
            "faults_injected": False,
            "protected_logs_observed": False,
            "independent_review_performed": False,
        },
        "claims": {
            "task_complete": False,
            "native_recovery_results_exist": False,
            "release_candidate_exists": False,
            "macos_support": False,
        },
        "remaining_blockers": [
            "No external Apple Silicon Swift 6 execution of the exact eight-scenario recovery matrix exists.",
            "No signed installed components have undergone bookmark, crash, install, launch, uninstall, or rollback faults.",
            "No native cleanup, prior-valid-state recovery, descendant, residue, or workspace-invariance results exist.",
            "No production release bundle, prior shipped package, or protected recovery logs exist.",
            "No independent reviewer has reconciled the native lifecycle recovery bundle.",
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
    args = parser.parse_args()
    try:
        if args.assemble:
            required = (
                args.policy,
                args.manifest,
                args.package,
                args.terminal,
                args.evidence_root,
                args.results_output,
            )
            if (
                args.verify
                or args.write_source_contract
                or any(item is None for item in required)
            ):
                raise ValueError("lifecycle assembly requires exactly its six path inputs")
            failures, result = assemble_results(*required[:-1])  # type: ignore[arg-type]
            if failures or result is None:
                raise ValueError("; ".join(failures))
            write_atomic(args.results_output, result)  # type: ignore[arg-type]
            print("macOS lifecycle recovery evidence assembled without native effects")
            return 0
        if args.verify:
            required = (
                args.policy,
                args.manifest,
                args.package,
                args.terminal,
                args.results,
                args.review_output,
            )
            if args.write_source_contract or any(item is None for item in required):
                raise ValueError("lifecycle verification requires exactly its six path inputs")
            failures, result = validate_results(*required[:-1])  # type: ignore[arg-type]
            if failures or result is None:
                raise ValueError("; ".join(failures))
            write_atomic(args.review_output, result)  # type: ignore[arg-type]
            print("macOS lifecycle recovery evidence accepted without support promotion")
            return 0
        if any(
            item is not None
            for item in (
                args.policy,
                args.manifest,
                args.package,
                args.terminal,
                args.evidence_root,
                args.results_output,
                args.results,
                args.review_output,
            )
        ):
            raise ValueError("lifecycle recovery paths require --assemble or --verify")
        if args.write_source_contract:
            report = build_source_report(source_revision=args.source_revision)
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_bytes(canonical_bytes(report))
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained lifecycle recovery contract has no revision")
            expected = canonical_bytes(build_source_report(source_revision=revision))
            if REPORT_PATH.read_bytes() != expected:
                raise ValueError("lifecycle recovery source contract is stale")
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS lifecycle recovery evidence verification failed: {error}")
        return 1
    print("macOS lifecycle recovery source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

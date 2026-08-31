#!/usr/bin/env python3
"""Assemble and verify content-free S-008-UT01 native refusal evidence."""

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
    "macos-ipc-handshake-refusal-results-source-contract.json"
)
SOURCE_PATHS = (
    "packaging/macos/IPC-HANDSHAKE-REFUSAL-RESULTS.md",
    "platforms/macos/Sources/AgentMageMacOSPlatform/AppGroupSocketBoundary.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSBridgeProtocol.swift",
    "platforms/macos/Sources/AgentMageMacOSPlatform/MacOSPeerVerification.swift",
    "platforms/macos/Tests/AgentMageMacOSPlatformTests/MacOSHandshakeRefusalMatrixTests.swift",
    "scripts/macos_ipc_handshake_refusal_results.py",
    "scripts/macos_signed_reference_package.py",
    "tests/test_macos_ipc_bookmark_receipts.py",
    "tests/test_macos_ipc_handshake_refusal_results.py",
)

# case id, mutation dimension, mutation, exact expected native refusal
REFUSAL_CASES = (
    ("code-identity.absent", "code-identity", "absent", "macos.ipc.peer.invalid-code-signature"),
    ("code-identity.wrong", "code-identity", "wrong", "macos.ipc.peer.designated-requirement-mismatch"),
    ("audit-token.absent", "audit-token", "absent", "macos.ipc.peer.audit-token-missing"),
    ("audit-token.wrong", "audit-token", "wrong", "macos.ipc.peer.audit-token-size-mismatch"),
    ("team-id.absent", "team-id", "absent", "macos.ipc.peer.wrong-team-identifier"),
    ("team-id.wrong", "team-id", "wrong", "macos.ipc.peer.wrong-team-identifier"),
    ("bundle-id.absent", "bundle-id", "absent", "macos.ipc.peer.wrong-bundle-identifier"),
    ("bundle-id.wrong", "bundle-id", "wrong", "macos.ipc.peer.wrong-bundle-identifier"),
    ("app-group.absent", "app-group", "absent", "macos.ipc.peer.wrong-app-group"),
    ("app-group.wrong", "app-group", "wrong", "macos.ipc.peer.wrong-app-group"),
    ("protocol-version.absent", "protocol-version", "absent", "macos.ipc.malformed-frame"),
    ("protocol-version.wrong", "protocol-version", "wrong", "macos.ipc.version-mismatch"),
    ("fresh-challenge.absent", "fresh-challenge", "absent", "macos.ipc.malformed-frame"),
    ("fresh-challenge.wrong", "fresh-challenge", "wrong", "macos.ipc.challenge-mismatch"),
    ("fresh-challenge.replay", "fresh-challenge", "replay", "macos.ipc.replay"),
    ("socket-mode.absent", "socket-mode", "absent", "macos.ipc.socket.wrong-socket-type"),
    ("socket-mode.wrong", "socket-mode", "wrong", "macos.ipc.socket.wrong-socket-mode"),
)
REQUIRED_CONTROLS = (
    "code-identity",
    "audit-token",
    "team-id",
    "bundle-app-group-identity",
    "protocol-version",
    "fresh-challenge",
    "socket-mode",
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
    "case_id",
    "dimension",
    "mutation",
    "expected_failure",
    "status",
    "mutated_handshake_refused",
    "precondition_valid_handshake_accepted",
    "valid_handshake_accepted_after_refusal",
    "challenge_consumed_only_by_valid_handshake",
    "raw_report_sha256",
}
RAW_RECORD_KEYS = {
    "schema_version",
    "record_type",
    "status",
    *COMMON_FIELDS,
    "required_controls",
    "case_order",
    "cases",
    "credential_values_present",
    "private_environment_values_present",
}
RESULTS_KEYS = {
    "schema_version",
    "record_type",
    "status",
    *COMMON_FIELDS,
    "required_controls",
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


def case_ids() -> tuple[str, ...]:
    return tuple(item[0] for item in REFUSAL_CASES)


def log_name(case_id: str) -> str:
    return f"{case_id}.log"


def exact_log_files(directory: Path) -> list[str]:
    failures = safe_owner_only_directory(directory, "handshake refusal log")
    if failures:
        return failures
    expected = tuple(sorted(log_name(case_id) for case_id in case_ids()))
    try:
        actual = tuple(sorted(path.name for path in directory.iterdir()))
    except OSError:
        return ["handshake refusal log closure unavailable"]
    if actual != expected:
        return ["handshake refusal log closure changed"]
    for name in expected:
        failures.extend(safe_regular(directory / name, "handshake refusal log"))
    return failures


def load_release_inputs(
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


def expected_case(case: tuple[str, str, str, str]) -> dict[str, Any]:
    case_id, dimension, mutation, failure = case
    replay = mutation == "replay"
    return {
        "case_id": case_id,
        "dimension": dimension,
        "mutation": mutation,
        "expected_failure": failure,
        "status": "passed-handshake-refused",
        "mutated_handshake_refused": True,
        "precondition_valid_handshake_accepted": replay,
        "valid_handshake_accepted_after_refusal": not replay,
        "challenge_consumed_only_by_valid_handshake": True,
    }


def validate_cases(cases: Any, log_root: Path | None = None) -> list[str]:
    if not isinstance(cases, list) or len(cases) != len(REFUSAL_CASES):
        return ["handshake refusal case closure changed"]
    failures: list[str] = []
    if [item.get("case_id") for item in cases if isinstance(item, dict)] != list(case_ids()):
        return ["handshake refusal matrix order or uniqueness changed"]
    raw_hashes: list[Any] = []
    for definition, item in zip(REFUSAL_CASES, cases, strict=True):
        case_id = definition[0]
        if not isinstance(item, dict) or set(item) != CASE_KEYS:
            failures.append(f"handshake refusal fields changed: {case_id}")
            continue
        for key, expected_value in expected_case(definition).items():
            if item.get(key) != expected_value:
                failures.append(f"handshake refusal mismatch: {case_id}:{key}")
        raw_hash = item.get("raw_report_sha256")
        raw_hashes.append(raw_hash)
        if not valid_sha256(raw_hash):
            failures.append(f"handshake refusal raw digest invalid: {case_id}")
        elif log_root is not None and raw_hash != sha256_file(log_root / log_name(case_id)):
            failures.append(f"handshake refusal raw digest mismatch: {case_id}")
    if len(raw_hashes) == len(REFUSAL_CASES) and len(set(raw_hashes)) != len(REFUSAL_CASES):
        failures.append("handshake refusal raw digests collide")
    return failures


def validate_raw_record(
    value: Any,
    policy: dict[str, Any],
    package_sha256: str,
    log_root: Path,
) -> list[str]:
    if not isinstance(value, dict) or set(value) != RAW_RECORD_KEYS:
        return ["handshake refusal record field closure changed"]
    failures = validate_common(
        value, common_expected(policy, package_sha256), "handshake refusal"
    )
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "macos-ipc-handshake-refusal-results"
        or value.get("status") != "passed"
        or value.get("required_controls") != list(REQUIRED_CONTROLS)
        or value.get("case_order") != list(case_ids())
        or value.get("credential_values_present") is not False
        or value.get("private_environment_values_present") is not False
    ):
        failures.append("handshake refusal record disposition is invalid")
    failures.extend(validate_cases(value.get("cases"), log_root))
    if contains_prohibited_key(value):
        failures.append("handshake refusal record contains credential or environment material")
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
    found, policy = load_release_inputs(policy_path, manifest_path, terminal_path)
    failures.extend(found)
    failures.extend(safe_owner_only_directory(evidence_root, "handshake refusal evidence"))
    if failures or reference is None or policy is None:
        return failures, None
    log_root = evidence_root / "refusal-logs"
    failures.extend(exact_log_files(log_root))
    raw_path = evidence_root / "ipc-handshake-refusal-results.json"
    raw, found = read_closed_json(raw_path, RAW_RECORD_KEYS, 8 * 1024 * 1024)
    failures.extend(found)
    if failures or not isinstance(raw, dict):
        return failures, None
    failures.extend(
        validate_raw_record(raw, policy, reference["package_sha256"], log_root)
    )
    if failures:
        return failures, None
    return [], {
        "schema_version": 1,
        "record_type": "macos-ipc-handshake-refusal-results",
        "status": "native-refusal-matrix-passed",
        **common_expected(policy, reference["package_sha256"]),
        "required_controls": list(REQUIRED_CONTROLS),
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
    found, policy = load_release_inputs(policy_path, manifest_path, terminal_path)
    failures.extend(found)
    results, found = read_closed_json(results_path, RESULTS_KEYS, 8 * 1024 * 1024)
    failures.extend(found)
    if failures or reference is None or policy is None or not isinstance(results, dict):
        return failures, None
    expected = {
        "schema_version": 1,
        "record_type": "macos-ipc-handshake-refusal-results",
        "status": "native-refusal-matrix-passed",
        **common_expected(policy, reference["package_sha256"]),
        "required_controls": list(REQUIRED_CONTROLS),
        "credential_values_present": False,
        "private_environment_values_present": False,
        "release_claim": "signed-package-candidate",
    }
    for key, expected_value in expected.items():
        if results.get(key) != expected_value:
            failures.append(f"assembled handshake refusal evidence mismatch: {key}")
    failures.extend(validate_cases(results.get("cases")))
    if not valid_sha256(results.get("raw_record_sha256")):
        failures.append("handshake refusal raw record digest is invalid")
    if contains_prohibited_key(results):
        failures.append("assembled handshake refusal evidence contains credential material")
    if failures:
        return failures, None
    return [], {
        "schema_version": 1,
        "record_type": "macos-ipc-handshake-refusal-evidence",
        "status": "accepted-candidate-evidence",
        "source_revision": policy["source_revision"],
        "version": policy["version"],
        "architecture": "arm64",
        "team_id": policy["team_id"],
        "package_sha256": reference["package_sha256"],
        "results_sha256": sha256_file(results_path),
        "raw_record_sha256": results["raw_record_sha256"],
        "required_control_count": len(REQUIRED_CONTROLS),
        "mutation_dimension_count": len({item[1] for item in REFUSAL_CASES}),
        "refusal_case_count": len(REFUSAL_CASES),
        "accepted_mutated_handshake_count": 0,
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
        "MacOSHandshakeRefusalMatrixTests.swift"
    ).read_text(encoding="utf-8")
    native_source = "\n".join(
        (
            root
            / "platforms/macos/Sources/AgentMageMacOSPlatform/"
            / name
        ).read_text(encoding="utf-8")
        for name in (
            "AppGroupSocketBoundary.swift",
            "MacOSBridgeProtocol.swift",
            "MacOSPeerVerification.swift",
        )
    )
    source = (root / "scripts/macos_ipc_handshake_refusal_results.py").read_text(
        encoding="utf-8"
    )
    tests = (root / "tests/test_macos_ipc_handshake_refusal_results.py").read_text(
        encoding="utf-8"
    )
    documentation = (
        root / "packaging/macos/IPC-HANDSHAKE-REFUSAL-RESULTS.md"
    ).read_text(encoding="utf-8")
    for case_id, _dimension, _mutation, failure in REFUSAL_CASES:
        if swift.count(f'"{case_id}"') != 1:
            failures.append(f"Swift refusal case is not exact: {case_id}")
        if failure not in native_source:
            failures.append(f"Swift refusal outcome is missing: {case_id}")
    for term in (
        "HandshakeRefusalMutation.allCases.count == 17",
        "try AppGroupSocketBoundary.validate(fixture.socket)",
        "try MacOSHandshakeFrame.decode(fixture.frameBytes)",
        "try admission.authenticate(observation: fixture.peer, frame: frame)",
        "try attemptHandshake(try refusalBaseline(), admission: admission)",
        '"accepted_mutated_handshake_count": 0',
        '"native_operations_executed_by_ingestor": False',
        '"macos_support_claim": False',
    ):
        target = swift if term.startswith(("try ", "Handshake")) else source
        if term not in target:
            failures.append(f"handshake refusal verifier missing term: {term}")
    if "Seven closed tests" not in tests:
        failures.append("handshake refusal mutation suite is not closed")
    for statement in (
        "exact 17-case refusal matrix",
        "No native signed host or bridge",
        "has no socket, process, signing, credential, or support-promotion authority",
    ):
        if statement not in documentation:
            failures.append(f"handshake refusal procedure missing statement: {statement}")
    return failures


def resolve_revision(source_revision: str, root: Path = ROOT) -> tuple[str, str]:
    revision = str(git("rev-parse", source_revision, root=root))
    tree = str(git("rev-parse", f"{revision}^{{tree}}", root=root))
    for relative in SOURCE_PATHS:
        committed = git("show", f"{revision}:{relative}", root=root, binary=True)
        if committed != (root / relative).read_bytes():
            raise ValueError(f"reviewed handshake refusal source changed: {relative}")
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
        "record_type": "macos-ipc-handshake-refusal-results-source-contract",
        "task_id": "8.1.3.1",
        "status": "prepared-source-only-blocked-macos",
        "source_revision": revision,
        "source_tree": tree,
        "source_files": [
            {"path": relative, "sha256": sha256_file(root / relative)}
            for relative in SOURCE_PATHS
        ],
        "contract": {
            "required_control_count": len(REQUIRED_CONTROLS),
            "mutation_dimension_count": len({item[1] for item in REFUSAL_CASES}),
            "refusal_case_count": len(REFUSAL_CASES),
            "accepted_mutated_handshake_count": 0,
            "raw_audit_tokens_retained": False,
            "raw_credentials_retained": False,
            "raw_private_paths_retained": False,
            "native_operation_authority": False,
            "socket_authority": False,
            "process_authority": False,
            "credential_authority": False,
            "package_copy_authority": False,
            "workspace_authority": False,
            "support_promotion_authority": False,
        },
        "execution": {
            "swift_tests_performed": False,
            "native_refusal_matrix_observed": False,
            "signed_host_executed": False,
            "signed_bridge_executed": False,
            "app_group_socket_observed": False,
            "protected_logs_observed": False,
            "independent_review_performed": False,
        },
        "claims": {
            "task_complete": False,
            "native_handshake_refusal_results_exist": False,
            "release_candidate_exists": False,
            "macos_support": False,
        },
        "remaining_blockers": [
            "No external Apple Silicon Swift 6 execution of the exact 17-case refusal matrix exists.",
            "No signed installed host and bridge have exercised a live App Group socket.",
            "No native absent, wrong, or replay refusal results and protected raw logs exist.",
            "No production release policy, signed package, release-derived manifest, or successful terminal record exists.",
            "No independent reviewer has reconciled the exact native refusal bundle.",
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
                arguments.policy,
                arguments.manifest,
                arguments.package,
                arguments.terminal,
                arguments.evidence_root,
                arguments.results_output,
            )
            if arguments.verify or arguments.write_source_contract or any(
                item is None for item in required
            ):
                raise ValueError("handshake refusal assembly requires exactly its six path inputs")
            failures, result = assemble_results(*required[:-1])  # type: ignore[arg-type]
            if failures or result is None:
                raise ValueError("; ".join(failures))
            write_atomic(arguments.results_output, result)  # type: ignore[arg-type]
            print("macOS handshake refusal evidence assembled without native effects")
            return 0
        if arguments.verify:
            required = (
                arguments.policy,
                arguments.manifest,
                arguments.package,
                arguments.terminal,
                arguments.results,
                arguments.review_output,
            )
            if arguments.write_source_contract or any(item is None for item in required):
                raise ValueError("handshake refusal verification requires exactly its six path inputs")
            failures, result = validate_results(*required[:-1])  # type: ignore[arg-type]
            if failures or result is None:
                raise ValueError("; ".join(failures))
            write_atomic(arguments.review_output, result)  # type: ignore[arg-type]
            print("macOS handshake refusal evidence accepted without support promotion")
            return 0
        if any(
            item is not None
            for item in (
                arguments.policy,
                arguments.manifest,
                arguments.package,
                arguments.terminal,
                arguments.evidence_root,
                arguments.results_output,
                arguments.results,
                arguments.review_output,
            )
        ):
            raise ValueError("handshake refusal paths require --assemble or --verify")
        if arguments.write_source_contract:
            report = build_source_report(source_revision=arguments.source_revision)
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_bytes(canonical_bytes(report))
        else:
            retained = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
            revision = retained.get("source_revision")
            if not isinstance(revision, str):
                raise ValueError("retained handshake refusal contract has no revision")
            expected = canonical_bytes(build_source_report(source_revision=revision))
            if REPORT_PATH.read_bytes() != expected:
                raise ValueError("handshake refusal source contract is stale")
    except (OSError, subprocess.CalledProcessError, ValueError, json.JSONDecodeError) as error:
        print(f"macOS handshake refusal evidence verification failed: {error}")
        return 1
    print("macOS handshake refusal source contract passed without native promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Record allowlisted platform identity for synthetic test results."""

from __future__ import annotations

import argparse
import hashlib
import json
import locale
import os
import platform
import re
import sys
from pathlib import Path
from typing import Any, Mapping


ROOT = Path(__file__).resolve().parents[1]
PROFILE_PATH = ROOT / "fixtures" / "platform-result-profile.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "platform-result-recorder-report.json"
)
EXPECTED_ENVIRONMENT_FIELDS = (
    "os_family",
    "distribution_id",
    "distribution_version",
    "architecture",
    "kernel_release",
    "python_implementation",
    "python_version",
    "text_encoding",
    "privilege_class",
    "containerized",
    "continuous_integration",
)
EXPECTED_FORBIDDEN_FIELDS = (
    "hostname",
    "username",
    "uid",
    "gid",
    "home_directory",
    "working_directory",
    "executable_path",
    "environment_variables",
    "ip_address",
    "mac_address",
    "hardware_serial",
    "timezone",
    "locale_region",
)
EXPECTED_RESULT_IDENTITIES = ("build", "fixture_set", "model", "runtime", "policy")
EXPECTED_SIDE_EFFECTS = {
    "executes_external_commands": False,
    "uses_network": False,
    "writes_environment": False,
    "reads_secret_stores": False,
    "persists_ambient_environment_values": False,
}
RESULT_STATUSES = {"pass", "fail", "skipped", "cancelled"}
CI_KEYS = ("CI", "GITHUB_ACTIONS", "GITLAB_CI", "BUILD_BUILDID", "TF_BUILD")
IDENTIFIER = re.compile(r"^[a-z0-9][a-z0-9._-]{0,63}$")
VERSION = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._+-]{0,127}$")
HASH = re.compile(r"^[0-9a-f]{64}$")
TIMESTAMP = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
SENSITIVE_VALUE = re.compile(
    r"(?:token|secret|password|passwd|private|bearer|api[_-]?key|gh[pousr]_)",
    re.IGNORECASE,
)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def parse_os_release(path: Path = Path("/etc/os-release")) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" not in line or line.startswith("#"):
            continue
        key, value = line.split("=", 1)
        if key in {"ID", "VERSION_ID"}:
            values[key] = value.strip().strip('"')
    return values


def enabled(value: str | None) -> bool:
    return bool(value and value.strip().lower() not in {"0", "false", "no", "off"})


def collect_current_linux_environment(
    environment: Mapping[str, str] | None = None,
    os_release_path: Path = Path("/etc/os-release"),
) -> dict[str, Any]:
    if platform.system().lower() != "linux":
        raise OSError("current platform collection is implemented only for Linux")
    ambient = os.environ if environment is None else environment
    os_release = parse_os_release(os_release_path)
    if "ID" not in os_release or "VERSION_ID" not in os_release:
        raise OSError("Linux distribution identity is incomplete")
    return {
        "architecture": platform.machine().lower(),
        "containerized": (
            Path("/.dockerenv").exists()
            or Path("/run/.containerenv").exists()
            or enabled(ambient.get("container"))
        ),
        "continuous_integration": any(enabled(ambient.get(key)) for key in CI_KEYS),
        "distribution_id": os_release["ID"].lower(),
        "distribution_version": os_release["VERSION_ID"],
        "kernel_release": platform.release(),
        "os_family": "linux",
        "privilege_class": "privileged" if os.geteuid() == 0 else "standard-user",
        "python_implementation": platform.python_implementation().lower(),
        "python_version": platform.python_version(),
        "text_encoding": locale.getencoding().lower().replace("_", "-"),
    }


def validate_environment(environment: Any) -> list[str]:
    if not isinstance(environment, dict):
        return ["platform environment must be an object"]
    failures: list[str] = []
    if set(environment) != set(EXPECTED_ENVIRONMENT_FIELDS):
        failures.append("platform environment fields do not match the allowlist")
        return failures
    identifier_fields = ("distribution_id", "architecture", "python_implementation")
    for field in identifier_fields:
        value = environment[field]
        if not isinstance(value, str) or not IDENTIFIER.fullmatch(value):
            failures.append(f"platform environment identifier is invalid: {field}")
    if environment["os_family"] != "linux":
        failures.append("platform recorder currently admits Linux environment records only")
    for field in ("distribution_version", "kernel_release", "python_version", "text_encoding"):
        value = environment[field]
        if not isinstance(value, str) or not VERSION.fullmatch(value):
            failures.append(f"platform environment version is invalid: {field}")
    for field, value in environment.items():
        if isinstance(value, str) and SENSITIVE_VALUE.search(value):
            failures.append(f"platform environment value resembles sensitive material: {field}")
    if environment["privilege_class"] not in {"standard-user", "privileged"}:
        failures.append("platform privilege class is invalid")
    for field in ("containerized", "continuous_integration"):
        if not isinstance(environment[field], bool):
            failures.append(f"platform environment boolean is invalid: {field}")
    return failures


def validate_result_context(result: Any) -> list[str]:
    if not isinstance(result, dict):
        return ["platform result context must be an object"]
    failures: list[str] = []
    expected_fields = set(EXPECTED_RESULT_IDENTITIES) | {"result_id", "status"}
    if set(result) != expected_fields:
        failures.append("platform result context fields do not match the contract")
        return failures
    if not isinstance(result["result_id"], str) or not IDENTIFIER.fullmatch(result["result_id"]):
        failures.append("platform result identity is invalid")
    if result["status"] not in RESULT_STATUSES:
        failures.append("platform result status is invalid")
    for field in EXPECTED_RESULT_IDENTITIES:
        identity = result[field]
        if not isinstance(identity, dict) or set(identity) != {"id", "sha256"}:
            failures.append(f"platform result identity record is invalid: {field}")
            continue
        if not isinstance(identity["id"], str) or not IDENTIFIER.fullmatch(identity["id"]):
            failures.append(f"platform result identity name is invalid: {field}")
        if not isinstance(identity["sha256"], str) or not HASH.fullmatch(identity["sha256"]):
            failures.append(f"platform result hash is invalid: {field}")
    return failures


def record_platform_result(
    environment: dict[str, Any],
    result: dict[str, Any],
    captured_at: str,
    allowlist_version: str = "platform-environment-v1",
) -> dict[str, Any]:
    failures = validate_environment(environment) + validate_result_context(result)
    if not TIMESTAMP.fullmatch(captured_at):
        failures.append("platform result timestamp is invalid")
    if allowlist_version != "platform-environment-v1":
        failures.append("platform environment allowlist version is invalid")
    if failures:
        raise ValueError("; ".join(failures))
    identity_material = {
        "allowlist_version": allowlist_version,
        "captured_at": captured_at,
        "environment": environment,
        "result": result,
    }
    record = {
        "schema_version": 1,
        "record_type": "platform-result-environment",
        "record_id": f"am-platform-result-{sha256_bytes(canonical_json(identity_material))[:20]}",
        "captured_at": captured_at,
        "environment": dict(sorted(environment.items())),
        "result": {
            **{field: result[field] for field in EXPECTED_RESULT_IDENTITIES},
            "result_id": result["result_id"],
            "status": result["status"],
        },
        "redaction": {
            "allowlist_version": allowlist_version,
            "ambient_environment_values_recorded": False,
            "forbidden_field_count": len(EXPECTED_FORBIDDEN_FIELDS),
            "private_paths_recorded": False,
            "secret_store_values_recorded": False,
        },
        "macos_support_claim": "none",
    }
    record["record_sha256"] = sha256_bytes(canonical_json(record))
    return record


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


def validate_record(record: Any) -> list[str]:
    if not isinstance(record, dict):
        return ["platform result record must be an object"]
    failures: list[str] = []
    expected_fields = {
        "schema_version",
        "record_type",
        "record_id",
        "captured_at",
        "environment",
        "result",
        "redaction",
        "macos_support_claim",
        "record_sha256",
    }
    if set(record) != expected_fields:
        failures.append("platform result record fields do not match the contract")
        return failures
    if record["schema_version"] != 1 or record["record_type"] != "platform-result-environment":
        failures.append("platform result record identity is invalid")
    failures.extend(validate_environment(record["environment"]))
    failures.extend(validate_result_context(record["result"]))
    if record["macos_support_claim"] != "none":
        failures.append("platform result record made a macOS support claim")
    if nested_keys(record) & set(EXPECTED_FORBIDDEN_FIELDS):
        failures.append("platform result record contains a forbidden field")
    expected_redaction = {
        "allowlist_version": "platform-environment-v1",
        "ambient_environment_values_recorded": False,
        "forbidden_field_count": len(EXPECTED_FORBIDDEN_FIELDS),
        "private_paths_recorded": False,
        "secret_store_values_recorded": False,
    }
    if record["redaction"] != expected_redaction:
        failures.append("platform result redaction contract was weakened")
    unhashed = dict(record)
    recorded_hash = unhashed.pop("record_sha256")
    if not isinstance(recorded_hash, str) or sha256_bytes(canonical_json(unhashed)) != recorded_hash:
        failures.append("platform result record hash is invalid")
    return failures


def validate_profile(profile: Any) -> list[str]:
    if not isinstance(profile, dict):
        return ["platform result profile must be an object"]
    failures: list[str] = []
    if (
        profile.get("schema_version") != 1
        or profile.get("profile_id") != "agentmage-platform-result-recorder-v1"
        or profile.get("status") != "synthetic-platform-result-contract"
    ):
        failures.append("platform result profile identity is invalid")
    if profile.get("allowlist_version") != "platform-environment-v1":
        failures.append("platform result allowlist version drifted")
    if tuple(profile.get("allowed_environment_fields", [])) != EXPECTED_ENVIRONMENT_FIELDS:
        failures.append("platform result environment allowlist drifted")
    if tuple(profile.get("forbidden_environment_fields", [])) != EXPECTED_FORBIDDEN_FIELDS:
        failures.append("platform result forbidden-field closure drifted")
    if tuple(profile.get("required_result_identities", [])) != EXPECTED_RESULT_IDENTITIES:
        failures.append("platform result identity closure drifted")
    if profile.get("hash_algorithm") != "sha256-canonical-json":
        failures.append("platform result hash algorithm drifted")
    if profile.get("side_effect_contract") != EXPECTED_SIDE_EFFECTS:
        failures.append("platform result side-effect contract was weakened")
    if profile.get("macos_execution_status") != "blocked-macos":
        failures.append("platform result profile lost blocked macOS status")
    runs = profile.get("synthetic_runs")
    if not isinstance(runs, list) or len(runs) != 2:
        failures.append("platform result synthetic-run closure drifted")
        return failures
    for run in runs:
        if not isinstance(run, dict) or set(run) != {"captured_at", "environment", "result"}:
            failures.append("platform result synthetic run is malformed")
            continue
        try:
            record = record_platform_result(
                run["environment"],
                run["result"],
                run["captured_at"],
                profile.get("allowlist_version", ""),
            )
        except ValueError as error:
            failures.append(str(error))
            continue
        failures.extend(validate_record(record))
    return failures


def record_set_sha256(records: list[dict[str, Any]]) -> str:
    digest = hashlib.sha256()
    for record in records:
        content = canonical_json(record)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def build_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / PROFILE_PATH.relative_to(ROOT)
    profile = read_json(profile_path)
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    records = [
        record_platform_result(
            run["environment"],
            run["result"],
            run["captured_at"],
            profile["allowlist_version"],
        )
        for run in profile["synthetic_runs"]
    ]
    return {
        "schema_version": 1,
        "task_id": "2.1.1.7",
        "status": "pass",
        "profile": {
            "id": profile["profile_id"],
            "sha256": sha256_bytes(profile_path.read_bytes()),
        },
        "allowlist": {
            "allowed_environment_fields": list(EXPECTED_ENVIRONMENT_FIELDS),
            "forbidden_environment_fields": list(EXPECTED_FORBIDDEN_FIELDS),
            "version": profile["allowlist_version"],
        },
        "synthetic_record_set": {
            "record_count": len(records),
            "sha256": record_set_sha256(records),
            "records": records,
        },
        "side_effect_contract": EXPECTED_SIDE_EFFECTS,
        "ambient_environment_values_recorded": False,
        "current_host_record_persisted": False,
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["platform result recorder report must be an object"]
    failures: list[str] = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.1.7":
        failures.append("platform result recorder report identity is invalid")
    if report.get("status") != "pass":
        failures.append("platform result recorder did not pass")
    if report.get("ambient_environment_values_recorded") is not False:
        failures.append("platform result recorder retained ambient environment values")
    if report.get("current_host_record_persisted") is not False:
        failures.append("platform result recorder persisted current-host identity")
    if report.get("macos_execution_status") != "blocked-macos":
        failures.append("platform result recorder lost blocked macOS status")
    if report.get("macos_support_claim") != "none":
        failures.append("platform result recorder made a macOS support claim")
    if report != build_report(root):
        failures.append("platform result recorder report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    output = root / REPORT_PATH.relative_to(ROOT)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read platform result recorder report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--current", action="store_true")
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        if args.write_report:
            write_report()
        failures = check_report()
        if args.current:
            current = collect_current_linux_environment()
            current_failures = validate_environment(current)
            if current_failures:
                raise ValueError("; ".join(current_failures))
            print(json.dumps(current, indent=2, sort_keys=True))
    except (OSError, ValueError) as error:
        print(f"platform result recorder failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"platform result recorder failed: {failure}", file=sys.stderr)
        return 1
    if not args.current:
        print("allowlisted platform result recorder validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

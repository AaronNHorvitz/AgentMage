#!/usr/bin/env python3
"""Build and validate hostile network-surface injection evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts import hostile_network_injection_gate as gate  # noqa: E402


FIXTURE_PATH: Final = "fixtures/strict-local-hostile-surfaces/v1/mutations.json"
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-10/story-10.1/hostile-network-injection.json"
)
SOURCE_PATHS: Final = (
    "Cargo.lock",
    FIXTURE_PATH,
    "package-lock.json",
    "package.json",
    "scripts/hostile_network_injection_evidence.py",
    "scripts/hostile_network_injection_gate.py",
    "scripts/strict_local_source_audit.py",
    "security/strict-local-source-policy.json",
    "shells/vscode/package.json",
    "tests/test_hostile_network_injection_evidence.py",
    "tests/test_hostile_network_injection_gate.py",
    "tests/test_strict_local_source_audit.py",
)
COMMAND_SPECS: Final = (
    (
        ("python3", "scripts/hostile_network_injection_gate.py"),
        "Hostile network injection gate blocked all 8 cases before execution",
    ),
    (
        (
            "python3",
            "-m",
            "unittest",
            "tests.test_hostile_network_injection_gate",
            "tests.test_strict_local_source_audit",
        ),
        "Ran 15 tests",
    ),
    (
        (
            "python3",
            "-m",
            "unittest",
            "tests.test_hostile_network_injection_evidence",
        ),
        "Ran 5 tests",
    ),
    (
        ("npm", "run", "product:lint"),
        "Structural effect mediation boundary validated.",
    ),
)
MUTATION_PROFILE: Final = {
    "schema_version": 1,
    "case_count": 8,
    "families": [
        "telemetry-upload",
        "crash-upload",
        "remote-font-or-asset",
        "marketplace-contact",
        "update-download",
        "ambient-proxy",
        "hostile-loopback-client",
        "hostile-loopback-runtime-dependency",
    ],
    "evaluation": "in-memory-production-source-and-package-audit",
    "disposition": "blocked-before-execution",
}
CLAIMS: Final = {
    "all_declared_hostile_surface_families_blocked": True,
    "production_source_and_package_policy_exercised": True,
    "build_or_test_gate_failed_before_injected_execution": True,
    "hostile_dependency_installed": False,
    "injected_code_executed": False,
    "runtime_startup_behavior_proven": False,
    "packet_capture_performed": False,
    "external_network_used": False,
    "inference_performed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The corpus is applied in memory to the production source and package policy; no injected implementation is compiled, started, installed, imported, or permitted to contact a destination.",
    "This proves deterministic pre-execution rejection for the eight declared mutation families, not the semantic behavior of every possible third-party package or obfuscated implementation.",
    "Runtime syscall, DNS, socket, process, and packet observation remain assigned to the dependency-blocked 60-minute product workflow in Sub-task 10.1.3.2.",
    "No model inference, physical-network execution, cross-platform certification, or release acceptance is claimed.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class HostileInjectionEvidenceError(ValueError):
    """Raised when hostile-injection evidence is invalid or overstated."""


def git_revision(candidate: str) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        timeout=30,
        check=False,
    )
    revision = completed.stdout.strip()
    if completed.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise HostileInjectionEvidenceError("source revision is unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
        check=False,
    )
    if completed.returncode != 0 or not completed.stdout:
        raise HostileInjectionEvidenceError("committed source is unavailable")
    return completed.stdout


def validate_committed_fixture(revision: str) -> str:
    data = git_bytes(revision, FIXTURE_PATH)
    try:
        fixture = json.loads(data)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise HostileInjectionEvidenceError("committed fixture is invalid") from error
    if errors := gate.validate_fixture(fixture):
        raise HostileInjectionEvidenceError("; ".join(errors))
    return hashlib.sha256(data).hexdigest()


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {
            "path": path,
            "bytes": len(data := git_bytes(revision, path)),
            "sha256": hashlib.sha256(data).hexdigest(),
        }
        for path in SOURCE_PATHS
    ]


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "exit_code": 0,
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    completed = subprocess.run(
        list(arguments),
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if completed.returncode != 0 or marker not in completed.stdout + completed.stderr:
        raise HostileInjectionEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    result = gate.run_gate()
    if errors := gate.validate_result(result):
        raise HostileInjectionEvidenceError("; ".join(errors))
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-hostile-network-injection",
        "source_revision": revision,
        "task_ids": ["10.1.3.3", "S-010-ST02"],
        "status": "pass-hostile-surfaces-blocked-before-execution",
        "fixture_sha256": validate_committed_fixture(revision),
        "mutation_profile": MUTATION_PROFILE,
        "gate_result": result,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "sources": source_records(revision),
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    exact = {
        "schema_version": 1,
        "artifact_id": "strict-local-hostile-network-injection",
        "task_ids": ["10.1.3.3", "S-010-ST02"],
        "status": "pass-hostile-surfaces-blocked-before-execution",
        "mutation_profile": MUTATION_PROFILE,
        "gate_result": gate.expected_result(),
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            errors.append(f"hostile injection {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        errors.append("hostile injection source revision is invalid")
    if SHA256.fullmatch(str(report.get("fixture_sha256", ""))) is None:
        errors.append("hostile injection fixture identity is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        errors.append("hostile injection source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        errors.append("hostile injection source records are invalid")
    else:
        fixture_record = sources[SOURCE_PATHS.index(FIXTURE_PATH)]
        if fixture_record["sha256"] != report.get("fixture_sha256"):
            errors.append("hostile injection fixture identity changed")
    return errors


def validate_committed_sources(report: dict[str, Any]) -> None:
    revision = str(report["source_revision"])
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(revision, item["path"])).hexdigest() != item["sha256"]:
            raise HostileInjectionEvidenceError("committed source binding changed")
    if validate_committed_fixture(revision) != report["fixture_sha256"]:
        raise HostileInjectionEvidenceError("committed fixture binding changed")


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise HostileInjectionEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise HostileInjectionEvidenceError("evidence report is invalid")
    return report


def write_atomic(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        report = build_report(git_revision(arguments.source_revision))
        if errors := validate_report(report):
            raise HostileInjectionEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise HostileInjectionEvidenceError("; ".join(errors))
    validate_committed_sources(report)
    print("Strict-local hostile network-injection evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

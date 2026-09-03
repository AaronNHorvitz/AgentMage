#!/usr/bin/env python3
"""Build and validate the source-bound Sprint 45 scaffold application artifact."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT: Final = ROOT / "artifacts/sprints/sprint-45/scaffold-application-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
RESULT: Final = re.compile(
    rb"test result: ok\. 5 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out;"
)
SOURCES: Final = (
    "capabilities/repository-map/src/package_scaffold.rs",
    "capabilities/repository-map/src/lib.rs",
    "tests/test_sprint_45_scaffold_evidence.py",
    "scripts/sprint_45_scaffold_evidence.py",
)
COMMAND: Final = (
    "cargo", "test", "-p", "agentmage-capability-repository-map",
    "package_scaffold", "--lib", "--locked",
)


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True,
        check=False, timeout=30,
    )
    if result.returncode:
        raise ValueError(f"scaffold evidence source unavailable: {path}")
    return result.stdout


def expected(revision: str, exit_code: int, output: bytes) -> dict[str, Any]:
    passed = exit_code == 0 and RESULT.search(output) is not None
    return {
        "schema_version": 1,
        "record_type": "agentmage-sprint-45-scaffold-application-evidence",
        "source_revision": revision,
        "source_sha256": {
            path: hashlib.sha256(git_bytes(revision, path)).hexdigest() for path in SOURCES
        },
        "command": list(COMMAND),
        "command_exit_code": exit_code,
        "command_output_sha256": hashlib.sha256(output).hexdigest(),
        "approved_package_convention_count": 5 if passed else None,
        "application_manifest_count": 5 if passed else None,
        "mutation_case_count": 7 if passed else None,
        "ignored_test_count": 0 if passed else None,
        "atomic_root_publication_required": True if passed else None,
        "separate_grant_required": True if passed else None,
        "mutation_authority": False,
        "status": "PASS_LOCAL_SCAFFOLD_APPLICATION" if passed else "FAIL",
        "limitations": [
            "The application manifest is inert and grants no filesystem or command authority.",
            "No native filesystem execution, trusted-launch, or platform claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["scaffold evidence is not an object"]
    failures: list[str] = []
    revision = str(value.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        return ["source revision invalid"]
    if value.get("command") != list(COMMAND) or value.get("command_exit_code") != 0:
        failures.append("command result invalid")
    if not re.fullmatch(r"[0-9a-f]{64}", str(value.get("command_output_sha256", ""))):
        failures.append("command output digest invalid")
    expected_claims = {
        "approved_package_convention_count": 5,
        "application_manifest_count": 5,
        "mutation_case_count": 7,
        "ignored_test_count": 0,
        "atomic_root_publication_required": True,
        "separate_grant_required": True,
        "mutation_authority": False,
        "status": "PASS_LOCAL_SCAFFOLD_APPLICATION",
    }
    if any(value.get(key) != expected_value for key, expected_value in expected_claims.items()):
        failures.append("scaffold claim drift")
    source = value.get("source_sha256")
    if not isinstance(source, dict) or list(source) != list(SOURCES):
        failures.append("source inventory drift")
    else:
        for path in SOURCES:
            try:
                digest = hashlib.sha256(git_bytes(revision, path)).hexdigest()
            except ValueError as error:
                failures.append(str(error))
                continue
            if source.get(path) != digest:
                failures.append(f"source hash drift: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        revision = subprocess.run(
            ["git", "rev-parse", arguments.source_revision], cwd=ROOT, check=True,
            capture_output=True, text=True,
        ).stdout.strip()
        result = subprocess.run(COMMAND, cwd=ROOT, capture_output=True, check=False, timeout=300)
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(json.dumps(expected(revision, result.returncode, result.stdout + result.stderr), indent=2) + "\n")
    try:
        value = json.loads(REPORT.read_text())
    except (OSError, json.JSONDecodeError) as error:
        print(error, file=sys.stderr)
        return 1
    failures = validate(value)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 45 scaffold application evidence passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

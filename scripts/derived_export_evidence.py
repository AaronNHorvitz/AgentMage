#!/usr/bin/env python3
"""Build and validate one-way JSON Lines export evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/derived-json-lines.json"
SOURCE_PATHS: Final = (
    "kernel/engine/src/operational_store.rs",
    "docs/architecture/durable-authority-store.md",
    "scripts/derived_export_evidence.py",
    "tests/test_derived_export_evidence.py",
)
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "operational_store", "--locked"), "18 passed; 0 failed"),
    (("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
    (("python3", "-m", "unittest", "tests.test_derived_export_evidence"), "Ran 4 tests"),
)
CLAIMS: Final = {
    "versioned_non_executable_header": True,
    "raw_record_identifiers_omitted": True,
    "raw_record_bodies_omitted": True,
    "identity_and_retained_hashes_only": True,
    "deterministic_sorted_regeneration": True,
    "bounded_records_and_bytes": True,
    "atomic_create_without_overwrite": True,
    "occupied_and_ineligible_destinations_preserved": True,
    "deletion_or_tamper_changes_no_canonical_state": True,
    "json_lines_startup_authority_rejected": True,
    "json_lines_import_api_present": False,
    "dual_write_present": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The v0.1 derivative is a content-free audit/export view, not a full user-data portability archive.",
    "No JSON Lines import is implemented; any future import requires a separate explicit validated contract and cannot silently become startup authority.",
    "Export UI preview, user grant composition, encryption wrapping, crash campaign, canary campaign, cross-platform execution, packaging, and release acceptance remain later work.",
]
FRAGMENTS: Final = (
    "pub fn export_json_lines(",
    'content_mode: "identity-and-retained-hashes-only",',
    "startup_authority: false,",
    "const MAX_DERIVED_EXPORT_RECORDS: usize = 100_000;",
    "const MAX_DERIVED_EXPORT_BYTES: usize = 64 * 1024 * 1024;",
    "fs::hard_link(&temporary, destination)",
    "fn json_lines_export_is_deterministic_content_free_and_export_only()",
    "fn json_lines_export_rejects_occupied_or_ineligible_destinations_without_change()",
)
FORBIDDEN: Final = ("pub fn import_json_lines", "fn load_json_lines", "fn resume_from_json_lines")
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    """Raised when derived-export evidence is incomplete or overstated."""


def validate_source(value: str) -> list[str]:
    failures = [
        f"derived export source fragment changed: {index}"
        for index, fragment in enumerate(FRAGMENTS, 1)
        if value.count(fragment) != 1
    ]
    failures.extend(
        f"forbidden JSON Lines authority appeared: {fragment}"
        for fragment in FORBIDDEN
        if fragment in value
    )
    return failures


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=30, check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision is unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if result.returncode or not result.stdout:
        raise EvidenceError("committed source is unavailable")
    return result.stdout


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "exit_code": 0,
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    result = subprocess.run(
        list(arguments), cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        timeout=300, check=False, env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification command failed: {Path(arguments[0]).name}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        if path.endswith("operational_store.rs") and (failures := validate_source(data.decode())):
            raise EvidenceError("; ".join(failures))
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "derived-json-lines-export",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-one-way-derived-export",
        "task_ids": ["11.1.1.7"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures = []
    exact = {
        "artifact_id": "derived-json-lines-export",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-one-way-derived-export",
        "task_ids": ["11.1.1.7"],
        "verification_commands": expected_commands(),
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"derived export evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("derived export evidence revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("derived export evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int) or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources
    ):
        failures.append("derived export evidence source records are invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item["sha256"]:
            raise EvidenceError("committed source binding changed")


def write_atomic(report: dict[str, Any]) -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{REPORT_PATH.name}.", dir=REPORT_PATH.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, REPORT_PATH)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    if arguments.write:
        report = build_report(revision)
        write_atomic(report)
    else:
        report = json.loads(REPORT_PATH.read_text())
    failures = validate_report(report)
    if failures:
        raise EvidenceError("; ".join(failures))
    validate_committed(report)
    print("derived export evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

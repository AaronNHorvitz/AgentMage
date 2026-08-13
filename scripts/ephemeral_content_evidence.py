#!/usr/bin/env python3
"""Build and validate structural ephemeral-content evidence."""

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
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-11/story-11.1/ephemeral-content.json"
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/persistence.rs",
    "scripts/ephemeral_content_evidence.py",
    "tests/test_ephemeral_content_evidence.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "persistence::tests",
            "--locked",
        ),
        "12 passed; 0 failed",
    ),
    (
        ("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"),
        "Finished `dev` profile",
    ),
    (
        ("python3", "-m", "unittest", "tests.test_ephemeral_content_evidence"),
        "Ran 4 tests",
    ),
)
CONTENT_CLASSES: Final = [
    "raw_attachment",
    "full_tool_output",
    "environment_variable",
    "prompt",
    "model_response",
]
CLAIMS: Final = {
    "closed_raw_content_class": True,
    "public_constructor_forces_ephemeral": True,
    "forged_handling_refused": True,
    "raw_only_candidate_selects_no_storage": True,
    "mixed_candidate_retains_only_minimized_metadata": True,
    "raw_value_absent_from_prepared_record": True,
    "raw_value_digest_absent_from_prepared_record": True,
    "raw_value_and_digest_absent_from_receipt": True,
    "same_shape_content_substitution_changes_no_receipt": True,
    "typed_ingress_adapters_implemented": False,
    "database_page_canary_campaign_complete": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Attachment, tool, environment, prompt, and model ingress adapters remain later work; this artifact proves the kernel persistence contract they must use.",
    "The deterministic secret scanner retains its declared bounded signature coverage and makes no exhaustive-discovery claim.",
    "Database-page, WAL, temporary-file, backup, export, log, crash-output, and model-context canary inspection remains assigned to Sub-task 11.1.3.2.",
    "Expiration, holds, erasure, export, restore, corruption recovery, cross-platform evidence, and release acceptance remain later work.",
]
FRAGMENTS: Final = (
    "pub enum EphemeralContentClass {",
    "pub const fn ephemeral_content(",
    "ephemeral_class: Some(class),",
    "&& field.handling != PersistenceFieldHandling::Ephemeral",
    "fn every_raw_content_class_is_ephemeral_without_value_or_digest_receipt()",
    "fn raw_content_is_omitted_when_minimized_metadata_is_admitted()",
    "fn forged_raw_content_handling_fails_before_policy_decision()",
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    """Raised when ephemeral-content evidence is incomplete or overstated."""


def validate_source(value: str) -> list[str]:
    failures = [
        f"ephemeral source fragment changed: {index}"
        for index, fragment in enumerate(FRAGMENTS, 1)
        if value.count(fragment) != 1
    ]
    for variant in (
        "RawAttachment",
        "FullToolOutput",
        "EnvironmentVariable",
        "Prompt",
        "ModelResponse",
    ):
        if value.count(f"    {variant},") != 1:
            failures.append(f"ephemeral content variant changed: {variant}")
    return failures


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        timeout=30,
        check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision is unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
        check=False,
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
        list(arguments),
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification command failed: {Path(arguments[0]).name}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        if path.endswith("persistence.rs") and (failures := validate_source(data.decode())):
            raise EvidenceError("; ".join(failures))
        records.append(
            {"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()}
        )
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "structural-ephemeral-content",
        "claims": CLAIMS,
        "content_classes": CONTENT_CLASSES,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-structural-ephemeral-defaults",
        "task_ids": ["11.1.1.5"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures = []
    exact = {
        "artifact_id": "structural-ephemeral-content",
        "claims": CLAIMS,
        "content_classes": CONTENT_CLASSES,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-structural-ephemeral-defaults",
        "task_ids": ["11.1.1.5"],
        "verification_commands": expected_commands(),
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"ephemeral evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("ephemeral evidence revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("ephemeral evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("ephemeral evidence source records are invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item[
            "sha256"
        ]:
            raise EvidenceError("committed source binding changed")


def write_atomic(report: dict[str, Any]) -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(
        prefix=f".{REPORT_PATH.name}.", dir=REPORT_PATH.parent
    )
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
    if arguments.write:
        report = build_report(git_revision(arguments.source_revision))
        if failures := validate_report(report):
            raise EvidenceError("; ".join(failures))
        write_atomic(report)
    report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    if failures := validate_report(report):
        raise EvidenceError("; ".join(failures))
    validate_committed(report)
    print("Structural ephemeral-content evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

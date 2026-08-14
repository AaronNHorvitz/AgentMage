#!/usr/bin/env python3
"""Build and validate S-012-I06 session-environment evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/session-environment.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-012-i06-session-environment-results.md",
    "kernel/engine/src/session_environment.rs",
    "kernel/engine/src/configuration.rs",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/lib.rs",
    "scripts/session_environment_evidence.py",
    "tests/test_session_environment_evidence.py",
)
CASE_IDS: Final = tuple(f"SES-{index:02}" for index in range(1, 7))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for bounded session environment capture",
    "Focused cases closed: **6 of 6**.",
    "not ambient absolute paths",
    "Ambient operating-system, Git,",
    "manual fuzzing",
)
SOURCE_FRAGMENTS: Final = {
    "kernel/engine/src/session_environment.rs": (
        "pub struct SessionEnvironmentInput {",
        "pub struct SessionEnvironmentCapture {",
        "pub struct RepositoryObservation {",
        "pub struct AttachedFileProvenance {",
        "pub fn capture_session_environment(",
        "fn validate_workspaces(input: &SessionEnvironmentInput)",
        "fn validate_attachments(",
        "permission_profile_id: configuration.permission_profile_id()",
        "model_profile_id: configuration.model_profile_id()",
    ),
    "kernel/engine/src/configuration.rs": (
        "pub fn model_profile_id(&self) -> &str",
        "pub fn permission_profile_id(&self) -> &str",
    ),
    "kernel/engine/src/authority.rs": (
        "SessionRecord,",
        "impl_non_authoritative!(SessionRecord => crate::session_environment::SessionEnvironmentCapture);",
    ),
    "kernel/engine/src/lib.rs": ("pub mod session_environment;",),
}
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "session_environment::tests",
            "--locked",
        ),
        "6 passed; 0 failed",
    ),
    (
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-kernel-engine",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
        "Finished `dev` profile",
    ),
    (
        ("python3", "-m", "unittest", "tests.test_session_environment_evidence"),
        "Ran 4 tests",
    ),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 6,
    "captured_environment_family_count": 9,
    "maximum_workspace_roots": 16,
    "maximum_attachment_records": 32,
    "configuration_bound_profiles": True,
    "canonical_workspace_scopes_only": True,
    "session_authority_admitted_count": 0,
    "ambient_environment_discovery_implemented": False,
    "attachment_path_resolution_implemented": False,
    "attachment_content_parsing_implemented": False,
    "persistent_session_capture_implemented": False,
    "live_git_observations": 0,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The kernel validates supplied observations; ambient operating-system, Git, editor, and attachment discovery remain adapter and integration work.",
    "Attachment path resolution and content parsers remain assigned later work.",
    "The snapshot is in memory only; persistence, restart reconciliation, checkpoints, and UI rendering remain later work.",
    "Synthetic values exercise the protocol; no private user data, live repository, model, collector, external network, or file content is used.",
    "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(SES-\d{2})`", document)) != CASE_IDS:
        failures.append("S-012-I06 case closure changed")
    failures.extend(
        f"S-012-I06 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-012-I06 source fragment changed: {path}:{index}"
            for index, fragment in enumerate(fragments, 1)
            if values[path].count(fragment) != 1
        )
    return failures


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision unavailable")
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
        raise EvidenceError(f"committed source unavailable: {path}")
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
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification failed: {arguments[0]}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    values = {}
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append(
            {"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()}
        )
    if failures := validate_sources(values):
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "s-012-i06-session-environment",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-bounded-session-environment-capture",
        "task_ids": ["12.1.1.6", "S-012-I06"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-012-i06-session-environment",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-bounded-session-environment-capture",
        "task_ids": ["12.1.1.6", "S-012-I06"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"S-012-I06 evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-012-I06 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("S-012-I06 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("S-012-I06 evidence source records invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        digest = hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest()
        if digest != item["sha256"]:
            raise EvidenceError(f"committed source binding changed: {item['path']}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            revision = git_revision(args.source_revision)
            atomic_write(REPORT_PATH, canonical_json_bytes(build_report(revision)))
        report = json.loads(REPORT_PATH.read_text())
        if failures := validate_report(report):
            raise EvidenceError("; ".join(failures))
        validate_committed(report)
    except (EvidenceError, OSError, json.JSONDecodeError) as error:
        print(f"S-012-I06 session environment evidence: FAIL: {error}")
        return 1
    print("S-012-I06 session environment evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

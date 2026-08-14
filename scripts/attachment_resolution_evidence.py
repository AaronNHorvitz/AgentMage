#!/usr/bin/env python3
"""Build and validate S-012-I07 attachment-resolution evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-12/story-12.1/attachment-resolution.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-012-i07-attachment-resolution-results.md",
    "kernel/engine/src/attachment.rs",
    "kernel/engine/src/session_environment.rs",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/lib.rs",
    "scripts/attachment_resolution_evidence.py",
    "tests/test_attachment_resolution_evidence.py",
)
CASE_IDS: Final = tuple(f"ATT-{index:02}" for index in range(1, 7))
DOCUMENT_FRAGMENTS: Final = (
    "Pass for bounded attachment metadata and path resolution",
    "Focused cases closed: **6 of 6**.",
    "No UTF-8 decoder or structural parser runs here.",
    "does not claim live file",
    "manual fuzzing",
)
SOURCE_FRAGMENTS: Final = {
    "kernel/engine/src/attachment.rs": (
        "pub enum AttachmentFormat {",
        "pub enum AttachmentParserDisposition {",
        "pub struct AttachmentMetadata {",
        "pub struct ResolvedAttachment {",
        "pub const fn parser_disposition(format: AttachmentFormat)",
        "pub fn resolve_attachment<A: PlatformPathAdapter>(",
        "PathResolutionIntent::ContentHash,",
        "preimage.byte_len() != metadata.byte_len",
        "hex(preimage.content_sha256()) != metadata.content_sha256",
    ),
    "kernel/engine/src/session_environment.rs": ("pub struct AttachedFileProvenance {",),
    "kernel/engine/src/authority.rs": (
        "crate::attachment::AttachmentMetadata,",
        "crate::attachment::ResolvedAttachment,",
    ),
    "kernel/engine/src/lib.rs": ("pub mod attachment;",),
}
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "attachment::tests",
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
        ("python3", "-m", "unittest", "tests.test_attachment_resolution_evidence"),
        "Ran 4 tests",
    ),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "focused_case_count": 6,
    "attachment_format_count": 12,
    "raw_text_eligible_format_count": 4,
    "deferred_release_pack_format_count": 7,
    "unsupported_format_count": 1,
    "exact_preimage_required": True,
    "resolved_record_authority_admitted_count": 0,
    "parser_executions": 0,
    "file_content_bytes_retained": 0,
    "live_file_observations": 0,
    "format_detection_implemented": False,
    "persistent_resolution_records_implemented": False,
    "external_network_used": False,
    "manual_fuzzing_executed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The focused suite uses a deterministic fake adapter and synthetic preimages; no live file or platform execution is claimed.",
    "The boundary resolves and hashes metadata only; it retains no file bytes and runs no decoder, parser, scanner, or renderer.",
    "Shell discovery, format detection, content reading, parsers, persistence, checkpoints, and UI rendering remain later work.",
    "No private user data, model, external network, publication, or release path is used.",
    "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(values: dict[str, str]) -> list[str]:
    failures = []
    document = values[SOURCE_PATHS[0]]
    if tuple(re.findall(r"`(ATT-\d{2})`", document)) != CASE_IDS:
        failures.append("S-012-I07 case closure changed")
    failures.extend(
        f"S-012-I07 document fragment changed: {index}"
        for index, fragment in enumerate(DOCUMENT_FRAGMENTS, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in SOURCE_FRAGMENTS.items():
        failures.extend(
            f"S-012-I07 source fragment changed: {path}:{index}"
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
        "artifact_id": "s-012-i07-attachment-resolution",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-bounded-attachment-resolution",
        "task_ids": ["12.1.1.7", "S-012-I07"],
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-012-i07-attachment-resolution",
        "case_ids": list(CASE_IDS),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-bounded-attachment-resolution",
        "task_ids": ["12.1.1.7", "S-012-I07"],
        "verification_commands": expected_commands(),
    }
    failures = [
        f"S-012-I07 evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-012-I07 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("S-012-I07 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("S-012-I07 evidence source records invalid")
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
        print(f"S-012-I07 attachment resolution evidence: FAIL: {error}")
        return 1
    print("S-012-I07 attachment resolution evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

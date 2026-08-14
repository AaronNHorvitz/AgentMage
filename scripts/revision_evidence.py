#!/usr/bin/env python3
"""Declarative revision-bound evidence reports for focused task closures."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

try:
    from scripts.evidence_core import (
        EvidenceError,
        atomic_write,
        canonical_json_bytes,
        git_blob,
        git_source_identity,
        valid_sha256,
    )
except ModuleNotFoundError:
    from evidence_core import (
        EvidenceError,
        atomic_write,
        canonical_json_bytes,
        git_blob,
        git_source_identity,
        valid_sha256,
    )


@dataclass(frozen=True)
class RevisionEvidenceSpec:
    """Closed inputs and expected claims for one focused evidence report."""

    root: Path
    report_path: Path
    artifact_id: str
    case_pattern: str
    case_ids: tuple[str, ...]
    source_paths: tuple[str, ...]
    document_fragments: tuple[str, ...]
    source_fragments: dict[str, tuple[str, ...]]
    command_specs: tuple[tuple[tuple[str, ...], str], ...]
    claims: dict[str, Any]
    limitations: tuple[str, ...]
    status: str
    task_ids: tuple[str, ...]
    label: str


def validate_sources(spec: RevisionEvidenceSpec, values: dict[str, str]) -> list[str]:
    """Validate exact cases and stable fragments in source text."""

    failures = []
    document = values[spec.source_paths[0]]
    if tuple(re.findall(spec.case_pattern, document)) != spec.case_ids:
        failures.append(f"{spec.label} case closure changed")
    failures.extend(
        f"{spec.label} document fragment changed: {index}"
        for index, fragment in enumerate(spec.document_fragments, 1)
        if document.count(fragment) != 1
    )
    for path, fragments in spec.source_fragments.items():
        failures.extend(
            f"{spec.label} source fragment changed: {path}:{index}"
            for index, fragment in enumerate(fragments, 1)
            if values[path].count(fragment) != 1
        )
    return failures


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    """Return a content-free identity for one successful command contract."""

    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "exit_code": 0,
        "status": "pass",
    }


def expected_commands(spec: RevisionEvidenceSpec) -> list[dict[str, Any]]:
    """Return every expected command identity in declared order."""

    return [command_record(arguments, marker) for arguments, marker in spec.command_specs]


def run_checked(
    root: Path, arguments: tuple[str, ...], marker: str
) -> dict[str, Any]:
    """Run one bounded verification command and retain no raw output."""

    result = subprocess.run(
        list(arguments),
        cwd=root,
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification failed: {' '.join(arguments)}")
    return command_record(arguments, marker)


def source_records(spec: RevisionEvidenceSpec, revision: str) -> list[dict[str, Any]]:
    """Bind every declared source to its exact committed bytes."""

    values = {}
    records = []
    for path in spec.source_paths:
        data = git_blob(spec.root, revision, path)
        values[path] = data.decode("utf-8")
        records.append(
            {
                "bytes": len(data),
                "path": path,
                "sha256": hashlib.sha256(data).hexdigest(),
            }
        )
    if failures := validate_sources(spec, values):
        raise EvidenceError("; ".join(failures))
    return records


def build_report(spec: RevisionEvidenceSpec, revision: str) -> dict[str, Any]:
    """Build one report after executing every declared verification command."""

    return {
        "artifact_id": spec.artifact_id,
        "case_ids": list(spec.case_ids),
        "claims": spec.claims,
        "external_network_used": False,
        "limitations": list(spec.limitations),
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(spec, revision),
        "status": spec.status,
        "task_ids": list(spec.task_ids),
        "verification_commands": [
            run_checked(spec.root, arguments, marker)
            for arguments, marker in spec.command_specs
        ],
    }


def validate_report(spec: RevisionEvidenceSpec, report: dict[str, Any]) -> list[str]:
    """Validate exact report claims without consulting the working tree."""

    exact = {
        "artifact_id": spec.artifact_id,
        "case_ids": list(spec.case_ids),
        "claims": spec.claims,
        "external_network_used": False,
        "limitations": list(spec.limitations),
        "private_user_data_used": False,
        "schema_version": 1,
        "status": spec.status,
        "task_ids": list(spec.task_ids),
        "verification_commands": expected_commands(spec),
    }
    failures = [
        f"{spec.label} evidence {key} changed"
        for key, value in exact.items()
        if report.get(key) != value
    ]
    revision = report.get("source_revision")
    if not isinstance(revision, str) or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", revision):
        failures.append(f"{spec.label} evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        spec.source_paths
    ):
        failures.append(f"{spec.label} evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or not valid_sha256(item.get("sha256"))
        for item in sources
    ):
        failures.append(f"{spec.label} evidence source identity invalid")
    return failures


def validate_current(spec: RevisionEvidenceSpec, report: dict[str, Any]) -> list[str]:
    """Validate report shape and replay its committed source identities."""

    failures = validate_report(spec, report)
    revision = report.get("source_revision")
    if not isinstance(revision, str) or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", revision):
        return failures
    try:
        expected_sources = source_records(spec, revision)
    except EvidenceError as error:
        failures.append(str(error))
    else:
        if report.get("sources") != expected_sources:
            failures.append(f"{spec.label} evidence source bytes changed")
    return failures


def load_report(spec: RevisionEvidenceSpec) -> dict[str, Any]:
    """Load one report as a JSON object."""

    try:
        value = json.loads(spec.report_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"evidence report unavailable: {error}") from error
    if not isinstance(value, dict):
        raise EvidenceError("evidence report root invalid")
    return value


def run_cli(spec: RevisionEvidenceSpec) -> int:
    """Provide the standard write-or-validate command-line entry point."""

    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            identity = git_source_identity(spec.root, arguments.source_revision)
            report = build_report(spec, identity["revision"])
            atomic_write(spec.report_path, canonical_json_bytes(report))
        failures = validate_current(spec, load_report(spec))
    except EvidenceError as error:
        print(f"{spec.label} evidence: FAIL: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"{spec.label} evidence: FAIL: {failure}")
        return 1
    print(f"{spec.label} evidence: pass")
    return 0

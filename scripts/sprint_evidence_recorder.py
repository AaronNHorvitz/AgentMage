#!/usr/bin/env python3
"""Shared fail-closed recorder for locally executable sprint evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any


REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS = re.compile(rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;")
PROHIBITED_FIELDS = (
    "credential_value", "secret_value", "private_key", "raw_output", "raw_result",
    "prompt_text", "raw_document", "raw_workbook", "remote_url", "repository_path",
    "access_token",
)


@dataclass(frozen=True)
class SprintEvidenceDefinition:
    """Immutable expected content for one local sprint evidence report."""

    sprint: int
    root: Path
    output: str
    source_paths: tuple[str, ...]
    commands: tuple[tuple[str, tuple[str, ...]], ...]
    focused_commands: tuple[str, ...]
    rust_focused_commands: frozenset[str]
    security_requirement_ids: tuple[str, ...]
    implemented_contracts: dict[str, Any]
    verification_evidence: dict[str, Any]
    blockers: tuple[dict[str, str], ...]
    summary: dict[str, Any]

    @property
    def record_type(self) -> str:
        """Return the exact record type."""

        return f"sprint_{self.sprint}_local_evidence"


def digest(value: bytes) -> str:
    """Return one lowercase SHA-256 digest."""

    return hashlib.sha256(value).hexdigest()


def git_file(definition: SprintEvidenceDefinition, revision: str, path: str) -> bytes:
    """Read one exact committed source blob."""

    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=definition.root, check=False,
        stdin=subprocess.DEVNULL, capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint {definition.sprint} source is absent: {path}")
    return result.stdout


def _version(root: Path, executable: str, *arguments: str) -> str:
    resolved = shutil.which(executable)
    if resolved is None:
        return "unavailable"
    result = subprocess.run(
        (resolved, *arguments), cwd=root, check=False, stdin=subprocess.DEVNULL,
        capture_output=True, text=True, timeout=30,
    )
    output = (result.stdout + result.stderr).strip().splitlines()
    return output[0][:256] if result.returncode == 0 and output else "unavailable"


def environment_manifest(definition: SprintEvidenceDefinition) -> dict[str, str]:
    """Return a content-minimized local environment identity."""

    root = definition.root
    return {
        "system": platform.system(), "release": platform.release(), "machine": platform.machine(),
        "python": platform.python_version(), "rustc": _version(root, "rustc", "--version"),
        "cargo": _version(root, "cargo", "--version"), "node": _version(root, "node", "--version"),
        "npm": _version(root, "npm", "--version"),
    }


def run_commands(definition: SprintEvidenceDefinition) -> list[dict[str, Any]]:
    """Run every exact local command and retain only digests and outcome metadata."""

    records = []
    for identifier, argv in definition.commands:
        executable = shutil.which(argv[0])
        if executable is None:
            code, output = 127, b"executable-unavailable"
        else:
            result = subprocess.run(
                (executable, *argv[1:]), cwd=definition.root, check=False,
                stdin=subprocess.DEVNULL, capture_output=True, timeout=1800,
            )
            code, output = result.returncode, result.stdout + result.stderr
        skip_count = None
        if identifier in definition.rust_focused_commands:
            matches = IGNORED_TESTS.findall(output)
            skip_count = sum(int(value) for value in matches) if matches else -1
        elif identifier in definition.focused_commands:
            skip_count = 0 if code == 0 else None
        records.append({
            "id": identifier, "argv": list(argv), "exit_code": code,
            "output_sha256": digest(output), "blocking_skip_count": skip_count,
        })
    return records


def build_report(
    definition: SprintEvidenceDefinition,
    revision: str,
    commands: list[dict[str, Any]],
    environment: dict[str, str] | None = None,
) -> dict[str, Any]:
    """Build one source-bound local report without changing completion truth."""

    return {
        "schema_version": 1, "record_type": definition.record_type,
        "source_revision": revision,
        "source_sha256": {
            path: digest(git_file(definition, revision, path))
            for path in definition.source_paths
        },
        "environment": environment if environment is not None else environment_manifest(definition),
        "commands": commands,
        "security_requirement_ids": list(definition.security_requirement_ids),
        "implemented_contracts": dict(definition.implemented_contracts),
        "verification_evidence": dict(definition.verification_evidence),
        "blockers": [dict(item) for item in definition.blockers],
        "summary": dict(definition.summary),
    }


def validate_report(
    definition: SprintEvidenceDefinition,
    report: dict[str, Any],
    *,
    verify_ancestry: bool = True,
) -> list[str]:
    """Recompute report inventory, command, source, blocker, and completion truth."""

    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision): failures.append("source revision invalid")
    if report.get("schema_version") != 1 or report.get("record_type") != definition.record_type:
        failures.append("report identity invalid")
    expected_fields = (
        (report.get("security_requirement_ids"), list(definition.security_requirement_ids), "security mapping drift"),
        (report.get("implemented_contracts"), definition.implemented_contracts, "implemented contract drift"),
        (report.get("verification_evidence"), definition.verification_evidence, "verification claim drift"),
        (report.get("blockers"), list(definition.blockers), "blocker drift"),
        (report.get("summary"), definition.summary, "summary or release claim drift"),
    )
    for actual, expected, name in expected_fields:
        if actual != expected: failures.append(name)
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in definition.commands]:
        failures.append("command inventory drift")
    if len(commands) != len(definition.commands) or any(
        item.get("argv") != list(expected[1])
        or item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item, expected in zip(commands, definition.commands, strict=False)
    ):
        failures.append("command result invalid")
    for identifier in definition.focused_commands:
        focused = next((item for item in commands if item.get("id") == identifier), None)
        if focused is None or focused.get("blocking_skip_count") != 0:
            failures.append(f"focused skipped, suppressed, or unavailable check: {identifier}")
    source = report.get("source_sha256", {})
    if list(source) != list(definition.source_paths):
        failures.append("source inventory drift")
    elif REVISION.fullmatch(revision):
        for path in definition.source_paths:
            try:
                expected = digest(git_file(definition, revision, path))
            except ValueError as error:
                failures.append(str(error)); continue
            if source.get(path) != expected: failures.append(f"source digest drift: {path}")
    if verify_ancestry and REVISION.fullmatch(revision):
        result = subprocess.run(
            ["git", "merge-base", "--is-ancestor", revision, "HEAD"], cwd=definition.root,
            check=False, stdin=subprocess.DEVNULL, capture_output=True, timeout=30,
        )
        if result.returncode != 0: failures.append("report source revision is not an ancestor of current HEAD")
    encoded = json.dumps(report, sort_keys=True).lower()
    for prohibited in PROHIBITED_FIELDS:
        if prohibited in encoded: failures.append(f"prohibited evidence field: {prohibited}")
    return failures


def main(definition: SprintEvidenceDefinition) -> int:
    """Write or validate one report according to command-line arguments."""

    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision")
    arguments = parser.parse_args()
    output = definition.root / definition.output
    if arguments.write:
        revision = arguments.source_revision or subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=definition.root, check=True,
            stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30,
        ).stdout.strip()
        report = build_report(definition, revision, run_commands(definition))
        failures = validate_report(definition, report, verify_ancestry=False)
        if failures:
            for failure in failures: print(f"error: {failure}")
            return 1
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {output.relative_to(definition.root)}")
        return 0
    if not output.is_file():
        print(f"error: missing {output.relative_to(definition.root)}"); return 1
    failures = validate_report(definition, json.loads(output.read_text()))
    if failures:
        for failure in failures: print(f"error: {failure}")
        return 1
    print(f"Sprint {definition.sprint} local evidence validated")
    return 0

#!/usr/bin/env python3
"""Build and verify the Fedora Linux path identity and race harness artifact."""

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
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-6/story-6.1/path-race-report.json"
TESTS = (
    ("held_rename_replacement", "descriptor_walk_holds_the_original_file_and_exact_preimage_across_replacement"),
    ("symlink_hardlink_kind_limit", "symlink_hard_link_special_kind_and_resource_limit_fail_closed"),
    ("post_resolution_mutation", "affinity_precedes_observation_and_post_resolution_mutation_is_detected"),
    ("mount_identity_and_strategy", "automatic_strategy_matches_strict_openat2_probe_and_mount_ids_cannot_drift"),
    ("concurrent_toctou", "concurrent_symlink_replacement_never_changes_held_file_authority"),
)
SOURCE_PATHS = (
    "kernel/contracts/src/platform_path.rs",
    "platforms/linux/src/lib.rs",
    "scripts/path_race_artifact.py",
    "tests/test_path_race_artifact.py",
)


class PathRaceArtifactError(ValueError):
    """Raised when Linux path-race evidence is stale, incomplete, or overclaimed."""


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-path-race-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def run_test(test_name: str, root: Path = ROOT) -> None:
    completed = subprocess.run(
        ["cargo", "test", "--locked", "--offline", "-p", "agentmage-platform-linux",
         test_name, "--", "--exact", "--test-threads=1"],
        cwd=root, check=False, stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=120,
    )
    if completed.returncode != 0:
        raise PathRaceArtifactError(f"Linux path race scenario failed: {test_name}")


def execute_harness(root: Path = ROOT) -> dict[str, Any]:
    traces = []
    for scenario_id, test_name in TESTS:
        run_test(f"tests::{test_name}", root)
        traces.append({
            "scenario_id": scenario_id,
            "test_name": test_name,
            "execution_status": "pass-fedora-local",
            "out_of_root_access_count": 0,
        })
    return {
        "executed_scenario_count": len(traces),
        "out_of_root_access_count": 0,
        "traces": traces,
    }


def git_revision(candidate: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise PathRaceArtifactError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    if completed.returncode != 0:
        raise PathRaceArtifactError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    execution = execute_harness(root)
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
    )
    if ancestor.returncode != 0:
        raise PathRaceArtifactError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        if committed != (root / relative).read_bytes():
            raise PathRaceArtifactError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "6.1.2.3",
        "related_verification_task": "6.1.3.2",
        "artifact_id": "linux-file-identity-race-harness",
        "status": "pass-fedora-unprivileged-scope",
        "reference_revision": reference_revision,
        "sources": sources,
        "coverage": {
            "attack_class_count": 8,
            "executed_scenario_count": execution["executed_scenario_count"],
            "out_of_root_access_count": execution["out_of_root_access_count"],
            "concurrent_minimum_resolution_attempts": 512,
        },
        "traces": execution["traces"],
        "attack_status": {
            "symlink": "pass-fedora-local",
            "hard_link": "pass-fedora-local",
            "rename": "pass-fedora-local",
            "replacement": "pass-fedora-local",
            "content_mutation": "pass-fedora-local",
            "concurrent_toctou": "pass-fedora-local",
            "mount_identity_drift": "pass-synthetic-unit",
            "privileged_mount_swap": "not-executed-requires-isolated-privilege",
            "macos_alias": "blocked-macos",
        },
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "limitations": [
            "privileged bind-mount replacement is not executed on this host",
            "macOS alias attacks are blocked with the macOS implementation",
            "Ubuntu execution is not performed",
        ],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["path race report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "6.1.2.3"
        or value.get("related_verification_task") != "6.1.3.2"
        or value.get("artifact_id") != "linux-file-identity-race-harness"
        or value.get("status") != "pass-fedora-unprivileged-scope"
        or re.fullmatch(r"[0-9a-f]{40}", str(value.get("reference_revision"))) is None
    ):
        failures.append("path race report identity changed")
    if value.get("coverage") != {
        "attack_class_count": 8,
        "executed_scenario_count": 5,
        "out_of_root_access_count": 0,
        "concurrent_minimum_resolution_attempts": 512,
    }:
        failures.append("path race coverage changed")
    traces = value.get("traces")
    if (
        not isinstance(traces, list)
        or len(traces) != 5
        or any(trace.get("execution_status") != "pass-fedora-local" for trace in traces)
        or any(trace.get("out_of_root_access_count") != 0 for trace in traces)
    ):
        failures.append("path race traces are incomplete")
    status = value.get("attack_status")
    if not isinstance(status, dict) or status.get("privileged_mount_swap") != "not-executed-requires-isolated-privilege" or status.get("macos_alias") != "blocked-macos":
        failures.append("path race blocker status changed")
    if value.get("macos_evidence_substituted") is not False or value.get("release_claim") != "none":
        failures.append("path race report made an unsupported claim")
    if not isinstance(value.get("limitations"), list) or len(value["limitations"]) != 3:
        failures.append("path race limitations are incomplete")
    return failures


def write_report(reference_revision: str, root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, pretty_json(build_report(reference_revision, root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise PathRaceArtifactError(f"cannot read path race report: {error}") from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision, root):
        failures.append("path race report is stale or malformed")
    if failures:
        raise PathRaceArtifactError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            write_report(git_revision(args.source_revision))
        check_report()
    except (OSError, UnicodeError, PathRaceArtifactError, subprocess.SubprocessError) as error:
        print(f"Path race artifact failed: {error}", file=sys.stderr)
        return 1
    print("Story 6.1 Linux path race harness validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

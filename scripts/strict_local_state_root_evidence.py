#!/usr/bin/env python3
"""Build and validate strict-local Linux state-root evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-10/story-10.1/state-root-policy.json"
SOURCE_PATHS: Final = (
    "docs/architecture/strict-local-boundary.md",
    "kernel/engine/src/strict_local.rs",
    "platforms/linux/src/configuration_store.rs",
    "platforms/linux/src/lifecycle.rs",
    "platforms/linux/src/platform.rs",
    "platforms/linux/src/strict_local.rs",
    "scripts/strict_local_state_root_evidence.py",
    "tests/test_strict_local_state_root_evidence.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "strict_local_state_root",
            "--locked",
        ),
        "test result: ok. 6 passed; 0 failed; 0 ignored",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "storage_policy_rejects_every_risky_or_unknown_observation",
            "--locked",
        ),
        "test result: ok. 1 passed; 0 failed; 0 ignored",
    ),
    (
        ("python3", "scripts/strict_local_source_audit.py"),
        "Strict-local source audit passed with zero undeclared network paths.",
    ),
)
POLICY_PROFILE: Final = {
    "admitted_filesystem_class": "known-local-only",
    "known_local_filesystem_count": 11,
    "known_remote_filesystem_count": 7,
    "rejected_filesystem_classes": ["remote", "fuse", "unknown"],
    "synchronization_providers": [
        "box",
        "dropbox",
        "google-drive",
        "icloud-drive",
        "mega",
        "nextcloud",
        "onedrive",
        "owncloud",
        "pcloud",
        "proton-drive",
        "syncthing",
        "tresorit",
    ],
    "root_sentinels": ["dropbox-cache", "explicit-cloud-sync", "syncthing-folder"],
    "path_controls": [
        "absolute-only",
        "descriptor-relative",
        "no-symbolic-links",
        "current-user-owner",
        "owner-only-mode",
        "held-object-revalidation",
        "mount-and-filesystem-identity",
    ],
    "protected_consumers": [
        "configuration-store",
        "operational-key-lifecycle",
        "authority-store",
    ],
    "refusal_classes": [
        "cloud-synchronized",
        "remote-filesystem",
        "fuse-filesystem",
        "unknown-filesystem",
    ],
}
LIMITATIONS: Final = [
    "Remote, FUSE, and unknown filesystem classes are covered by deterministic filesystem-magic and kernel-policy matrices; live remote-mount fixtures remain Task 10.1.2.4.",
    "Folder-name and sentinel detection is conservative and cannot identify every third-party synchronization client or a deliberately disguised synchronizer.",
    "This evidence is Linux-only and does not claim macOS, Windows, backup, restore, packet-capture, or release acceptance.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class StrictLocalStateRootEvidenceError(ValueError):
    """Raised when strict-local state-root evidence is incomplete or overstated."""


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
        raise StrictLocalStateRootEvidenceError("source revision is unavailable")
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
        raise StrictLocalStateRootEvidenceError("committed source is unavailable")
    return completed.stdout


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
    output = completed.stdout + completed.stderr
    if completed.returncode != 0 or marker not in output:
        raise StrictLocalStateRootEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-linux-state-root-policy",
        "source_revision": revision,
        "task_ids": ["10.1.1.4"],
        "status": "pass-linux-detection-and-pre-io-rejection",
        "policy_profile": POLICY_PROFILE,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "integration_results": {
            "provider_and_sentinel_roots_rejected": True,
            "sentinel_added_after_inspection_rejected": True,
            "configuration_read_started": False,
            "configuration_preimage_preserved": True,
            "authority_database_created": False,
            "kernel_rejected_every_risky_or_unknown_observation": True,
        },
        "claims": {
            "linux_state_root_policy_implemented": True,
            "configuration_and_authority_pre_io_rejection_tested": True,
            "live_remote_mount_tested": False,
            "all_sync_clients_detected": False,
            "release_support": False,
        },
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "sources": source_records(revision),
    }


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise StrictLocalStateRootEvidenceError("state-root evidence is unavailable") from error
    if not isinstance(value, dict):
        raise StrictLocalStateRootEvidenceError("state-root evidence is invalid")
    return value


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if (
        not isinstance(value, dict)
        or value.get("schema_version") != 1
        or value.get("artifact_id") != "strict-local-linux-state-root-policy"
        or value.get("task_ids") != ["10.1.1.4"]
        or value.get("status") != "pass-linux-detection-and-pre-io-rejection"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        return ["strict-local state-root evidence identity changed"]
    if value.get("policy_profile") != POLICY_PROFILE:
        failures.append("strict-local state-root policy profile changed")
    if value.get("verification_commands") != expected_commands():
        failures.append("strict-local state-root verification commands changed")
    if value.get("integration_results") != {
        "provider_and_sentinel_roots_rejected": True,
        "sentinel_added_after_inspection_rejected": True,
        "configuration_read_started": False,
        "configuration_preimage_preserved": True,
        "authority_database_created": False,
        "kernel_rejected_every_risky_or_unknown_observation": True,
    }:
        failures.append("strict-local state-root integration results changed")
    if value.get("claims") != {
        "linux_state_root_policy_implemented": True,
        "configuration_and_authority_pre_io_rejection_tested": True,
        "live_remote_mount_tested": False,
        "all_sync_clients_detected": False,
        "release_support": False,
    }:
        failures.append("strict-local state-root evidence overclaimed")
    if value.get("limitations") != LIMITATIONS:
        failures.append("strict-local state-root limitations changed")
    if (
        value.get("private_user_data_used") is not False
        or value.get("external_network_used") is not False
    ):
        failures.append("strict-local state-root evidence used prohibited data or network")
    sources = value.get("sources")
    if (
        not isinstance(sources, list)
        or [item.get("path") for item in sources] != list(SOURCE_PATHS)
        or any(
            not isinstance(item.get("bytes"), int)
            or item.get("bytes", 0) <= 0
            or SHA256.fullmatch(str(item.get("sha256"))) is None
            for item in sources
        )
    ):
        failures.append("strict-local state-root source evidence changed")
    return failures


def write_atomic(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-state-root-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            write_atomic(REPORT_PATH, build_report(git_revision(arguments.source_revision)))
        report = read_json(REPORT_PATH)
        failures = validate_report(report)
        if not failures and report["sources"] != source_records(report["source_revision"]):
            failures.append("strict-local state-root source evidence is stale")
        if failures:
            raise StrictLocalStateRootEvidenceError("; ".join(failures))
    except (
        StrictLocalStateRootEvidenceError,
        OSError,
        UnicodeError,
        ValueError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Strict-local state-root evidence failed: {error}", file=sys.stderr)
        return 1
    print("Strict-local Linux state-root evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

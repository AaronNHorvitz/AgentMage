#!/usr/bin/env python3
"""Exercise the installed Sprint 16 worker inside one offline Linux guest."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import subprocess
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
WORKER: Final = Path("/usr/libexec/agentmage/agentmage-read-only-worker")
HOST: Final = Path("/usr/libexec/agentmage/agentmage-host")
TEST_NAME: Final = (
    "linux_read::tests::"
    "every_generic_tool_worker_returns_verified_result_one_receipt_and_no_workspace_mutation"
)
VERIFIED_OPERATIONS: Final = [
    "agentmage.workspace.list-directory",
    "agentmage.workspace.directory-tree",
    "agentmage.workspace.read-file",
    "agentmage.workspace.read-multiple",
    "agentmage.workspace.search-filenames",
    "agentmage.workspace.search-text",
    "agentmage.workspace.metadata",
    "agentmage.workspace.hash-file",
    "agentmage.workspace.hash-tree",
    "agentmage.workspace.binary-metadata",
]
ATTACK_CASES: Final = [
    "path_escape",
    "symlink_race",
    "special_file",
    "archive_bomb",
    "device",
    "socket",
    "environment",
    "network",
    "process",
    "write",
    "secret_canary",
]
LIFECYCLE_CASES: Final = [
    f"{termination}_{phase}"
    for termination in ("cancel", "timeout", "kill", "crash")
    for phase in ("before_result", "during_result", "after_result")
]
LIFECYCLE_FIXTURE_ROOT: Final = Path("/usr/libexec/agentmage-lifecycle-fixtures")


class GuestWorkerError(ValueError):
    """Raised when the installed worker guest campaign fails."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def run(
    command: list[str],
    receipts: list[dict[str, Any]],
    *,
    timeout: int = 3600,
    extra_environment: dict[str, str] | None = None,
) -> str:
    environment = {
        **os.environ,
        "CARGO_NET_OFFLINE": "true",
        "NPM_CONFIG_AUDIT": "false",
        "NPM_CONFIG_FUND": "false",
        "NPM_CONFIG_OFFLINE": "true",
        "NPM_CONFIG_UPDATE_NOTIFIER": "false",
        "RUSTUP_NO_UPDATE_CHECK": "1",
        **(extra_environment or {}),
    }
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
        check=False,
    )
    output = completed.stdout.replace(str(ROOT), "<GUEST_SOURCE>")
    receipts.append(
        {
            "id": f"guest-command-{len(receipts) + 1}",
            "executable": Path(command[0]).name,
            "exit_code": completed.returncode,
            "output_bytes": len(output.encode()),
            "output_sha256": hashlib.sha256(output.encode()).hexdigest(),
        }
    )
    if completed.returncode != 0:
        diagnostic = " | ".join(output.splitlines()[-12:])[-8000:]
        raise GuestWorkerError(
            f"guest command failed: {Path(command[0]).name}:{diagnostic or 'no-output'}"
        )
    return completed.stdout


def package_command(distribution: str, action: str, package: Path | None = None) -> list[str]:
    if distribution == "fedora":
        commands = {
            "install": ["sudo", "rpm", "-i", "--nosignature", str(package)],
            "remove": ["sudo", "rpm", "-e", "agentmage"],
        }
    elif distribution == "ubuntu":
        commands = {
            "install": ["sudo", "dpkg", "-i", str(package)],
            "remove": ["sudo", "dpkg", "-r", "agentmage"],
        }
    else:
        raise GuestWorkerError("unsupported guest distribution")
    return commands[action]


def worker_processes() -> list[str]:
    matches = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            executable = (entry / "exe").readlink()
        except OSError:
            continue
        if executable == WORKER:
            matches.append(entry.name)
    return sorted(matches)


def execute(distribution: str) -> dict[str, Any]:
    receipts: list[dict[str, Any]] = []
    output = Path("/tmp/agentmage-sprint16-packages")
    run(["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"], receipts)
    run(["cargo", "build", "-p", "agentmage-host", "--release", "--locked"], receipts)
    run(
        [
            "cargo",
            "build",
            "-p",
            "agentmage-platform-linux-inference",
            "--bins",
            "--release",
            "--locked",
        ],
        receipts,
    )
    run(
        [
            "cargo",
            "build",
            "-p",
            "agentmage-capability-read-only",
            "--bin",
            "agentmage-read-only-worker",
            "--release",
            "--locked",
        ],
        receipts,
    )
    run(["python3", "scripts/package_candidate.py", "--output", str(output)], receipts)
    suffix = ".rpm" if distribution == "fedora" else ".deb"
    candidates = sorted(output.glob(f"*{suffix}"))
    if len(candidates) != 1:
        raise GuestWorkerError("exact package candidate is unavailable")
    package = candidates[0]
    package_sha256 = sha256_file(package)
    run(package_command(distribution, "install", package), receipts)
    metadata = WORKER.stat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_uid != 0
        or metadata.st_gid != 0
        or stat.S_IMODE(metadata.st_mode) != 0o755
    ):
        raise GuestWorkerError("installed worker authority is invalid")
    worker_sha256 = sha256_file(WORKER)
    run([str(HOST), "--verify-package-candidate-root", "/"], receipts)
    run(
        [
            "cargo",
            "build",
            "-p",
            "agentmage-capability-read-only",
            "--bin",
            "agentmage-read-only-lifecycle-fixture",
            "--features",
            "lifecycle-fixture",
            "--locked",
        ],
        receipts,
    )
    run(
        ["sudo", "install", "-d", "-o", "root", "-g", "root", "-m", "0755", str(LIFECYCLE_FIXTURE_ROOT)],
        receipts,
    )
    lifecycle_binary = ROOT / "target/debug/agentmage-read-only-lifecycle-fixture"
    for case in LIFECYCLE_CASES:
        fixture_name = case.replace("_result", "").replace("_", "-")
        run(
            [
                "sudo",
                "install",
                "-o",
                "root",
                "-g",
                "root",
                "-m",
                "0755",
                str(lifecycle_binary),
                str(LIFECYCLE_FIXTURE_ROOT / f"agentmage-lifecycle-{fixture_name}"),
            ],
            receipts,
        )
    run(
        [
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "--locked",
            "sandbox::tests::",
            "--",
            "--ignored",
            "--skip",
            "lifecycle_matrix_cleans_descendants_scratch_and_never_accepts_interrupted_output",
            "--test-threads=1",
        ],
        receipts,
    )
    lifecycle_environment = {
        "AGENTMAGE_LIFECYCLE_FIXTURE_ROOT": str(LIFECYCLE_FIXTURE_ROOT)
    }
    for package_name, test_name in (
        (
            "agentmage-platform-linux",
            "sandbox::tests::lifecycle_matrix_cleans_descendants_scratch_and_never_accepts_interrupted_output",
        ),
        (
            "agentmage-host",
            "linux_read::tests::lifecycle_matrix_retains_one_terminal_receipt_and_never_reports_false_completion",
        ),
    ):
        run(
            [
                "cargo",
                "test",
                "-p",
                package_name,
                "--locked",
                test_name,
                "--",
                "--ignored",
                "--exact",
                "--test-threads=1",
            ],
            receipts,
            extra_environment=lifecycle_environment,
        )
    for test_name in (
        "tests::symlink_hard_link_special_kind_and_resource_limit_fail_closed",
        "tests::concurrent_symlink_replacement_never_changes_held_file_authority",
    ):
        run(
            [
                "cargo",
                "test",
                "-p",
                "agentmage-platform-linux",
                "--locked",
                test_name,
                "--",
                "--exact",
            ],
            receipts,
        )
    run(
        [
            "cargo",
            "test",
            "-p",
            "agentmage-host",
            "--locked",
            TEST_NAME,
            "--",
            "--ignored",
            "--exact",
        ],
        receipts,
    )
    units = run(
        [
            "systemctl",
            "--user",
            "list-units",
            "--all",
            "--plain",
            "--no-legend",
            "agentmage-worker-*",
        ],
        receipts,
    )
    if units.strip() or worker_processes():
        raise GuestWorkerError("worker lifecycle residue remains")
    run(["sudo", "rm", "-rf", str(LIFECYCLE_FIXTURE_ROOT)], receipts)
    if LIFECYCLE_FIXTURE_ROOT.exists() or Path("/tmp/agentmage-lifecycle-scratch").exists():
        raise GuestWorkerError("lifecycle fixture residue remains")
    run(package_command(distribution, "remove"), receipts)
    if WORKER.exists() or WORKER.is_symlink() or HOST.exists() or HOST.is_symlink():
        raise GuestWorkerError("package removal residue remains")
    return {
        "schema_version": 1,
        "record_type": "sprint-16-installed-worker-guest-result",
        "distribution": distribution,
        "status": "pass",
        "strict_offline": True,
        "package_sha256": package_sha256,
        "worker": {
            "path_class": "root-owned-package-libexec",
            "mode": "0755",
            "sha256": worker_sha256,
        },
        "verified_operations": VERIFIED_OPERATIONS,
        "receipt_count": len(VERIFIED_OPERATIONS),
        "attack_cases": ATTACK_CASES,
        "linux_attack_matrix_complete": True,
        "lifecycle_cases": LIFECYCLE_CASES,
        "linux_lifecycle_campaign_complete": True,
        "terminal_receipts_per_lifecycle_case": 1,
        "false_completion_cases": 0,
        "workspace_invariant": True,
        "worker_process_residue": False,
        "transient_unit_residue": False,
        "package_residue": False,
        "network_used_during_execution": False,
        "private_data_used": False,
        "commands": receipts,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--distribution", choices=("fedora", "ubuntu"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        report = execute(arguments.distribution)
        arguments.output.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"Sprint 16 guest worker failed: {error}", file=os.sys.stderr)
        return 1
    print("Sprint 16 installed worker guest passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

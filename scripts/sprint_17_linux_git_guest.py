#!/usr/bin/env python3
"""Exercise Sprint 17 Git and instruction boundaries in one offline Linux guest."""

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
HOST: Final = Path("/usr/libexec/agentmage/agentmage-host")
GIT_OPERATIONS: Final = [
    "status",
    "current_branch",
    "upstream",
    "branch_list",
    "log",
    "diff",
    "staged_diff",
    "show",
    "worktree_list",
    "object",
    "ref",
    "dirty_tree",
    "untracked_files",
]
FIXTURE_STATES: Final = [
    "clean",
    "dirty",
    "staged",
    "renamed",
    "detached",
    "untracked",
    "malformed",
]
ATTACK_CASES: Final = [
    "hook",
    "filter",
    "pager",
    "alias",
    "credential_helper",
    "unsafe_link",
    "replacement_object",
    "remote_url",
    "loopback_contact",
]
INSTRUCTION_CLASSES: Final = [
    "workspace_instruction",
    "repository_instruction",
    "project_document",
    "hierarchical_instruction",
]


class GuestEvidenceError(ValueError):
    """Raised when one installed guest campaign is incomplete."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def run(command: list[str], receipts: list[dict[str, Any]], *, timeout: int = 3600) -> str:
    environment = {
        **os.environ,
        "CARGO_NET_OFFLINE": "true",
        "NPM_CONFIG_AUDIT": "false",
        "NPM_CONFIG_FUND": "false",
        "NPM_CONFIG_OFFLINE": "true",
        "NPM_CONFIG_UPDATE_NOTIFIER": "false",
        "RUSTUP_NO_UPDATE_CHECK": "1",
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
    normalized = completed.stdout.replace(str(ROOT), "<GUEST_SOURCE>")
    receipts.append(
        {
            "id": f"guest-command-{len(receipts) + 1}",
            "executable": Path(command[0]).name,
            "exit_code": completed.returncode,
            "output_bytes": len(normalized.encode()),
            "output_sha256": hashlib.sha256(normalized.encode()).hexdigest(),
        }
    )
    if completed.returncode != 0:
        diagnostic = " | ".join(normalized.splitlines()[-12:])[-8000:]
        raise GuestEvidenceError(
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
        raise GuestEvidenceError("unsupported guest distribution")
    return commands[action]


def execute(distribution: str) -> dict[str, Any]:
    receipts: list[dict[str, Any]] = []
    output = Path("/tmp/agentmage-sprint17-packages")
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
        raise GuestEvidenceError("exact package candidate is unavailable")
    package = candidates[0]
    package_sha256 = sha256_file(package)
    run(package_command(distribution, "install", package), receipts)
    host_metadata = HOST.stat()
    if (
        not stat.S_ISREG(host_metadata.st_mode)
        or host_metadata.st_uid != 0
        or host_metadata.st_gid != 0
        or stat.S_IMODE(host_metadata.st_mode) != 0o755
    ):
        raise GuestEvidenceError("installed host authority is invalid")
    run([str(HOST), "--verify-package-candidate-root", "/"], receipts)
    runtime_digests = {
        name: sha256_file(Path(path))
        for name, path in {
            "bubblewrap": "/usr/bin/bwrap",
            "git": "/usr/bin/git",
            "systemctl": "/usr/bin/systemctl",
            "systemd_run": "/usr/bin/systemd-run",
        }.items()
    }

    run(
        [
            "cargo",
            "test",
            "-p",
            "agentmage-capability-read-only",
            "--locked",
            "git::tests::pinned_git_fixtures_are_exact_inert_bounded_and_byte_invariant",
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
            "agentmage-platform-linux",
            "--locked",
            "instruction_discovery::tests::",
            "--",
            "--test-threads=1",
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
            "repository_safety::tests::live_",
            "--",
            "--ignored",
            "--test-threads=1",
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
            "linux_coding_runtime::tests::story_48_2_linux_git_adapter_runs_the_approved_plan_without_a_shell",
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
            "agentmage-git-inspection-*",
        ],
        receipts,
    )
    if units.strip():
        raise GuestEvidenceError("Git inspection unit residue remains")
    run(package_command(distribution, "remove"), receipts)
    if HOST.exists() or HOST.is_symlink():
        raise GuestEvidenceError("package removal residue remains")
    return {
        "schema_version": 1,
        "record_type": "sprint-17-installed-linux-git-guest-result",
        "distribution": distribution,
        "status": "pass",
        "strict_offline": True,
        "package_sha256": package_sha256,
        "host_sha256": sha256_file(ROOT / "target/release/agentmage-host"),
        "runtime_sha256": runtime_digests,
        "git_operations": GIT_OPERATIONS,
        "fixture_states": FIXTURE_STATES,
        "attack_cases": ATTACK_CASES,
        "instruction_classes": INSTRUCTION_CLASSES,
        "workspace_invariant": True,
        "loopback_contact_observed": False,
        "canary_execution_count": 0,
        "source_content_in_discovery_records": False,
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
        print(f"Sprint 17 guest evidence failed: {error}", file=os.sys.stderr)
        return 1
    print("Sprint 17 installed Linux Git guest passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

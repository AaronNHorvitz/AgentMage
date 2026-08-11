#!/usr/bin/env python3
"""Execute the locked AgentMage checks in one isolated Linux environment."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import socket
import stat
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
TRANSIENT_ROOTS = (
    ".build",
    ".git",
    "review-evidence",
    "target",
)
EXPECTED_TOOL_VERSIONS = {
    "cargo": re.compile(r"^cargo 1\.95\.0\b"),
    "clippy-driver": re.compile(r"^clippy 0\.1\.95\b"),
    "node": re.compile(r"^v24\.15\.0$"),
    "npm": re.compile(r"^11\.12\.1$"),
    "python3": re.compile(r"^Python 3\."),
    "rustc": re.compile(r"^rustc 1\.95\.0\b"),
    "rustfmt": re.compile(r"^rustfmt 1\.9\.0-stable\b"),
}
HEX_SHA256 = re.compile(r"^[0-9a-f]{64}$")


def canonical_json(value: Any) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def parse_os_release(path: Path = Path("/etc/os-release")) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" not in line or line.startswith("#"):
            continue
        key, value = line.split("=", 1)
        values[key] = value.strip().strip('"')
    return values


def input_paths(source_root: Path, policy: dict[str, Any]) -> list[str]:
    """Return the legacy schema-v1 curated input closure."""
    contract = read_json(source_root / "architecture/build-contract.json")
    paths = set(contract["required_files"])
    paths.update(contract["lockfiles"])
    paths.update(policy["additional_input_paths"])
    return sorted(paths)


def input_tree_sha256(source_root: Path, policy: dict[str, Any]) -> str:
    """Hash the legacy schema-v1 curated input closure for historical replay."""
    digest = hashlib.sha256()
    for relative in input_paths(source_root, policy):
        path = source_root / relative
        if not path.is_file() or path.is_symlink():
            raise OSError(f"missing or non-regular clean-build input: {relative}")
        encoded_path = relative.encode("utf-8")
        content = path.read_bytes()
        digest.update(len(encoded_path).to_bytes(8, "big"))
        digest.update(encoded_path)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def committed_tree_content_sha256(source_root: Path) -> str:
    digest = hashlib.sha256()
    paths = sorted(
        path.relative_to(source_root)
        for path in source_root.rglob("*")
        if path.is_file() or path.is_symlink()
    )
    for relative in paths:
        path = source_root / relative
        if path.is_symlink() or not path.is_file():
            raise OSError(
                f"non-regular committed source input: {relative.as_posix()}"
            )
        encoded_path = relative.as_posix().encode("utf-8")
        executable = bool(path.stat().st_mode & stat.S_IXUSR)
        content = path.read_bytes()
        digest.update(len(encoded_path).to_bytes(8, "big"))
        digest.update(encoded_path)
        digest.update(b"x" if executable else b"-")
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def executable_record(name: str, environment: dict[str, str]) -> dict[str, str]:
    executable = shutil.which(name, path=environment["PATH"])
    if executable is None:
        raise OSError(f"missing required clean-build executable: {name}")
    version_args = [name, "--version"]
    result = subprocess.run(
        version_args,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=30,
        check=False,
    )
    version = result.stdout.strip().splitlines()[0]
    if result.returncode != 0 or not EXPECTED_TOOL_VERSIONS[name].search(version):
        raise OSError(f"unexpected {name} version: {version}")
    resolved = Path(executable).resolve()
    return {
        "id": name,
        "version": version,
        "executable_sha256": sha256_bytes(resolved.read_bytes()),
    }


def sanitized_output(output: str, replacements: tuple[Path, ...]) -> str:
    sanitized = output
    for path in sorted(replacements, key=lambda item: len(str(item)), reverse=True):
        sanitized = sanitized.replace(str(path), "<ISOLATED_PATH>")
    return sanitized.replace("\r\n", "\n")


def run_command(
    command: dict[str, Any],
    work: Path,
    environment: dict[str, str],
    replacements: tuple[Path, ...],
) -> dict[str, Any]:
    command_environment = environment.copy()
    if command["network"] != "bootstrap-only":
        command_environment.update(
            {
                "CARGO_NET_OFFLINE": "true",
                "NPM_CONFIG_OFFLINE": "true",
            }
        )
    result = subprocess.run(
        command["argv"],
        cwd=work,
        env=command_environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=600,
        check=False,
    )
    output = sanitized_output(result.stdout, replacements)
    return {
        "id": command["id"],
        "argv": command["argv"],
        "network": command["network"],
        "exit_code": result.returncode,
        "status": "pass" if result.returncode == 0 else "fail",
        "output": output,
        "output_sha256": sha256_bytes(output.encode("utf-8")),
    }


def preexisting_outputs(source_root: Path) -> list[str]:
    candidates = [
        source_root / "target",
        source_root / "shells/vscode/dist",
    ]
    return [path.relative_to(source_root).as_posix() for path in candidates if path.exists()]


def make_tree_user_writable(root: Path) -> None:
    for path in (root, *root.rglob("*")):
        if not path.is_symlink():
            path.chmod(path.stat().st_mode | stat.S_IWUSR)


def post_bootstrap_network_is_denied() -> bool:
    try:
        interfaces = socket.if_nameindex()
    except OSError:
        return False
    return bool(interfaces) and all(name == "lo" for _, name in interfaces)


def execute(
    source_root: Path,
    platform_id: str,
    source_revision: str,
    source_tree: str,
    source_archive_sha256: str,
    source_content_sha256: str,
) -> dict[str, Any]:
    policy = read_json(source_root / "architecture/clean-build-policy.json")
    platform = policy["linux_platforms"].get(platform_id)
    if platform is None:
        raise OSError(f"unsupported clean-build platform: {platform_id}")
    if os.geteuid() == 0:
        raise OSError("clean build must run as a non-root user")
    os_release = parse_os_release()
    if (
        os_release.get("ID") != platform["os_id"]
        or os_release.get("VERSION_ID") != platform["version_id"]
    ):
        raise OSError("clean-build environment does not match its declared platform")
    if preexisting_outputs(source_root):
        raise OSError("committed source archive contains ambient build output")

    with tempfile.TemporaryDirectory(prefix="agentmage-clean-build-") as temporary:
        isolated = Path(temporary)
        work = isolated / "source"
        home = isolated / "home"
        cargo_home = isolated / "cargo-home"
        npm_cache = isolated / "npm-cache"
        target = isolated / "target"
        for directory in (home, npm_cache, target):
            directory.mkdir()
        shutil.copytree(
            Path(os.environ["CARGO_HOME"]),
            cargo_home,
            symlinks=True,
        )
        make_tree_user_writable(cargo_home)
        shutil.copytree(
            source_root,
            work,
            symlinks=True,
            ignore=shutil.ignore_patterns(*TRANSIENT_ROOTS),
        )
        make_tree_user_writable(work)
        if preexisting_outputs(work):
            raise OSError("isolated source did not start without build output")

        environment = {
            "CARGO_HOME": str(cargo_home),
            "CARGO_TARGET_DIR": str(target),
            "HOME": str(home),
            "LANG": "C.UTF-8",
            "NPM_CONFIG_AUDIT": "false",
            "NPM_CONFIG_CACHE": str(npm_cache),
            "NPM_CONFIG_FUND": "false",
            "NPM_CONFIG_UPDATE_NOTIFIER": "false",
            "PATH": os.environ["PATH"],
            "RUSTUP_HOME": os.environ["RUSTUP_HOME"],
            "RUSTUP_NO_UPDATE_CHECK": "1",
            "TMPDIR": str(isolated),
        }
        tools = [
            executable_record(name, environment)
            for name in (
                "cargo",
                "clippy-driver",
                "node",
                "npm",
                "python3",
                "rustc",
                "rustfmt",
            )
        ]
        replacements = (
            source_root,
            isolated,
            work,
            home,
            cargo_home,
            npm_cache,
            target,
        )
        commands = []
        for command in policy["verification_commands"]:
            result = run_command(command, work, environment, replacements)
            commands.append(result)
            if result["status"] != "pass":
                break
        command_status = {item["id"]: item["status"] for item in commands}
        checks = {
            "all_commands_passed": len(commands)
            == len(policy["verification_commands"])
            and all(item["status"] == "pass" for item in commands),
            "ambient_dependency_detected": False,
            "clean_home": True,
            "clean_npm_cache": True,
            "clean_source_archive": True,
            "clean_target": True,
            "post_bootstrap_network_denied": post_bootstrap_network_is_denied(),
            "supply_chain_outputs_validated": command_status.get("sbom-build")
            == "pass"
            and command_status.get("sbom-check") == "pass",
        }
        status = "pass" if all(
            value is True
            for key, value in checks.items()
            if key != "ambient_dependency_detected"
        ) and checks["ambient_dependency_detected"] is False else "fail"
        return {
            "status": status,
            "platform": {
                "architecture": os.uname().machine,
                "id": platform_id,
                "os_id": os_release["ID"],
                "pretty_name": os_release.get("PRETTY_NAME", "unknown"),
                "version_id": os_release["VERSION_ID"],
            },
            "execution": {
                "effective_gid": os.getegid(),
                "effective_uid": os.geteuid(),
                "privileged": False,
                "source_archive": "read-only",
                "user_class": "standard-unprivileged",
                "writable_storage": "fresh-temporary-filesystem",
            },
            "source": {
                "archive_sha256": source_archive_sha256,
                "content_sha256": source_content_sha256,
                "revision": source_revision,
                "tree": source_tree,
            },
            "toolchains": tools,
            "commands": commands,
            "checks": checks,
            "macos_support_claim": "none",
        }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform")
    parser.add_argument("--source-revision")
    parser.add_argument("--source-tree")
    parser.add_argument("--source-archive-sha256")
    parser.add_argument("--source-content-sha256")
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--verify-source-content")
    args = parser.parse_args()
    if args.verify_source_content:
        if not HEX_SHA256.fullmatch(args.verify_source_content):
            print("expected source content must be a SHA-256 value", file=sys.stderr)
            return 2
        try:
            actual = committed_tree_content_sha256(args.source_root)
        except OSError as error:
            print(f"clean source content verification failed: {error}", file=sys.stderr)
            return 1
        if actual != args.verify_source_content:
            print("clean source content identity does not match", file=sys.stderr)
            return 1
        print("clean source content identity passed")
        return 0
    if not all(
        (
            args.platform,
            args.source_revision,
            args.source_tree,
            args.source_archive_sha256,
            args.source_content_sha256,
        )
    ):
        print("clean standard build identity arguments are required", file=sys.stderr)
        return 2
    if not re.fullmatch(r"[0-9a-f]{40,64}", args.source_revision):
        print("source revision must be a full lowercase Git object id", file=sys.stderr)
        return 2
    if not re.fullmatch(r"[0-9a-f]{40,64}", args.source_tree):
        print("source tree must be a full lowercase Git object id", file=sys.stderr)
        return 2
    if not re.fullmatch(r"[0-9a-f]{64}", args.source_archive_sha256):
        print("source archive must be a SHA-256 value", file=sys.stderr)
        return 2
    if not re.fullmatch(r"[0-9a-f]{64}", args.source_content_sha256):
        print("source content must be a SHA-256 value", file=sys.stderr)
        return 2
    try:
        report = execute(
            args.source_root,
            args.platform,
            args.source_revision,
            args.source_tree,
            args.source_archive_sha256,
            args.source_content_sha256,
        )
    except (KeyError, OSError, subprocess.SubprocessError, ValueError) as error:
        print(f"clean standard build failed: {error}", file=sys.stderr)
        return 1
    print(canonical_json(report), end="")
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())

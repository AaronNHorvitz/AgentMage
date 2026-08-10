#!/usr/bin/env python3
"""Execute the locked AgentMage checks in one isolated Linux environment."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SUPPLY_CHAIN_PATHS = (
    "supply-chain/dependency-hashes.sha256",
    "supply-chain/dependency-provenance.json",
    "supply-chain/sbom.cdx.json",
)
TRANSIENT_ROOTS = (
    ".build",
    ".git",
    "node_modules",
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
    contract = read_json(source_root / "architecture/build-contract.json")
    paths = set(contract["required_files"])
    paths.update(contract["lockfiles"])
    paths.update(policy["additional_input_paths"])
    return sorted(paths)


def input_tree_sha256(source_root: Path, policy: dict[str, Any]) -> str:
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
        source_root / "node_modules",
        source_root / "target",
        source_root / "shells/vscode/dist",
    ]
    return [path.relative_to(source_root).as_posix() for path in candidates if path.exists()]


def execute(
    source_root: Path,
    platform_id: str,
    source_revision: str,
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
        for directory in (home, cargo_home, npm_cache, target):
            directory.mkdir()
        shutil.copytree(
            source_root,
            work,
            symlinks=True,
            ignore=shutil.ignore_patterns(*TRANSIENT_ROOTS),
        )
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
        supply_before = {
            relative: sha256_bytes((work / relative).read_bytes())
            for relative in SUPPLY_CHAIN_PATHS
        }
        replacements = (source_root, isolated, work, home, cargo_home, npm_cache, target)
        commands = []
        for command in policy["commands"]:
            result = run_command(command, work, environment, replacements)
            commands.append(result)
            if result["status"] != "pass":
                break
        supply_after = {
            relative: sha256_bytes((work / relative).read_bytes())
            for relative in SUPPLY_CHAIN_PATHS
        }
        checks = {
            "all_commands_passed": len(commands) == len(policy["commands"])
            and all(item["status"] == "pass" for item in commands),
            "ambient_dependency_detected": False,
            "clean_home": True,
            "clean_npm_cache": True,
            "clean_source_archive": True,
            "clean_target": True,
            "supply_chain_unchanged": supply_before == supply_after,
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
                "input_sha256": input_tree_sha256(source_root, policy),
                "revision": source_revision,
            },
            "toolchains": tools,
            "commands": commands,
            "checks": checks,
            "macos_support_claim": "none",
        }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_revision):
        print("source revision must be a full lowercase Git object id", file=sys.stderr)
        return 2
    try:
        report = execute(args.source_root, args.platform, args.source_revision)
    except (KeyError, OSError, subprocess.SubprocessError, ValueError) as error:
        print(f"clean standard build failed: {error}", file=sys.stderr)
        return 1
    print(canonical_json(report), end="")
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())

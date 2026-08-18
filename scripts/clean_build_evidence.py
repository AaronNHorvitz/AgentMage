#!/usr/bin/env python3
"""Run and validate source-bound clean Linux build evidence."""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import io
import json
import re
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any

try:
    from scripts.clean_standard_build import (
        committed_tree_content_sha256,
        input_tree_sha256,
    )
except ModuleNotFoundError:
    from clean_standard_build import committed_tree_content_sha256, input_tree_sha256


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "architecture" / "clean-build-policy.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-1"
    / "story-1.1"
    / "clean-build-report.json"
)
EXPECTED_PLATFORMS = ("fedora-x86_64", "ubuntu-x86_64")
EXPECTED_BOOTSTRAP_COMMANDS = ("npm-clean-install", "cargo-fetch")
EXPECTED_VERIFICATION_COMMANDS = (
    "npm-locked-tree",
    "cargo-locked-tree",
    "build",
    "lint",
    "format-check",
    "test",
    "sbom-build",
    "sbom-check",
    "diagnostic",
)
EXPECTED_COMMANDS = EXPECTED_BOOTSTRAP_COMMANDS + EXPECTED_VERIFICATION_COMMANDS
EXPECTED_BOOTSTRAP_ARGV = (
    ("npm-clean-install", ["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"]),
    ("cargo-fetch", ["cargo", "fetch", "--locked"]),
)
EXPECTED_VERIFICATION_ARGV = (
    ("npm-locked-tree", ["npm", "ls", "--all", "--json"]),
    (
        "cargo-locked-tree",
        [
            "cargo",
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
            "--no-deps",
        ],
    ),
    ("build", ["npm", "run", "product:build"]),
    ("lint", ["npm", "run", "product:lint"]),
    ("format-check", ["npm", "run", "product:format-check"]),
    ("test", ["npm", "run", "product:test"]),
    ("sbom-build", ["npm", "run", "supply-chain:build"]),
    ("sbom-check", ["npm", "run", "supply-chain:check"]),
    ("diagnostic", ["npm", "run", "diagnostic:check"]),
)
LEGACY_COMMANDS = (
    "npm-clean-install",
    "npm-locked-tree",
    "cargo-fetch",
    "cargo-locked-tree",
    "build",
    "lint",
    "format-check",
    "test",
    "sbom-build",
    "sbom-check",
    "diagnostic",
)
EXPECTED_TOOLCHAINS = (
    "cargo",
    "clippy-driver",
    "git",
    "node",
    "npm",
    "python3",
    "rustc",
    "rustfmt",
)
LEGACY_SCHEMA2_TOOLCHAINS = tuple(
    toolchain for toolchain in EXPECTED_TOOLCHAINS if toolchain != "git"
)
EXPECTED_CONTROLS = {
    "capabilities": "all-dropped",
    "host_source": "complete-committed-tree-in-image",
    "memory_limit_bytes": 12 * 1024 * 1024 * 1024,
    "no_new_privileges": True,
    "pids_limit": 512,
    "root_filesystem": "read-only",
    "runtime": "rootless-podman",
    "runtime_network": "disabled-after-bootstrap",
    "tmpfs_size_bytes": 8 * 1024 * 1024 * 1024,
    "writable_storage": "fresh-tmpfs",
}
LEGACY_CONTROLS = {
    "capabilities": "all-dropped",
    "host_source": "read-only-committed-archive",
    "no_new_privileges": True,
    "root_filesystem": "read-only",
    "runtime": "rootless-podman",
    "writable_storage": "fresh-tmpfs",
}
SHA256_ID = re.compile(r"^sha256:[0-9a-f]{64}$")
HEX_SHA256 = re.compile(r"^[0-9a-f]{64}$")
GIT_ID = re.compile(r"^[0-9a-f]{40,64}$")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _run(
    command: list[str], cwd: Path, timeout: int = 1800
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=timeout,
        check=False,
    )


def _run_bytes(
    command: list[str], cwd: Path, timeout: int = 60
) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        cwd=cwd,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )


def _archive_bytes(root: Path, revision: str) -> bytes:
    archive = _run_bytes(["git", "archive", "--format=tar", revision], root)
    if archive.returncode != 0:
        raise OSError("could not create the committed clean-build source archive")
    return archive.stdout


def _extract_archive(archive: bytes, destination: Path) -> None:
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:") as tar:
        tar.extractall(destination, filter="data")


def git_object_exists(root: Path, revision: str) -> bool:
    result = _run(["git", "cat-file", "-e", f"{revision}^{{commit}}"], root, 60)
    return result.returncode == 0


def git_source_identity(root: Path, revision: str) -> dict[str, str]:
    commit = _run(["git", "rev-parse", f"{revision}^{{commit}}"], root, 60)
    if commit.returncode != 0 or not GIT_ID.fullmatch(commit.stdout.strip()):
        raise OSError("clean-build source revision is invalid")
    resolved = commit.stdout.strip()
    tree = _run(["git", "rev-parse", f"{resolved}^{{tree}}"], root, 60)
    if tree.returncode != 0 or not GIT_ID.fullmatch(tree.stdout.strip()):
        raise OSError("clean-build source tree is invalid")
    archive = _archive_bytes(root, resolved)
    with tempfile.TemporaryDirectory(prefix="agentmage-source-identity-") as temporary:
        extracted = Path(temporary) / "source"
        extracted.mkdir()
        _extract_archive(archive, extracted)
        content_sha256 = committed_tree_content_sha256(extracted)
    return {
        "archive_sha256": hashlib.sha256(archive).hexdigest(),
        "content_sha256": content_sha256,
        "revision": resolved,
        "tree": tree.stdout.strip(),
    }


def expected_toolchains_for_revision(root: Path, revision: str) -> tuple[str, ...]:
    result = _run(
        ["git", "show", f"{revision}:architecture/clean-build-policy.json"],
        root,
        60,
    )
    if result.returncode != 0:
        return ()
    try:
        historical_policy = json.loads(result.stdout)
    except json.JSONDecodeError:
        return ()
    if historical_policy.get("toolchains", {}).get("git") == "platform-packaged":
        return EXPECTED_TOOLCHAINS
    return LEGACY_SCHEMA2_TOOLCHAINS


def _ignored_path_allowed(path: str, patterns: list[str]) -> bool:
    normalized = path.rstrip("/")
    for pattern in patterns:
        if pattern.endswith("/**"):
            prefix = pattern[:-3].rstrip("/")
            if normalized == prefix or normalized.startswith(prefix + "/"):
                return True
            if prefix == "__pycache__" and (
                normalized.endswith("/__pycache__")
                or "/__pycache__/" in normalized
            ):
                return True
        if fnmatch.fnmatch(normalized, pattern) or fnmatch.fnmatch(
            Path(normalized).name, pattern
        ):
            return True
    return False


def working_tree_failures(
    root: Path,
    policy: dict[str, Any],
    revision: str = "HEAD",
    allowed_generated_outputs: tuple[str, ...] = (),
) -> list[str]:
    failures: list[str] = []
    source_binding = policy.get("source_binding", {})
    dirty = _run(
        ["git", "status", "--porcelain=v1", "-z", "--untracked-files=all"],
        root,
        60,
    )
    if dirty.returncode != 0:
        return ["clean-build source status could not be read"]
    for line in dirty.stdout.split("\0"):
        if not line:
            continue
        status = line[:2]
        relative = line[3:]
        if status == " M" and relative in allowed_generated_outputs:
            continue
        if status == "??":
            failures.append("clean-build source has an untracked path")
        elif status[0] != " ":
            failures.append("clean-build source index is dirty")
        elif status[1] != " ":
            failures.append("clean-build source worktree is dirty")

    ignored = _run(
        [
            "git",
            "status",
            "--porcelain=v1",
            "-z",
            "--ignored",
            "--untracked-files=all",
        ],
        root,
        60,
    )
    if ignored.returncode != 0:
        failures.append("clean-build ignored-path status could not be read")
    else:
        patterns = source_binding.get("ignored_exclusions", [])
        for line in ignored.stdout.split("\0"):
            if line.startswith("!! ") and not _ignored_path_allowed(line[3:], patterns):
                failures.append("clean-build source has an unapproved ignored path")

    tree = _run_bytes(["git", "ls-tree", "-r", "-z", revision], root, 60)
    if tree.returncode != 0:
        failures.append("clean-build tracked tree could not be read")
    else:
        for record in tree.stdout.split(b"\0"):
            if not record:
                continue
            metadata, _, _path = record.partition(b"\t")
            mode = metadata.split(b" ", 1)[0]
            if mode == b"120000":
                failures.append("clean-build tracked tree contains a symbolic link")
            elif mode == b"160000":
                failures.append("clean-build tracked tree contains a gitlink")
    return sorted(set(failures))


def validate_policy(policy: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(policy, dict):
        return ["clean-build policy must be an object"]
    if (
        policy.get("schema_version") != 2
        or policy.get("status") != "enforced"
        or policy.get("test_id") != "S-001-IT01"
    ):
        failures.append("clean-build policy identity is invalid")
    if tuple(policy.get("linux_platforms", {})) != EXPECTED_PLATFORMS:
        failures.append("clean-build Linux platform closure drifted")
    if tuple(item.get("id") for item in policy.get("bootstrap_commands", [])) != (
        EXPECTED_BOOTSTRAP_COMMANDS
    ):
        failures.append("clean-build bootstrap command closure drifted")
    if tuple(
        (item.get("id"), item.get("argv"))
        for item in policy.get("bootstrap_commands", [])
    ) != EXPECTED_BOOTSTRAP_ARGV:
        failures.append("clean-build bootstrap command arguments drifted")
    if tuple(
        item.get("id") for item in policy.get("verification_commands", [])
    ) != EXPECTED_VERIFICATION_COMMANDS:
        failures.append("clean-build verification command closure drifted")
    if tuple(
        (item.get("id"), item.get("argv"))
        for item in policy.get("verification_commands", [])
    ) != EXPECTED_VERIFICATION_ARGV:
        failures.append("clean-build verification command arguments drifted")
    if any(
        item.get("network") != "bootstrap-container-build"
        for item in policy.get("bootstrap_commands", [])
    ):
        failures.append("clean-build bootstrap network boundary drifted")
    if any(
        item.get("network") != "container-disabled"
        for item in policy.get("verification_commands", [])
    ):
        failures.append("clean-build verification network boundary drifted")
    source_binding = policy.get("source_binding", {})
    required_binding = {
        "identity": "exact-git-commit-and-tree",
        "tracked_scope": "complete-recursive-git-tree",
        "tracked_exclusions": [],
        "reject_dirty_index": True,
        "reject_dirty_worktree": True,
        "reject_untracked_paths": True,
        "reject_tracked_symlinks": True,
        "reject_gitlinks": True,
    }
    if any(source_binding.get(key) != value for key, value in required_binding.items()):
        failures.append("clean-build source binding was weakened")
    if not source_binding.get("ignored_exclusions"):
        failures.append("clean-build ignored exclusions are not explicit")
    if policy.get("container_controls") != EXPECTED_CONTROLS:
        failures.append("clean-build container controls were weakened")
    if policy.get("execution_user") != {
        "gid": 10001,
        "privileged": False,
        "uid": 10001,
    }:
        failures.append("clean-build execution user is not fixed and unprivileged")
    if policy.get("toolchains", {}).get("git") != "platform-packaged":
        failures.append("clean-build Git runtime dependency is not declared")
    if policy.get("platform_status") != {
        "fedora": "executable",
        "macos": "blocked-macos",
        "ubuntu": "executable",
    }:
        failures.append("clean-build platform status drifted")
    for platform_id, platform in policy.get("linux_platforms", {}).items():
        image = platform.get("base_image", "")
        if "@sha256:" not in image or not SHA256_ID.fullmatch(
            "sha256:" + image.rsplit("@sha256:", 1)[1]
        ):
            failures.append(f"base image is not digest pinned: {platform_id}")
    return failures


def build_report(
    platform_runs: dict[str, dict[str, Any]], source: dict[str, str]
) -> dict[str, Any]:
    linux_passed = all(
        platform_runs.get(platform_id, {}).get("status") == "pass"
        for platform_id in EXPECTED_PLATFORMS
    )
    return {
        "schema_version": 2,
        "test_id": "S-001-IT01",
        "status": "pass-linux" if linux_passed else "fail",
        "source": source,
        "container_controls": EXPECTED_CONTROLS,
        "platform_runs": platform_runs,
        "macos": {
            "status": "blocked-macos",
            "implementation": "not-executed",
            "evidence_substitution": "prohibited",
            "support_claim": "none",
        },
        "summary": {
            "linux_platforms_passed": sum(
                platform_runs.get(platform_id, {}).get("status") == "pass"
                for platform_id in EXPECTED_PLATFORMS
            ),
            "linux_platforms_required": 2,
            "linux_scope_complete": linux_passed,
            "cross_platform_task_complete": False,
        },
    }


def _legacy_input_hash(root: Path, revision: str) -> str | None:
    if not git_object_exists(root, revision):
        return None
    archive = _archive_bytes(root, revision)
    with tempfile.TemporaryDirectory(prefix="agentmage-legacy-clean-source-") as temporary:
        source = Path(temporary) / "source"
        source.mkdir()
        _extract_archive(archive, source)
        policy = read_json(source / "architecture/clean-build-policy.json")
        if policy.get("schema_version") != 1:
            raise OSError("historical clean-build policy is not schema v1")
        return input_tree_sha256(source, policy)


def _validate_source(
    source: Any, schema_version: int, root: Path, failures: list[str]
) -> None:
    if not isinstance(source, dict):
        failures.append("clean-build source identity is invalid")
        return
    revision = source.get("revision", "")
    if not GIT_ID.fullmatch(revision):
        failures.append("clean-build source revision is invalid")
        return
    if schema_version == 1:
        if not HEX_SHA256.fullmatch(source.get("input_sha256", "")):
            failures.append("legacy clean-build source hash is invalid")
            return
        try:
            historical_hash = _legacy_input_hash(root, revision)
        except (OSError, json.JSONDecodeError, KeyError):
            failures.append("legacy clean-build source could not be replayed")
        else:
            if historical_hash is not None and historical_hash != source["input_sha256"]:
                failures.append("legacy clean-build source hash does not replay")
        return
    if set(source) != {"archive_sha256", "content_sha256", "revision", "tree"}:
        failures.append("clean-build source identity field closure is invalid")
        return
    if not HEX_SHA256.fullmatch(source.get("archive_sha256", "")):
        failures.append("clean-build source archive hash is invalid")
    if not HEX_SHA256.fullmatch(source.get("content_sha256", "")):
        failures.append("clean-build source content hash is invalid")
    if not GIT_ID.fullmatch(source.get("tree", "")):
        failures.append("clean-build source tree is invalid")
    if not git_object_exists(root, revision):
        failures.append("clean-build source revision is unavailable")
    else:
        try:
            expected = git_source_identity(root, revision)
        except OSError:
            failures.append("clean-build source identity could not be replayed")
        else:
            if source != expected:
                failures.append("clean-build source identity does not match its revision")


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    policy = read_json(root / "architecture/clean-build-policy.json")
    failures = validate_policy(policy)
    if not isinstance(report, dict):
        return [*failures, "clean-build report must be an object"]
    schema_version = report.get("schema_version")
    if schema_version not in (1, 2) or report.get("test_id") != "S-001-IT01":
        return [*failures, "clean-build report identity is invalid"]
    legacy = schema_version == 1
    expected_controls = LEGACY_CONTROLS if legacy else EXPECTED_CONTROLS
    expected_commands = LEGACY_COMMANDS if legacy else EXPECTED_COMMANDS
    expected_status = "blocked-macos" if legacy else "pass-linux"
    expected_toolchains = (
        LEGACY_SCHEMA2_TOOLCHAINS
        if legacy
        else expected_toolchains_for_revision(
            root, report.get("source", {}).get("revision", "")
        )
    )
    if report.get("status") != expected_status:
        failures.append("clean-build report status is invalid")
    _validate_source(report.get("source"), schema_version, root, failures)
    if report.get("container_controls") != expected_controls:
        failures.append("clean-build report container controls drifted")
    runs = report.get("platform_runs", {})
    if tuple(runs) != EXPECTED_PLATFORMS:
        failures.append("clean-build platform result closure is invalid")
    for platform_id in EXPECTED_PLATFORMS:
        run = runs.get(platform_id, {})
        expected_platform = policy["linux_platforms"][platform_id]
        if run.get("status") != "pass":
            failures.append(f"clean build did not pass: {platform_id}")
        platform = run.get("platform", {})
        if (
            platform.get("id") != platform_id
            or platform.get("os_id") != expected_platform["os_id"]
            or platform.get("version_id") != expected_platform["version_id"]
            or platform.get("architecture") != "x86_64"
        ):
            failures.append(f"clean-build platform identity is invalid: {platform_id}")
        execution = run.get("execution", {})
        if (
            execution.get("effective_uid") != 10001
            or execution.get("effective_gid") != 10001
            or (not legacy and execution.get("cargo_incremental") != "disabled")
            or execution.get("privileged") is not False
            or execution.get("user_class") != "standard-unprivileged"
        ):
            failures.append(f"clean build was not unprivileged: {platform_id}")
        if run.get("source") != report.get("source"):
            failures.append(f"clean-build source identity differs: {platform_id}")
        if not SHA256_ID.fullmatch(run.get("container_image_id", "")):
            failures.append(f"clean-build image identity is invalid: {platform_id}")
        if run.get("base_image") != expected_platform["base_image"]:
            failures.append(f"clean-build base image was substituted: {platform_id}")
        commands = run.get("commands", [])
        if tuple(item.get("id") for item in commands) != expected_commands:
            failures.append(f"clean-build command closure is invalid: {platform_id}")
        if any(item.get("status") != "pass" for item in commands):
            failures.append(f"clean-build command failed: {platform_id}")
        toolchains = run.get("toolchains", [])
        if tuple(item.get("id") for item in toolchains) != expected_toolchains:
            failures.append(f"clean-build toolchain closure is invalid: {platform_id}")
        for toolchain in toolchains:
            if not isinstance(toolchain.get("version"), str) or not toolchain["version"]:
                failures.append(f"clean-build toolchain version is invalid: {platform_id}")
            if not HEX_SHA256.fullmatch(toolchain.get("executable_sha256", "")):
                failures.append(f"clean-build toolchain hash is invalid: {platform_id}")
        checks = run.get("checks", {})
        required_true = {
            "all_commands_passed",
            "clean_home",
            "clean_npm_cache",
            "clean_source_archive",
            "clean_target",
        }
        if legacy:
            required_true.add("supply_chain_unchanged")
        else:
            required_true.add("post_bootstrap_network_denied")
            required_true.add("source_content_verified_during_bootstrap")
            required_true.add("supply_chain_outputs_validated")
        if any(checks.get(key) is not True for key in required_true):
            failures.append(f"clean-build invariant failed: {platform_id}")
        if checks.get("ambient_dependency_detected") is not False:
            failures.append(f"ambient dependency was detected: {platform_id}")
        if run.get("macos_support_claim") != "none":
            failures.append(f"Linux evidence made a Mac support claim: {platform_id}")
    if report.get("macos") != {
        "status": "blocked-macos",
        "implementation": "not-executed",
        "evidence_substitution": "prohibited",
        "support_claim": "none",
    }:
        failures.append("macOS result was weakened or promoted")
    expected_summary = {
        "linux_platforms_passed": 2,
        "linux_platforms_required": 2,
        "cross_platform_task_complete": False,
    }
    if not legacy:
        expected_summary["linux_scope_complete"] = True
    if report.get("summary") != expected_summary:
        failures.append("clean-build summary is invalid")
    return failures


def report_applicability(
    report: dict[str, Any], reviewed_revision: str, root: Path = ROOT
) -> str:
    if report.get("schema_version") == 1:
        return "historical-legacy"
    try:
        reviewed = git_source_identity(root, reviewed_revision)
    except OSError:
        return "reviewed-source-unavailable"
    if report.get("source") != reviewed:
        return "historical"
    policy = read_json(root / "architecture/clean-build-policy.json")
    allowed_report = REPORT_PATH.relative_to(ROOT).as_posix()
    if working_tree_failures(
        root,
        policy,
        reviewed["revision"],
        allowed_generated_outputs=(allowed_report,),
    ):
        return "reviewed-source-dirty"
    return "current-reviewed-source"


def normalized_sha256_id(value: str) -> str:
    candidate = value.strip()
    if re.fullmatch(r"[0-9a-f]{64}", candidate):
        candidate = f"sha256:{candidate}"
    if not SHA256_ID.fullmatch(candidate):
        raise OSError("clean-build image identity is not a SHA-256 value")
    return candidate


def _rootless_podman() -> bool:
    result = _run(
        ["podman", "info", "--format", "{{.Host.Security.Rootless}}"], ROOT, 60
    )
    return result.returncode == 0 and result.stdout.strip() == "true"


def _image_id(tag: str) -> str:
    result = _run(["podman", "image", "inspect", tag, "--format", "{{.Id}}"], ROOT, 60)
    if result.returncode != 0:
        raise OSError("could not resolve clean-build image identity")
    return normalized_sha256_id(result.stdout)


def container_run_argv(
    tag: str,
    platform_id: str,
    source: dict[str, str],
) -> list[str]:
    return [
        "podman",
        "run",
        "--rm",
        "--network=none",
        "--read-only",
        "--cap-drop=all",
        "--security-opt=no-new-privileges",
        "--security-opt=label=disable",
        "--pids-limit=512",
        f"--memory={EXPECTED_CONTROLS['memory_limit_bytes']}",
        "--tmpfs=/tmp:rw,exec,nosuid,nodev,"
        f"size={EXPECTED_CONTROLS['tmpfs_size_bytes']}",
        "--user=10001:10001",
        tag,
        "python3",
        "/source/scripts/clean_standard_build.py",
        "--platform",
        platform_id,
        "--source-revision",
        source["revision"],
        "--source-tree",
        source["tree"],
        "--source-archive-sha256",
        source["archive_sha256"],
        "--source-content-sha256",
        source["content_sha256"],
        "--source-root",
        "/source",
    ]


def run_platform(
    platform_id: str,
    platform: dict[str, str],
    policy: dict[str, Any],
    source: dict[str, str],
    source_root: Path,
) -> dict[str, Any]:
    tag = f"localhost/agentmage-clean-build:{platform_id}"
    build = _run(
        [
            "podman",
            "build",
            "--pull=never",
            "--build-arg",
            f"BASE_IMAGE={platform['base_image']}",
            "--build-arg",
            f"NODE_VERSION={policy['toolchains']['node']}",
            "--build-arg",
            f"NODE_SHA256={policy['download_integrity']['node_linux_x64_sha256']}",
            "--build-arg",
            f"NPM_VERSION={policy['toolchains']['npm']}",
            "--build-arg",
            f"RUST_VERSION={policy['toolchains']['rustc']}",
            "--build-arg",
            "RUSTUP_INIT_SHA256="
            f"{policy['download_integrity']['rustup_init_linux_x64_sha256']}",
            "--build-arg",
            f"SOURCE_CONTENT_SHA256={source['content_sha256']}",
            "--file",
            str(source_root / "release/clean-build/Containerfile.linux"),
            "--tag",
            tag,
            str(source_root),
        ],
        ROOT,
    )
    if build.returncode != 0:
        combined = "\n".join((build.stdout, build.stderr)).replace(
            str(source_root), "<COMMITTED_SOURCE>"
        )
        detail = " | ".join(combined.strip().splitlines()[-8:]) or "unknown"
        raise OSError(f"clean-build image setup failed for {platform_id}: {detail}")
    image_id = _image_id(tag)
    run = _run(container_run_argv(tag, platform_id, source), ROOT)
    if run.returncode != 0:
        detail = f"container exited {run.returncode}"
        try:
            failed_report = json.loads(run.stdout)
        except json.JSONDecodeError:
            output_tail = " | ".join(
                "\n".join((run.stdout, run.stderr)).strip().splitlines()[-20:]
            )
            if output_tail:
                detail = f"{detail}: {output_tail[-2000:]}"
        else:
            failed_commands = [
                item.get("id", "unknown")
                for item in failed_report.get("commands", [])
                if item.get("status") != "pass"
            ]
            failed_checks = sorted(
                key
                for key, value in failed_report.get("checks", {}).items()
                if value is not True and key != "ambient_dependency_detected"
            )
            if failed_report.get("checks", {}).get("ambient_dependency_detected") is True:
                failed_checks.append("ambient_dependency_detected")
            detail = (
                f"{detail}: failed_commands={failed_commands}; "
                f"failed_checks={failed_checks}"
            )
        raise OSError(f"clean build failed for {platform_id}: {detail}")
    try:
        result = json.loads(run.stdout)
    except json.JSONDecodeError as error:
        raise OSError(f"clean build returned invalid evidence for {platform_id}") from error
    bootstrap_results = [
        {
            "id": item["id"],
            "argv": item["argv"],
            "network": item["network"],
            "exit_code": 0,
            "status": "pass",
            "output": "",
            "output_sha256": hashlib.sha256(b"").hexdigest(),
        }
        for item in policy["bootstrap_commands"]
    ]
    result["commands"] = bootstrap_results + result["commands"]
    result["checks"]["source_content_verified_during_bootstrap"] = True
    result["base_image"] = platform["base_image"]
    result["container_image_id"] = image_id
    result["container_controls"] = policy["container_controls"]
    return result


def run_linux_builds(source_revision: str) -> dict[str, Any]:
    policy = read_json(POLICY_PATH)
    failures = validate_policy(policy)
    if failures:
        raise OSError("; ".join(failures))
    source_failures = working_tree_failures(ROOT, policy, source_revision)
    if source_failures:
        raise OSError("; ".join(source_failures))
    if not _rootless_podman():
        raise OSError("rootless Podman is required for clean Linux build evidence")
    source = git_source_identity(ROOT, source_revision)
    archive = _archive_bytes(ROOT, source["revision"])
    if hashlib.sha256(archive).hexdigest() != source["archive_sha256"]:
        raise OSError("clean-build source archive identity drifted")
    with tempfile.TemporaryDirectory(prefix="agentmage-clean-source-") as temporary:
        source_root = Path(temporary) / "source"
        source_root.mkdir()
        _extract_archive(archive, source_root)
        runs = {
            platform_id: run_platform(
                platform_id,
                policy["linux_platforms"][platform_id],
                policy,
                source,
                source_root,
            )
            for platform_id in EXPECTED_PLATFORMS
        }
    return build_report(runs, source)


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read clean-build report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-linux", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    parser.add_argument("--require-reviewed-revision")
    args = parser.parse_args()
    try:
        if args.run_linux:
            report = run_linux_builds(args.source_revision)
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_text(
                json.dumps(report, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
        report = read_json(REPORT_PATH)
        failures = validate_report(report)
        if args.require_reviewed_revision:
            applicability = report_applicability(
                report, args.require_reviewed_revision
            )
            if applicability != "current-reviewed-source":
                failures.append(
                    "clean-build report is not applicable to the reviewed revision: "
                    f"{applicability}"
                )
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(f"clean-build evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"clean-build evidence failed: {failure}", file=sys.stderr)
        return 1
    applicability = report_applicability(report, "HEAD")
    print(f"clean-build evidence passed: applicability={applicability}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

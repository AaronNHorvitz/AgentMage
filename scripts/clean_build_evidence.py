#!/usr/bin/env python3
"""Run and validate clean standard-user builds on executable platforms."""

from __future__ import annotations

import argparse
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
    from scripts.clean_standard_build import input_tree_sha256
except ModuleNotFoundError:
    from clean_standard_build import input_tree_sha256


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
EXPECTED_COMMANDS = (
    "npm-clean-install",
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
EXPECTED_CONTROLS = {
    "capabilities": "all-dropped",
    "host_source": "read-only-committed-archive",
    "no_new_privileges": True,
    "root_filesystem": "read-only",
    "runtime": "rootless-podman",
    "writable_storage": "fresh-tmpfs",
}
SHA256_ID = re.compile(r"^sha256:[0-9a-f]{64}$")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_policy(policy: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(policy, dict):
        return ["clean-build policy must be an object"]
    if (
        policy.get("schema_version") != 1
        or policy.get("status") != "enforced"
        or policy.get("test_id") != "S-001-IT01"
    ):
        failures.append("clean-build policy identity is invalid")
    if tuple(policy.get("linux_platforms", {})) != EXPECTED_PLATFORMS:
        failures.append("clean-build Linux platform closure drifted")
    if tuple(item.get("id") for item in policy.get("commands", [])) != EXPECTED_COMMANDS:
        failures.append("clean-build command closure drifted")
    if policy.get("container_controls") != EXPECTED_CONTROLS:
        failures.append("clean-build container controls were weakened")
    if policy.get("execution_user") != {
        "gid": 10001,
        "privileged": False,
        "uid": 10001,
    }:
        failures.append("clean-build execution user is not fixed and unprivileged")
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
    platform_runs: dict[str, dict[str, Any]],
    source_revision: str,
    root: Path = ROOT,
) -> dict[str, Any]:
    policy = read_json(root / "architecture/clean-build-policy.json")
    linux_passed = all(
        platform_runs.get(platform_id, {}).get("status") == "pass"
        for platform_id in EXPECTED_PLATFORMS
    )
    return {
        "schema_version": 1,
        "test_id": "S-001-IT01",
        "status": "blocked-macos" if linux_passed else "fail",
        "source": {
            "input_sha256": input_tree_sha256(root, policy),
            "revision": source_revision,
        },
        "container_controls": policy["container_controls"],
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
            "cross_platform_task_complete": False,
        },
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    policy = read_json(root / "architecture/clean-build-policy.json")
    failures = validate_policy(policy)
    if not isinstance(report, dict):
        return [*failures, "clean-build report must be an object"]
    if report.get("schema_version") != 1 or report.get("test_id") != "S-001-IT01":
        failures.append("clean-build report identity is invalid")
    if report.get("status") != "blocked-macos":
        failures.append("clean-build report must retain blocked macOS status")
    source = report.get("source", {})
    if not re.fullmatch(r"[0-9a-f]{40}", source.get("revision", "")):
        failures.append("clean-build source revision is invalid")
    if source.get("input_sha256") != input_tree_sha256(root, policy):
        failures.append("clean-build source input hash is stale")
    if report.get("container_controls") != EXPECTED_CONTROLS:
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
        if tuple(item.get("id") for item in commands) != EXPECTED_COMMANDS:
            failures.append(f"clean-build command closure is invalid: {platform_id}")
        if any(item.get("status") != "pass" for item in commands):
            failures.append(f"clean-build command failed: {platform_id}")
        checks = run.get("checks", {})
        required_true = {
            "all_commands_passed",
            "clean_home",
            "clean_npm_cache",
            "clean_source_archive",
            "clean_target",
            "supply_chain_unchanged",
        }
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
    if report.get("summary") != {
        "linux_platforms_passed": 2,
        "linux_platforms_required": 2,
        "cross_platform_task_complete": False,
    }:
        failures.append("clean-build summary is invalid")
    return failures


def _run(command: list[str], cwd: Path, timeout: int = 1800) -> subprocess.CompletedProcess[str]:
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


def _source_archive(source_revision: str, destination: Path) -> None:
    archive = subprocess.run(
        ["git", "archive", "--format=tar", source_revision],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=60,
        check=False,
    )
    if archive.returncode != 0:
        raise OSError("could not create the committed clean-build source archive")
    with tarfile.open(fileobj=io.BytesIO(archive.stdout), mode="r:") as tar:
        tar.extractall(destination, filter="data")


def _rootless_podman() -> bool:
    result = _run(
        ["podman", "info", "--format", "{{.Host.Security.Rootless}}"], ROOT, 60
    )
    return result.returncode == 0 and result.stdout.strip() == "true"


def _image_id(tag: str) -> str:
    result = _run(["podman", "image", "inspect", tag, "--format", "{{.Id}}"], ROOT, 60)
    if result.returncode != 0 or not SHA256_ID.fullmatch(result.stdout.strip()):
        raise OSError("could not resolve clean-build image identity")
    return result.stdout.strip()


def run_platform(
    platform_id: str,
    platform: dict[str, str],
    policy: dict[str, Any],
    source_revision: str,
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
            "--file",
            str(source_root / "release/clean-build/Containerfile.linux"),
            "--tag",
            tag,
            str(source_root / "release/clean-build"),
        ],
        ROOT,
    )
    if build.returncode != 0:
        raise OSError(f"clean-build image setup failed for {platform_id}")
    image_id = _image_id(tag)
    run = _run(
        [
            "podman",
            "run",
            "--rm",
            "--read-only",
            "--cap-drop=all",
            "--security-opt=no-new-privileges",
            "--security-opt=label=disable",
            "--pids-limit=512",
            "--memory=4g",
            "--tmpfs=/tmp:rw,exec,nosuid,nodev,size=2147483648",
            "--mount",
            f"type=bind,src={source_root},dst=/source,ro=true",
            "--user=10001:10001",
            tag,
            "python3",
            "/source/scripts/clean_standard_build.py",
            "--platform",
            platform_id,
            "--source-revision",
            source_revision,
            "--source-root",
            "/source",
        ],
        ROOT,
    )
    if run.returncode != 0:
        detail = run.stderr.strip().splitlines()[-1] if run.stderr.strip() else "unknown"
        raise OSError(f"clean build failed for {platform_id}: {detail}")
    try:
        result = json.loads(run.stdout)
    except json.JSONDecodeError as error:
        raise OSError(f"clean build returned invalid evidence for {platform_id}") from error
    result["base_image"] = platform["base_image"]
    result["container_image_id"] = image_id
    result["container_controls"] = policy["container_controls"]
    return result


def run_linux_builds(source_revision: str) -> dict[str, Any]:
    policy = read_json(POLICY_PATH)
    failures = validate_policy(policy)
    if failures:
        raise OSError("; ".join(failures))
    if not _rootless_podman():
        raise OSError("rootless Podman is required for clean Linux build evidence")
    revision = _run(["git", "rev-parse", source_revision], ROOT, 60)
    if revision.returncode != 0 or not re.fullmatch(r"[0-9a-f]{40}", revision.stdout.strip()):
        raise OSError("clean-build source revision is invalid")
    resolved_revision = revision.stdout.strip()
    with tempfile.TemporaryDirectory(prefix="agentmage-clean-source-") as temporary:
        source_root = Path(temporary) / "source"
        source_root.mkdir()
        _source_archive(resolved_revision, source_root)
        expected_hash = input_tree_sha256(ROOT, policy)
        if input_tree_sha256(source_root, policy) != expected_hash:
            raise OSError("committed source archive differs from the working input closure")
        runs = {
            platform_id: run_platform(
                platform_id,
                policy["linux_platforms"][platform_id],
                policy,
                resolved_revision,
                source_root,
            )
            for platform_id in EXPECTED_PLATFORMS
        }
    return build_report(runs, resolved_revision)


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
    args = parser.parse_args()
    try:
        if args.run_linux:
            report = run_linux_builds(args.source_revision)
            REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
            REPORT_PATH.write_text(
                json.dumps(report, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
        failures = check_report()
    except (OSError, subprocess.SubprocessError) as error:
        print(f"clean-build evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"clean-build evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Fedora and Ubuntu clean builds passed; macOS remains blocked")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

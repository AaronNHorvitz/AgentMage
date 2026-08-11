#!/usr/bin/env python3
"""Run shared logical path conformance on fake, Fedora, and Ubuntu adapters."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-6/story-6.1/path-platform-conformance.json"
UBUNTU_IMAGE = "localhost/agentmage-clean-build:ubuntu-x86_64"
FAKE_TEST = "logical_fixture_has_equivalent_fake_policy_semantics"
LINUX_TEST = "tests::logical_fixture_has_equivalent_linux_policy_semantics"
SOURCE_PATHS = (
    "fixtures/paths/README.md",
    "fixtures/paths/v1/logical-input.txt",
    "kernel/contracts/tests/platform_path_contract.rs",
    "platforms/linux/src/lib.rs",
    "scripts/path_platform_conformance.py",
    "tests/test_path_platform_conformance.py",
)


class PathPlatformConformanceError(ValueError):
    """Raised when cross-platform path conformance is stale or overclaimed."""


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-platform-path-", dir=path.parent)
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


def run_command(argv: list[str], root: Path = ROOT, timeout: int = 180) -> str:
    completed = subprocess.run(
        argv, cwd=root, check=False, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=timeout,
    )
    if completed.returncode != 0:
        raise PathPlatformConformanceError(f"conformance command failed: {argv[0]}")
    return completed.stdout


def run_host_tests(root: Path = ROOT) -> list[dict[str, Any]]:
    run_command([
        "cargo", "test", "--locked", "--offline", "-p", "agentmage-kernel-contracts",
        "--test", "platform_path_contract", FAKE_TEST, "--", "--exact", "--test-threads=1",
    ], root)
    run_command([
        "cargo", "test", "--locked", "--offline", "-p", "agentmage-platform-linux",
        LINUX_TEST, "--", "--exact", "--test-threads=1",
    ], root)
    os_release = Path("/etc/os-release").read_text(encoding="utf-8")
    if "ID=fedora" not in os_release or "VERSION_ID=44" not in os_release:
        raise PathPlatformConformanceError("host is not the declared Fedora 44 platform")
    return [
        {
            "adapter": "deterministic-fake", "platform": "shared-contract",
            "status": "pass", "policy_decision": "admit-canonical-read",
            "identity_evidence": "deterministic-domain-separated-digest",
        },
        {
            "adapter": "linux", "platform": "fedora-44-x86_64",
            "status": "pass", "policy_decision": "admit-canonical-read",
            "identity_evidence": "linux-mount-object-digests-and-exact-preimage",
        },
    ]


def git_archive(revision: str, destination: Path, root: Path = ROOT) -> None:
    completed = subprocess.run(
        ["git", "archive", "--format=tar", revision], cwd=root, check=False,
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60,
    )
    if completed.returncode != 0:
        raise PathPlatformConformanceError("could not archive conformance source")
    with tarfile.open(fileobj=io.BytesIO(completed.stdout), mode="r:") as archive:
        archive.extractall(destination, filter="data")


def image_identity(root: Path = ROOT) -> str:
    value = run_command(
        ["podman", "image", "inspect", UBUNTU_IMAGE, "--format", "{{.Id}}"], root, 60
    ).strip()
    if re.fullmatch(r"[0-9a-f]{64}", value):
        value = f"sha256:{value}"
    if re.fullmatch(r"sha256:[0-9a-f]{64}", value) is None:
        raise PathPlatformConformanceError("Ubuntu image identity is invalid")
    return value


def run_ubuntu_test(revision: str, root: Path = ROOT) -> dict[str, Any]:
    registry = (Path.home() / ".cargo/registry").resolve()
    if not registry.is_dir():
        raise PathPlatformConformanceError("read-only Cargo dependency cache is unavailable")
    with tempfile.TemporaryDirectory(prefix="agentmage-ubuntu-path-") as temporary:
        source = Path(temporary) / "source"
        source.mkdir()
        source.chmod(0o755)
        git_archive(revision, source, root)
        command = [
            "podman", "run", "--rm", "--network", "none", "--read-only",
            "--cap-drop=all", "--security-opt=no-new-privileges",
            "--security-opt=label=disable", "--pids-limit=256", "--memory=2g",
            "--tmpfs=/tmp:rw,exec,nosuid,nodev,size=2147483648",
            "--mount", f"type=bind,src={source},dst=/source,ro=true",
            "--mount", f"type=bind,src={registry},dst=/registry,ro=true",
            "--user=10001:10001", UBUNTU_IMAGE, "sh", "-lc",
            "mkdir -p /tmp/cargo /tmp/work /tmp/target"
            " && cp -a /registry /tmp/cargo/registry"
            " && cp -a /source/. /tmp/work/"
            " && cd /tmp/work"
            " && CARGO_HOME=/tmp/cargo CARGO_TARGET_DIR=/tmp/target"
            " CARGO_NET_OFFLINE=true cargo test --locked --offline"
            " -p agentmage-platform-linux"
            f" {LINUX_TEST} -- --exact --test-threads=1",
        ]
        run_command(command, root, 240)
    return {
        "adapter": "linux", "platform": "ubuntu-26.04-x86_64",
        "status": "pass", "policy_decision": "admit-canonical-read",
        "identity_evidence": "linux-mount-object-digests-and-exact-preimage",
        "container_image": UBUNTU_IMAGE,
        "container_image_id": image_identity(root),
        "container_network": "none",
        "source_revision": revision,
    }


def git_revision(candidate: str = "HEAD", root: Path = ROOT) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise PathPlatformConformanceError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str, root: Path = ROOT) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"], cwd=root, check=False,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10,
    )
    if completed.returncode != 0:
        raise PathPlatformConformanceError(f"source is absent at revision: {relative}")
    return completed.stdout


def build_report(reference_revision: str, root: Path = ROOT) -> dict[str, Any]:
    results = [*run_host_tests(root), run_ubuntu_test(reference_revision, root)]
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", reference_revision, "HEAD"],
        cwd=root, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
    )
    if ancestor.returncode != 0:
        raise PathPlatformConformanceError("reference revision is not an ancestor of HEAD")
    sources = []
    for relative in SOURCE_PATHS:
        committed = git_file(reference_revision, relative, root)
        if committed != (root / relative).read_bytes():
            raise PathPlatformConformanceError(f"source differs from reference revision: {relative}")
        sources.append({"path": relative, "sha256": sha256_bytes(committed)})
    return {
        "schema_version": 1,
        "task_id": "6.1.3.4",
        "artifact_id": "shared-logical-path-platform-conformance",
        "status": "pass-all-available-non-macos-platforms",
        "reference_revision": reference_revision,
        "fixture": {
            "path": "fixtures/paths/v1/logical-input.txt",
            "sha256": sha256_bytes((root / "fixtures/paths/v1/logical-input.txt").read_bytes()),
            "workspace_path": ["docs", "logical-input.txt"],
            "intent": "content-hash",
        },
        "sources": sources,
        "results": results,
        "coverage": {
            "available_adapter_result_count": 3,
            "equivalent_policy_decision_count": 3,
            "networked_test_count": 0,
            "out_of_workspace_access_count": 0,
        },
        "platform_status": {
            "deterministic_fake": "verified",
            "fedora_44": "verified-local",
            "ubuntu_26_04": "verified-no-network-container",
            "macos": "blocked-macos",
        },
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "limitations": ["macOS adapter implementation and execution remain blocked"],
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["path platform conformance report must be an object"]
    if (
        value.get("schema_version") != 1 or value.get("task_id") != "6.1.3.4"
        or value.get("artifact_id") != "shared-logical-path-platform-conformance"
        or value.get("status") != "pass-all-available-non-macos-platforms"
        or re.fullmatch(r"[0-9a-f]{40}", str(value.get("reference_revision"))) is None
    ):
        failures.append("path platform conformance identity changed")
    if value.get("coverage") != {
        "available_adapter_result_count": 3,
        "equivalent_policy_decision_count": 3,
        "networked_test_count": 0,
        "out_of_workspace_access_count": 0,
    }:
        failures.append("path platform conformance coverage changed")
    results = value.get("results")
    if (
        not isinstance(results, list) or len(results) != 3
        or any(result.get("status") != "pass" for result in results)
        or any(result.get("policy_decision") != "admit-canonical-read" for result in results)
        or results[2].get("container_network") != "none"
    ):
        failures.append("path platform conformance results are incomplete")
    if value.get("platform_status") != {
        "deterministic_fake": "verified", "fedora_44": "verified-local",
        "ubuntu_26_04": "verified-no-network-container", "macos": "blocked-macos",
    }:
        failures.append("path platform status changed")
    if value.get("macos_evidence_substituted") is not False or value.get("release_claim") != "none":
        failures.append("path platform conformance made an unsupported claim")
    if not isinstance(value.get("limitations"), list) or len(value["limitations"]) != 1:
        failures.append("path platform limitations are incomplete")
    return failures


def write_report(reference_revision: str, root: Path = ROOT) -> None:
    write_atomic(REPORT_PATH, pretty_json(build_report(reference_revision, root)))


def check_report(root: Path = ROOT) -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["reference_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise PathPlatformConformanceError(f"cannot read path platform report: {error}") from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision, root):
        failures.append("path platform conformance report is stale or malformed")
    if failures:
        raise PathPlatformConformanceError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.write:
            write_report(git_revision(args.source_revision))
        check_report()
    except (OSError, UnicodeError, PathPlatformConformanceError, subprocess.SubprocessError) as error:
        print(f"Path platform conformance failed: {error}", file=sys.stderr)
        return 1
    print("Story 6.1 path conformance passed on every available non-macOS platform")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

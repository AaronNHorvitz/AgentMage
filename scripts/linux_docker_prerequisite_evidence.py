#!/usr/bin/env python3
"""Build and validate packaged Docker guard/collector prerequisite evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Final

try:
    from scripts.package_candidate import MANIFEST_PATH, PAYLOAD_FILES, build_all, verify_payload
    from scripts.package_lifecycle import extract_deb, extract_rpm
except ModuleNotFoundError:
    from package_candidate import MANIFEST_PATH, PAYLOAD_FILES, build_all, verify_payload
    from package_lifecycle import extract_deb, extract_rpm


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT
    / "artifacts/sprints/sprint-9/story-9.2/linux-docker-production-prerequisites.json"
)
SOURCE_PATHS: Final = (
    "Cargo.lock",
    "Cargo.toml",
    "LICENSE",
    "docs/decisions/0033-production-docker-guard-and-observer-prerequisite.md",
    "docs/decisions/0036-observe-network-namespace-through-procfs.md",
    "docs/decisions/0037-admit-canonical-runner-digest-reference.md",
    "model-profiles/runtimes/docker-model-runner-guard-v1-linux-x86_64.json",
    "model-profiles/runtimes/docker-model-runner-v1.2.6-linux-x86_64.json",
    "package-lock.json",
    "package.json",
    "packaging/linux/agentmage-release.spec.in",
    "packaging/linux/agentmage.spec.in",
    "platforms/linux-inference/Cargo.toml",
    "platforms/linux-inference/src/docker_guard.rs",
    "platforms/linux-inference/src/docker_guard_main.rs",
    "platforms/linux-inference/src/docker_guard_service.rs",
    "platforms/linux-inference/src/docker_http_observer.rs",
    "platforms/linux-inference/src/docker_linux_observer.rs",
    "platforms/linux-inference/src/docker_live_collector.rs",
    "platforms/linux-inference/src/docker_preflight.rs",
    "platforms/linux-inference/src/docker_runtime.rs",
    "platforms/linux-inference/src/docker_topology_collector.rs",
    "platforms/linux-inference/src/docker_topology_collector_main.rs",
    "platforms/linux-inference/src/lib.rs",
    "platforms/linux-inference/src/main.rs",
    "platforms/linux-inference/tests/process_boundary.rs",
    "rust-toolchain.toml",
    "scripts/linux_clean_image_acceptance.py",
    "scripts/linux_docker_prerequisite_evidence.py",
    "scripts/linux_package_lifecycle_evidence.py",
    "scripts/package_candidate.py",
    "scripts/package_lifecycle.py",
    "scripts/package_release_lifecycle.py",
    "shells/host/src/linux_bootstrap.rs",
    "shells/host/src/package_verify.rs",
    "tests/test_linux_clean_image_acceptance.py",
    "tests/test_linux_docker_prerequisite_evidence.py",
    "tests/test_linux_package_lifecycle_evidence.py",
    "tests/test_package_candidate.py",
    "tests/test_package_lifecycle.py",
)
EXPECTED_GUARD_DESCRIPTOR: Final = {
    "accepted_operations": ["serve-one-session", "self-check"],
    "authority": "guarded-inference-transport-only",
    "component_id": "agentmage-docker-guard",
    "docker_control": False,
    "enabled": False,
    "network_egress": False,
    "protocol_version": 1,
    "raw_target": "private-namespace-loopback-only",
    "sessions": 1,
}
EXPECTED_COLLECTOR_DESCRIPTOR: Final = {
    "accepted_operations": ["observe", "self-check", "validate-observation-stdin"],
    "authority": "docker-topology-observation-only",
    "component_id": "agentmage-docker-topology-collector",
    "docker_mutation": False,
    "enabled": False,
    "network_egress": False,
    "preflight_contract_version": 3,
    "protocol_version": 2,
}
MUTATION_COVERAGE: Final = {
    "guard": [
        "bootstrap-identity-and-bounds",
        "caller-class-uid-process-cgroup-session",
        "challenge-secret-and-peer-authentication",
        "frame-and-request-path-bounds",
        "namespace-listener-route-and-socket-topology",
        "profile-process-mount-and-executable-identity",
        "replay-upstream-failure-and-teardown",
    ],
    "collector": [
        "administrator-before-input",
        "docker-read-route-allowlist-and-response-bounds",
        "explicit-process-and-source-identity",
        "freshness-replay-and-private-marker",
        "interface-route-listener-and-mount-parsing",
        "malformed-partial-unknown-and-oversized-input",
        "production-tag-proxy-mount-listener-and-duplicate-guard-drift",
    ],
    "package": [
        "exact-six-file-manifest",
        "executable-mode-hash-and-presence",
        "rpm-deb-cross-format-equivalence",
        "signed-and-unsigned-package-boundary",
    ],
}
LIMITATIONS: Final = [
    "The production guard and topology collector are packaged and executable, but only their inactive self-check operations were run for this artifact.",
    "Docker Engine and the Docker CLI are absent on this Fedora host; no daemon, socket, container, namespace, firewall, image, model, or packet path was inspected live.",
    "The administrator-only collector observation operation was not executed and no synthetic observation is represented as live evidence.",
    "Clean Fedora and Ubuntu Docker execution, hostile reachability, independent control disablement, inference, model admission, and release support remain later Story 9.2 gates.",
]
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")


class LinuxDockerPrerequisiteEvidenceError(ValueError):
    """Raised when prerequisite evidence is stale, malformed, or overstated."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-docker-prerequisite-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def run(arguments: list[str], timeout: int = 900) -> str:
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
        check=False,
    )
    if result.returncode != 0:
        raise LinuxDockerPrerequisiteEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return result.stdout


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        timeout=20,
        check=False,
    )
    revision = result.stdout.strip()
    if result.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise LinuxDockerPrerequisiteEvidenceError("source revision is unavailable")
    return revision


def committed_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=20,
        check=False,
    )
    if result.returncode != 0:
        raise LinuxDockerPrerequisiteEvidenceError(f"committed source absent: {relative}")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    for relative in SOURCE_PATHS:
        content = committed_file(revision, relative)
        current = ROOT / relative
        if not current.is_file() or current.read_bytes() != content:
            raise LinuxDockerPrerequisiteEvidenceError(f"source differs from revision: {relative}")
        records.append(
            {"bytes": len(content), "path": relative, "sha256": sha256_bytes(content)}
        )
    return records


def host_identity() -> dict[str, Any]:
    values = {}
    for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            values[key] = value.strip('"')
    return {
        "architecture": platform.machine(),
        "distribution": values.get("ID"),
        "version": values.get("VERSION_ID"),
        "docker_cli_available": shutil.which("docker") is not None,
        "live_collector_executed": False,
    }


def component_record(root: Path, package_format: str) -> dict[str, Any]:
    verify_payload(root)
    manifest_path = root / MANIFEST_PATH
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    executable_root = root / "usr/libexec/agentmage"
    guard = json.loads(run([str(executable_root / "agentmage-docker-guard"), "--self-check"]))
    collector = json.loads(
        run([str(executable_root / "agentmage-docker-topology-collector"), "--self-check"])
    )
    if guard != EXPECTED_GUARD_DESCRIPTOR or collector != EXPECTED_COLLECTOR_DESCRIPTOR:
        raise LinuxDockerPrerequisiteEvidenceError("packaged component descriptor changed")
    return {
        "collector_descriptor": collector,
        "format": package_format,
        "guard_descriptor": guard,
        "manifest_sha256": sha256_file(manifest_path),
        "payload": manifest["files"],
    }


def build_report(source_revision: str) -> dict[str, Any]:
    revision = git_revision(source_revision)
    commands = [
        ["cargo", "test", "-p", "agentmage-platform-linux-inference", "--locked"],
        ["cargo", "test", "-p", "agentmage-host", "--locked"],
        ["cargo", "clippy", "-p", "agentmage-platform-linux-inference", "--all-targets", "--locked", "--", "-D", "warnings"],
        ["python3", "-m", "unittest", "tests.test_package_candidate", "tests.test_package_lifecycle", "tests.test_linux_clean_image_acceptance", "tests.test_linux_package_lifecycle_evidence", "tests.test_linux_docker_prerequisite_evidence"],
        ["cargo", "build", "--workspace", "--release", "--locked"],
        ["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"],
    ]
    for command in commands:
        run(command)
    host = host_identity()
    if host != {
        "architecture": "x86_64",
        "distribution": "fedora",
        "version": "44",
        "docker_cli_available": False,
        "live_collector_executed": False,
    }:
        raise LinuxDockerPrerequisiteEvidenceError(
            "this artifact requires the declared Docker-absent Fedora host"
        )
    with tempfile.TemporaryDirectory(prefix="agentmage-docker-prerequisite-") as directory:
        temporary = Path(directory)
        first = build_all(temporary / "first")
        second = build_all(temporary / "second")
        if any(first[name].read_bytes() != second[name].read_bytes() for name in first):
            raise LinuxDockerPrerequisiteEvidenceError("package rebuild drifted")
        rpm_root = temporary / "rpm"
        deb_root = temporary / "deb"
        rpm_root.mkdir()
        deb_root.mkdir()
        extract_rpm(first["rpm"], rpm_root)
        extract_deb(first["deb"], deb_root)
        components = [component_record(rpm_root, "rpm"), component_record(deb_root, "deb")]
        if components[0]["payload"] != components[1]["payload"]:
            raise LinuxDockerPrerequisiteEvidenceError("cross-format payload drifted")
        packages = {
            name: {
                "bytes": path.stat().st_size,
                "filename": path.name,
                "sha256": sha256_file(path),
            }
            for name, path in sorted(first.items())
        }
    return {
        "artifact_id": "linux-docker-production-prerequisites",
        "components": components,
        "docker_engine_directly_tested": False,
        "host": host,
        "limitations": LIMITATIONS,
        "live_topology_inspected": False,
        "mutation_coverage": MUTATION_COVERAGE,
        "packages": packages,
        "reproducible_package_rebuild": True,
        "release_claim": "none",
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-packaged-prerequisites-no-live-docker",
        "task_ids": ["9.2.1.5"],
    }


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["Docker prerequisite report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-docker-production-prerequisites"
        or value.get("task_ids") != ["9.2.1.5"]
        or value.get("status") != "pass-packaged-prerequisites-no-live-docker"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Docker prerequisite report identity changed")
    components = value.get("components")
    expected_paths = [path.as_posix() for path in sorted(PAYLOAD_FILES)]
    expected_modes = [0o755, 0o755, 0o755, 0o755, 0o644, 0o644]
    if (
        not isinstance(components, list)
        or [item.get("format") for item in components if isinstance(item, dict)]
        != ["rpm", "deb"]
        or any(
            item.get("guard_descriptor") != EXPECTED_GUARD_DESCRIPTOR
            or item.get("collector_descriptor") != EXPECTED_COLLECTOR_DESCRIPTOR
            or SHA256.fullmatch(str(item.get("manifest_sha256"))) is None
            or [record.get("path") for record in item.get("payload", [])] != expected_paths
            or [record.get("mode") for record in item.get("payload", [])] != expected_modes
            or any(
                SHA256.fullmatch(str(record.get("sha256"))) is None
                or not isinstance(record.get("size"), int)
                or record.get("size", 0) <= 0
                for record in item.get("payload", [])
            )
            for item in components or []
        )
        or (len(components) == 2 and components[0].get("payload") != components[1].get("payload"))
    ):
        failures.append("Docker prerequisite component closure changed")
    packages = value.get("packages")
    if (
        not isinstance(packages, dict)
        or sorted(packages) != ["deb", "rpm", "vsix"]
        or any(
            SHA256.fullmatch(str(record.get("sha256"))) is None
            or not isinstance(record.get("bytes"), int)
            or record.get("bytes", 0) <= 0
            or not isinstance(record.get("filename"), str)
            for record in packages.values()
        )
        or value.get("reproducible_package_rebuild") is not True
    ):
        failures.append("Docker prerequisite package closure changed")
    if value.get("mutation_coverage") != MUTATION_COVERAGE:
        failures.append("Docker prerequisite mutation closure changed")
    host = value.get("host", {})
    if (
        host.get("architecture") != "x86_64"
        or host.get("distribution") != "fedora"
        or host.get("version") != "44"
        or host.get("docker_cli_available") is not False
        or host.get("live_collector_executed") is not False
    ):
        failures.append("Docker-absent prerequisite host evidence changed")
    if (
        value.get("docker_engine_directly_tested") is not False
        or value.get("live_topology_inspected") is not False
        or value.get("release_claim") != "none"
        or value.get("limitations") != LIMITATIONS
    ):
        failures.append("Docker prerequisite report made a live or release overclaim")
    sources = value.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("Docker prerequisite source closure changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item.get("bytes", 0) <= 0
        or SHA256.fullmatch(str(item.get("sha256"))) is None
        for item in sources
    ):
        failures.append("Docker prerequisite source identity is invalid")
    return failures


def check_report() -> list[str]:
    if not REPORT_PATH.is_file():
        return ["Docker prerequisite report is missing"]
    value = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    failures = validate_report(value)
    if failures:
        return failures
    try:
        expected_sources = source_records(value["source_revision"])
    except LinuxDockerPrerequisiteEvidenceError as error:
        return [str(error)]
    if value["sources"] != expected_sources:
        return ["Docker prerequisite source evidence is stale"]
    return []


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            report = build_report(arguments.source_revision)
            failures = validate_report(report)
            if failures:
                raise LinuxDockerPrerequisiteEvidenceError("; ".join(failures))
            write_atomic(REPORT_PATH, canonical_json(report))
        failures = check_report()
        if failures:
            raise LinuxDockerPrerequisiteEvidenceError("; ".join(failures))
    except (LinuxDockerPrerequisiteEvidenceError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"Docker prerequisite evidence failed: {error}", file=sys.stderr)
        return 1
    print("Docker guard and collector prerequisite evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

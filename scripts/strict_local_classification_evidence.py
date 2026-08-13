#!/usr/bin/env python3
"""Build and validate strict-local cross-layer classification evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
BOUNDARY_FIXTURE_PATH: Final = (
    "fixtures/strict-local-boundary/v1/classification-fixtures.json"
)
STORAGE_FIXTURE_PATH: Final = (
    "fixtures/strict-local-storage/v1/detection-fixtures.json"
)
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-10/story-10.1/strict-local-classification.json"
)
SOURCE_PATHS: Final = (
    "Cargo.lock",
    BOUNDARY_FIXTURE_PATH,
    STORAGE_FIXTURE_PATH,
    "kernel/contracts/src/network.rs",
    "kernel/engine/src/strict_local.rs",
    "platforms/linux/Cargo.toml",
    "platforms/linux/src/lib.rs",
    "platforms/linux/src/strict_local.rs",
    "platforms/linux/tests/strict_local_classification.rs",
    "scripts/strict_local_classification_evidence.py",
    "security/strict-local-source-policy.json",
    "tests/test_strict_local_classification_evidence.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "--test",
            "strict_local_classification",
            "--locked",
        ),
        "1 passed; 0 failed",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "strict_local::tests",
            "--locked",
        ),
        "14 passed; 0 failed",
    ),
    (
        (
            "python3",
            "-m",
            "unittest",
            "tests.test_strict_local_classification_evidence",
        ),
        "Ran 5 tests",
    ),
    (
        ("npm", "run", "product:lint"),
        "Structural effect mediation boundary validated.",
    ),
)
IP_CASES: Final = (
    ("ipv4-unspecified", "0.0.0.0", "unspecified"),
    ("ipv4-loopback", "127.0.0.1", "loopback"),
    ("ipv4-link-local", "169.254.1.1", "link_local"),
    ("ipv4-private", "10.1.2.3", "private_lan"),
    ("ipv4-shared", "100.64.0.1", "private_lan"),
    ("ipv4-multicast", "224.0.0.1", "multicast"),
    ("ipv4-external", "192.0.2.1", "external"),
    ("ipv6-unspecified", "::", "unspecified"),
    ("ipv6-loopback", "::1", "loopback"),
    ("ipv6-link-local", "fe80::1", "link_local"),
    ("ipv6-private", "fd00::1", "private_lan"),
    ("ipv6-multicast", "ff02::1", "multicast"),
    ("ipv6-external", "2001:db8::1", "external"),
)
POLICY_CASES: Final = (
    ("native-exact", "native", "kernel_native_inference_adapter", "authenticated_local_socket", "authenticated_unix_socket", "match", True, "allow"),
    ("docker-exact", "docker", "kernel_docker_inference_adapter", "loopback", "guarded_loopback_tcp", "match", True, "allow"),
    ("vscode-direct", "native", "visual_studio_code_extension", "authenticated_local_socket", "authenticated_unix_socket", "match", True, "block"),
    ("tool-direct", "docker", "tool_worker", "loopback", "guarded_loopback_tcp", "match", True, "block"),
    ("unauthenticated-unix", "native", "kernel_native_inference_adapter", "local_socket", "authenticated_unix_socket", "match", False, "block"),
    ("unspecified", "native", "kernel_native_inference_adapter", "unspecified", None, "none", False, "block"),
    ("link-local", "native", "kernel_native_inference_adapter", "link_local", None, "none", False, "block"),
    ("lan", "native", "kernel_native_inference_adapter", "private_lan", None, "none", False, "block"),
    ("multicast", "native", "kernel_native_inference_adapter", "multicast", None, "none", False, "block"),
    ("container", "native", "kernel_native_inference_adapter", "container_network", None, "none", False, "block"),
    ("proxy", "native", "kernel_native_inference_adapter", "proxy", None, "none", False, "block"),
    ("dns-override", "native", "kernel_native_inference_adapter", "dns", None, "none", False, "block"),
    ("external", "native", "kernel_native_inference_adapter", "external", None, "none", False, "block"),
    ("unknown", "native", "kernel_native_inference_adapter", "unknown", None, "none", False, "block"),
    ("endpoint-substitution", "native", "kernel_native_inference_adapter", "authenticated_local_socket", "authenticated_unix_socket", "mismatch", True, "block"),
)
FIXTURE_PROFILE: Final = {
    "schema_version": 1,
    "ip_case_count": 13,
    "policy_case_count": 15,
    "allowed_policy_case_count": 2,
    "blocked_policy_case_count": 13,
    "filesystem_magic_case_count": 22,
    "provider_component_count": 18,
    "root_sentinel_count": 3,
    "approved_topologies": [
        "native-authenticated-unix-socket",
        "docker-authenticated-guarded-loopback",
    ],
}
CLAIMS: Final = {
    "cross_layer_fixture_matrix_executed": True,
    "only_exact_declared_inference_transports_accepted": True,
    "direct_vscode_and_tool_worker_inference_blocked": True,
    "lan_container_proxy_dns_and_external_destinations_blocked": True,
    "remote_fuse_unknown_and_sync_storage_fixtures_rejected": True,
    "live_remote_mount_tested": False,
    "every_sync_client_detected": False,
    "external_network_used": False,
    "inference_performed": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Remote, FUSE, and unknown filesystems are exact deterministic magic-value fixtures; no live NFS, CIFS, 9P, AFS, Ceph, NCP, Coda, or FUSE mount is claimed.",
    "Cloud-synchronization cases use disposable owner-only directories for the declared provider and sentinel corpus and do not claim detection of every client or deliberately disguised synchronizer.",
    "The two accepted cases are mutually exclusive configured deployment alternatives; acceptance requires the exact adapter, destination, transport, endpoint digest, and authenticated peer for that topology.",
    "No product workflow, 60-minute observation, model inference, physical network, cross-platform certification, or release acceptance is claimed.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class ClassificationEvidenceError(ValueError):
    """Raised when classification evidence is incomplete or overstated."""


def expected_boundary_fixture() -> dict[str, Any]:
    return {
        "ip_cases": [
            {"address": address, "expected": expected, "id": identifier}
            for identifier, address, expected in IP_CASES
        ],
        "policy_cases": [
            {
                "component": component,
                "destination": destination,
                "endpoint": endpoint,
                "expected": expected,
                "id": identifier,
                "peer_authenticated": authenticated,
                "topology": topology,
                "transport": transport,
            }
            for (
                identifier,
                topology,
                component,
                destination,
                transport,
                endpoint,
                authenticated,
                expected,
            ) in POLICY_CASES
        ],
        "schema_version": 1,
        "storage_fixture": STORAGE_FIXTURE_PATH,
    }


def validate_boundary_fixture(value: Any) -> list[str]:
    return [] if value == expected_boundary_fixture() else ["boundary fixture changed"]


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
        raise ClassificationEvidenceError("source revision is unavailable")
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
        raise ClassificationEvidenceError("committed source is unavailable")
    return completed.stdout


def validate_committed_fixture(revision: str) -> str:
    data = git_bytes(revision, BOUNDARY_FIXTURE_PATH)
    try:
        value = json.loads(data)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ClassificationEvidenceError("committed fixture is invalid") from error
    if errors := validate_boundary_fixture(value):
        raise ClassificationEvidenceError("; ".join(errors))
    return hashlib.sha256(data).hexdigest()


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
    if completed.returncode != 0 or marker not in completed.stdout + completed.stderr:
        raise ClassificationEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "strict-local-cross-layer-classification",
        "source_revision": revision,
        "task_ids": ["10.1.3.1", "S-010-UT01"],
        "status": "pass-strict-local-cross-layer-fixture-matrix",
        "boundary_fixture_sha256": validate_committed_fixture(revision),
        "fixture_profile": FIXTURE_PROFILE,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "sources": source_records(revision),
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    exact = {
        "schema_version": 1,
        "artifact_id": "strict-local-cross-layer-classification",
        "task_ids": ["10.1.3.1", "S-010-UT01"],
        "status": "pass-strict-local-cross-layer-fixture-matrix",
        "fixture_profile": FIXTURE_PROFILE,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            errors.append(f"classification evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        errors.append("classification source revision is invalid")
    if SHA256.fullmatch(str(report.get("boundary_fixture_sha256", ""))) is None:
        errors.append("classification fixture identity is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        errors.append("classification source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        errors.append("classification source records are invalid")
    else:
        fixture_record = sources[SOURCE_PATHS.index(BOUNDARY_FIXTURE_PATH)]
        if fixture_record["sha256"] != report.get("boundary_fixture_sha256"):
            errors.append("classification fixture identity changed")
    return errors


def validate_committed_sources(report: dict[str, Any]) -> None:
    revision = str(report["source_revision"])
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(revision, item["path"])).hexdigest() != item["sha256"]:
            raise ClassificationEvidenceError("committed source binding changed")
    if validate_committed_fixture(revision) != report["boundary_fixture_sha256"]:
        raise ClassificationEvidenceError("committed fixture binding changed")


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ClassificationEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise ClassificationEvidenceError("evidence report is invalid")
    return report


def write_atomic(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        report = build_report(git_revision(arguments.source_revision))
        if errors := validate_report(report):
            raise ClassificationEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise ClassificationEvidenceError("; ".join(errors))
    validate_committed_sources(report)
    print("Strict-local cross-layer classification evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

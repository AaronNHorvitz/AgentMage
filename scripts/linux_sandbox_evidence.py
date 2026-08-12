#!/usr/bin/env python3
"""Run and bind Fedora IPC, sandbox, seccomp, and cgroup evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import stat
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-9"
    / "story-9.1"
    / "linux-control-verification.json"
)
SOURCE_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "docs/architecture/linux-platform-lifecycle.md",
    "docs/architecture/linux-worker-isolation.md",
    "docs/architecture/platform-adapter-contract.md",
    "kernel/engine/src/platform_startup.rs",
    "platforms/linux/Cargo.toml",
    "platforms/linux/src/ipc.rs",
    "platforms/linux/src/lib.rs",
    "platforms/linux/src/platform.rs",
    "platforms/linux/src/sandbox.rs",
    "platforms/linux/src/security_controls.rs",
    "platforms/linux/src/secret_service.rs",
    "scripts/linux_sandbox_evidence.py",
    "tests/test_linux_sandbox_evidence.py",
)
SANDBOX_TESTS = (
    "bounded_scratch_cannot_escape_into_the_held_object_or_host_workspace",
    "directory_projection_never_contains_excluded_or_nested_workspace_content",
    "directory_worker_receives_only_the_bounded_exclusion_safe_projection",
    "foreign_workspace_identity_never_starts_a_worker",
    "file_projection_is_the_approved_preimage_and_cannot_be_modified",
    "fresh_worker_reads_only_the_canonical_workspace_file",
    "limits_and_manifests_fail_closed",
    "policy_compiles_to_nonempty_classic_bpf",
    "stale_held_directory_fails_projection_before_any_worker_process_can_start",
    "stale_held_file_fails_before_any_worker_process_can_start",
    "transient_service_terminates_an_unbounded_worker",
    "worker_cannot_resolve_sibling_parent_or_hidden_descriptor_content",
    "worker_cannot_write_the_read_only_workspace",
    "worker_has_no_ambient_host_paths_devices_or_processes",
    "worker_kernel_status_confirms_no_new_privileges_and_seccomp",
    "worker_output_is_drained_but_never_retained_past_the_bound",
    "worker_receives_only_the_fixed_environment_and_no_network",
)
SANDBOX_STATIC_TESTS = (
    "directory_projection_never_contains_excluded_or_nested_workspace_content",
    "file_projection_is_the_approved_preimage_and_cannot_be_modified",
    "limits_and_manifests_fail_closed",
    "policy_compiles_to_nonempty_classic_bpf",
    "stale_held_directory_fails_projection_before_any_worker_process_can_start",
    "stale_held_file_fails_before_any_worker_process_can_start",
)
SANDBOX_LIVE_TESTS = tuple(
    name for name in SANDBOX_TESTS if name not in SANDBOX_STATIC_TESTS
)
IPC_TESTS = (
    "authenticated_endpoint_exposes_only_bounded_product_frames",
    "every_peer_and_frame_mutation_fails_without_consuming_valid_request",
    "exact_peer_authenticates_once_and_replay_fails",
    "generated_launch_material_is_fresh_and_redacted",
    "listener_drop_removes_only_its_unchanged_socket_identity",
    "private_socket_uses_kernel_peer_credentials_and_exact_frame",
    "process_start_parser_handles_parentheses_and_rejects_malformed_records",
    "unsafe_parent_existing_socket_and_short_frame_fail_closed",
)
SECRET_SERVICE_TESTS = (
    "key_debug_and_errors_never_disclose_candidate_content",
    "keys_values_and_manifests_are_bounded_and_redacted",
    "operational_store_key_decode_is_exact_and_bounded",
    "sensitive_output_is_bounded_without_a_content_digest",
)
SECRET_SERVICE_LIVE_TESTS = (
    "live_operational_key_provisioning_is_exact_non_overwriting_and_cleaned",
    "live_service_probe_returns_only_a_content_free_receipt",
    "live_service_round_trip_is_exact_and_cleanup_is_verified",
)
STARTUP_CONTROL_IDS = (
    "bubblewrap",
    "user-namespaces",
    "seccomp",
    "cgroups-v2",
    "secret-service",
    "descriptor-safe-paths",
    "network-isolation",
)
STARTUP_MAPPING_TESTS = (
    "control_identities_are_nonzero_distinct_and_status_independent",
    "every_linux_control_disablement_maps_to_fail_closed_startup_capabilities",
)
KERNEL_STARTUP_TESTS = ("every_missing_invalid_or_substituted_mechanism_refuses",)
STARTUP_LIVE_TESTS = (
    "live_required_control_preflight_verifies_without_workspace_authority",
)
TOOLS = {
    "bubblewrap": Path("/usr/bin/bwrap"),
    "secret-tool": Path("/usr/bin/secret-tool"),
    "systemd-run": Path("/usr/bin/systemd-run"),
}
CONTROLS = {
    "ipc": "mode-0600-unix-socket-peercred-executable-hmac-fresh-challenge",
    "workspace_transfer": "systemd-openfile-held-descriptors-to-bwrap-ro-bind-fd",
    "namespaces": "fresh-user-mount-pid-ipc-uts-cgroup-network",
    "network": "new-network-namespace-plus-address-family-and-seccomp-denial",
    "workspace": "single-authorized-root-read-only",
    "scratch": "private-tmpfs-16777216-bytes",
    "environment": ["LANG=C", "PATH=/app", "PWD=/workspace"],
    "privileges": "no-new-privileges-private-devices-cap-drop-all",
    "seccomp": "agentmage.linux.worker.deny.v1-kernel-mode-filter",
    "cgroup": [
        "CPUQuota",
        "MemoryMax",
        "MemorySwapMax=0",
        "RuntimeMaxSec",
        "TasksMax",
    ],
    "output": "concurrent-drain-bounded-retention-sha256-diagnostics",
    "secret_service": "fixed-attributes-pipe-only-zeroized-bounded-watchdog-client",
}
ATTACKS = (
    "ambient_home",
    "ambient_system_and_runtime_roots",
    "directory_exclusion_bypass",
    "device",
    "environment_inheritance",
    "foreign_workspace_identity",
    "host_processes",
    "network",
    "output_exhaustion",
    "scratch_escape",
    "secret_store_reachability",
    "sibling_parent_descriptor_resolution",
    "stale_directory_preimage",
    "stale_file_preimage",
    "ungranted_root",
    "workspace_write",
)
PLATFORM_STATUS = {
    "fedora_44_x86_64": "verified-local",
    "ubuntu_26_04_x86_64": "bounded-sandbox-evidence-recorded-separately",
    "macos": "blocked-macos",
}
LIMITATIONS = [
    "Clean package lifecycle is verified separately on Fedora and Ubuntu; bounded Ubuntu Bubblewrap, seccomp, and cgroup attack evidence is also recorded separately, while native Ubuntu execution and Ubuntu live Secret Service closure remain pending.",
    "The seven-control startup preflight passes natively on Fedora and its fail-closed mapping is exercised for Fedora and Ubuntu identities; native Ubuntu preflight execution remains pending.",
    "The inactive inference package boundary is verified separately; enabled inference runtime remains pending.",
    "macOS implementation and execution remain blocked and are not substituted.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class LinuxControlEvidenceError(ValueError):
    """Raised when Linux control evidence is unavailable, stale, or overclaimed."""


def pretty_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-linux-controls-", dir=path.parent)
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


def run_command(argv: list[str], timeout: int = 240) -> str:
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    completed = subprocess.run(
        argv,
        cwd=ROOT,
        env=environment,
        check=False,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
    )
    if completed.returncode != 0:
        raise LinuxControlEvidenceError(f"verification command failed: {argv[0]}")
    return completed.stdout


def git_revision(candidate: str = "HEAD") -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise LinuxControlEvidenceError("source revision is unavailable")
    return revision


def git_file(revision: str, relative: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=10,
    )
    if completed.returncode != 0:
        raise LinuxControlEvidenceError(f"committed source is absent: {relative}")
    return completed.stdout


def os_release() -> dict[str, str]:
    values: dict[str, str] = {}
    for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            values[key] = value.strip('"')
    return values


def verify_fedora_host() -> dict[str, str]:
    release = os_release()
    architecture = platform.machine()
    if release.get("ID") != "fedora" or release.get("VERSION_ID") != "44":
        raise LinuxControlEvidenceError("host is not the declared Fedora 44 platform")
    if architecture != "x86_64":
        raise LinuxControlEvidenceError("host is not the declared x86_64 architecture")
    mountinfo = Path("/proc/self/mountinfo").read_text(encoding="utf-8")
    if not any(" - cgroup2 " in line for line in mountinfo.splitlines()):
        raise LinuxControlEvidenceError("cgroup v2 is unavailable")
    return {
        "distribution": "fedora",
        "version": "44",
        "architecture": architecture,
        "cgroup_filesystem": "cgroup2",
    }


def tool_record(name: str, path: Path) -> dict[str, Any]:
    metadata = path.stat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_uid != 0
        or metadata.st_mode & 0o022
    ):
        raise LinuxControlEvidenceError(f"trusted tool identity is invalid: {name}")
    if name == "secret-tool":
        version_output = run_command(
            [
                "/usr/bin/rpm",
                "-qf",
                "--queryformat",
                "%{NAME} %{VERSION}-%{RELEASE}.%{ARCH}\\n",
                str(path),
            ],
            timeout=30,
        )
    else:
        version_output = run_command([str(path), "--version"], timeout=30)
    version = version_output.splitlines()[0].strip()
    if not version or len(version) > 160:
        raise LinuxControlEvidenceError(f"trusted tool version is invalid: {name}")
    return {
        "id": name,
        "path_class": "root-owned-usr-bin",
        "owner_uid": 0,
        "group_or_world_writable": False,
        "sha256": sha256_bytes(path.read_bytes()),
        "version": version,
    }


def observed_tests(
    filter_name: str,
    expected: tuple[str, ...],
    *,
    ignored: bool = False,
    package: str = "agentmage-platform-linux",
) -> list[dict[str, str]]:
    test_arguments = ["--test-threads=1"]
    if ignored:
        test_arguments.insert(0, "--ignored")
    output = run_command(
        [
            "cargo",
            "test",
            "--locked",
            "--offline",
            "-p",
            package,
            filter_name,
            "--",
            *test_arguments,
        ]
    )
    observed = {
        match.group(1)
        for line in output.splitlines()
        if (match := re.fullmatch(r"test .*::([^: ]+) \.\.\. ok", line))
    }
    missing = set(expected) - observed
    if missing:
        raise LinuxControlEvidenceError(f"verification test closure is incomplete: {filter_name}")
    return [{"test": name, "status": "pass"} for name in expected]


def source_records(revision: str) -> list[dict[str, str]]:
    records = []
    for relative in SOURCE_PATHS:
        committed = git_file(revision, relative)
        if committed != (ROOT / relative).read_bytes():
            raise LinuxControlEvidenceError(f"source differs from revision: {relative}")
        records.append({"path": relative, "sha256": sha256_bytes(committed)})
    return records


def build_report(revision: str) -> dict[str, Any]:
    host = verify_fedora_host()
    tools = [tool_record(name, path) for name, path in TOOLS.items()]
    observed_tests("sandbox::tests", SANDBOX_STATIC_TESTS)
    observed_tests("sandbox::tests", SANDBOX_LIVE_TESTS, ignored=True)
    sandbox_tests = [{"test": name, "status": "pass"} for name in SANDBOX_TESTS]
    ipc_tests = observed_tests("ipc::tests", IPC_TESTS)
    secret_service_tests = observed_tests("secret_service::tests", SECRET_SERVICE_TESTS)
    secret_service_live_tests = observed_tests(
        "secret_service::tests::live_",
        SECRET_SERVICE_LIVE_TESTS,
        ignored=True,
    )
    startup_mapping_tests = [
        *observed_tests(
            STARTUP_MAPPING_TESTS[0],
            STARTUP_MAPPING_TESTS[:1],
        ),
        *observed_tests(
            STARTUP_MAPPING_TESTS[1],
            STARTUP_MAPPING_TESTS[1:],
        ),
    ]
    kernel_startup_tests = observed_tests(
        KERNEL_STARTUP_TESTS[0],
        KERNEL_STARTUP_TESTS,
        package="agentmage-kernel-engine",
    )
    startup_live_tests = observed_tests(
        STARTUP_LIVE_TESTS[0],
        STARTUP_LIVE_TESTS,
        ignored=True,
    )
    return {
        "schema_version": 1,
        "artifact_id": "linux-ipc-sandbox-control-verification",
        "task_ids": [
            "9.1.1.5",
            "9.1.2.2",
            "9.1.2.3",
            "9.1.3.1",
            "9.1.3.2",
            "9.1.3.3",
        ],
        "test_ids": ["S-009-UT01", "S-009-ST01", "S-009-UT02"],
        "status": "pass-fedora-only",
        "source_revision": revision,
        "sources": source_records(revision),
        "host": host,
        "trusted_tools": tools,
        "controls": CONTROLS,
        "attack_coverage": {attack: "pass" for attack in ATTACKS},
        "sandbox_tests": sandbox_tests,
        "ipc_tests": ipc_tests,
        "secret_service_tests": secret_service_tests,
        "secret_service_live_tests": secret_service_live_tests,
        "startup_control_ids": list(STARTUP_CONTROL_IDS),
        "startup_mutation_statuses": ["unavailable", "invalid"],
        "startup_platform_families": ["fedora", "ubuntu"],
        "startup_mapping_tests": startup_mapping_tests,
        "kernel_startup_tests": kernel_startup_tests,
        "startup_live_tests": startup_live_tests,
        "platform_status": PLATFORM_STATUS,
        "private_values_present": False,
        "macos_evidence_substituted": False,
        "release_claim": "none",
        "limitations": LIMITATIONS,
    }


def validate_report(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict):
        return ["Linux control report must be an object"]
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-ipc-sandbox-control-verification"
        or value.get("status") != "pass-fedora-only"
        or REVISION.fullmatch(str(value.get("source_revision"))) is None
    ):
        failures.append("Linux control report identity changed")
    if value.get("task_ids") != [
        "9.1.1.5",
        "9.1.2.2",
        "9.1.2.3",
        "9.1.3.1",
        "9.1.3.2",
        "9.1.3.3",
    ]:
        failures.append("Linux control task mapping changed")
    if value.get("test_ids") != ["S-009-UT01", "S-009-ST01", "S-009-UT02"]:
        failures.append("Linux control test mapping changed")
    if value.get("host") != {
        "distribution": "fedora",
        "version": "44",
        "architecture": "x86_64",
        "cgroup_filesystem": "cgroup2",
    }:
        failures.append("Fedora host identity changed")
    tools = value.get("trusted_tools")
    if not isinstance(tools, list) or [item.get("id") for item in tools] != list(TOOLS):
        failures.append("trusted Linux tool closure changed")
    elif any(
        item.get("owner_uid") != 0
        or item.get("group_or_world_writable") is not False
        or SHA256.fullmatch(str(item.get("sha256"))) is None
        for item in tools
    ):
        failures.append("trusted Linux tool evidence is invalid")
    if value.get("controls") != CONTROLS:
        failures.append("Linux worker control inventory changed")
    sandbox = value.get("sandbox_tests")
    ipc = value.get("ipc_tests")
    if (
        not isinstance(sandbox, list)
        or [item.get("test") for item in sandbox] != list(SANDBOX_TESTS)
        or any(item.get("status") != "pass" for item in sandbox)
    ):
        failures.append("sandbox test closure is incomplete")
    if (
        not isinstance(ipc, list)
        or [item.get("test") for item in ipc] != list(IPC_TESTS)
        or any(item.get("status") != "pass" for item in ipc)
    ):
        failures.append("IPC test closure is incomplete")
    secret_service = value.get("secret_service_tests")
    secret_service_live = value.get("secret_service_live_tests")
    if (
        not isinstance(secret_service, list)
        or [item.get("test") for item in secret_service] != list(SECRET_SERVICE_TESTS)
        or any(item.get("status") != "pass" for item in secret_service)
    ):
        failures.append("Secret Service unit test closure is incomplete")
    if (
        not isinstance(secret_service_live, list)
        or [item.get("test") for item in secret_service_live]
        != list(SECRET_SERVICE_LIVE_TESTS)
        or any(item.get("status") != "pass" for item in secret_service_live)
    ):
        failures.append("Secret Service live test closure is incomplete")
    if value.get("startup_control_ids") != list(STARTUP_CONTROL_IDS):
        failures.append("Linux startup control closure changed")
    if value.get("startup_mutation_statuses") != ["unavailable", "invalid"]:
        failures.append("Linux startup mutation closure changed")
    if value.get("startup_platform_families") != ["fedora", "ubuntu"]:
        failures.append("Linux startup platform matrix changed")
    for field, expected, failure in (
        (
            "startup_mapping_tests",
            STARTUP_MAPPING_TESTS,
            "Linux startup mapping tests are incomplete",
        ),
        (
            "kernel_startup_tests",
            KERNEL_STARTUP_TESTS,
            "kernel startup refusal tests are incomplete",
        ),
        (
            "startup_live_tests",
            STARTUP_LIVE_TESTS,
            "Linux live startup preflight is incomplete",
        ),
    ):
        records = value.get(field)
        if (
            not isinstance(records, list)
            or [item.get("test") for item in records] != list(expected)
            or any(item.get("status") != "pass" for item in records)
        ):
            failures.append(failure)
    attacks = value.get("attack_coverage")
    if attacks != {attack: "pass" for attack in ATTACKS}:
        failures.append("Linux attack coverage is incomplete")
    if value.get("platform_status") != PLATFORM_STATUS:
        failures.append("Linux control platform status changed")
    if (
        value.get("private_values_present") is not False
        or value.get("macos_evidence_substituted") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("Linux control report made a private or unsupported claim")
    sources = value.get("sources")
    if (
        not isinstance(sources, list)
        or [item.get("path") for item in sources] != list(SOURCE_PATHS)
        or any(SHA256.fullmatch(str(item.get("sha256"))) is None for item in sources)
    ):
        failures.append("Linux control source closure is incomplete")
    if value.get("limitations") != LIMITATIONS:
        failures.append("Linux control limitations are incomplete")
    return failures


def write_report(revision: str) -> None:
    write_atomic(REPORT_PATH, pretty_json(build_report(revision)))


def check_report() -> None:
    try:
        actual = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        revision = actual["source_revision"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise LinuxControlEvidenceError(f"cannot read Linux control report: {error}") from error
    failures = validate_report(actual)
    if not isinstance(revision, str) or actual != build_report(revision):
        failures.append("Linux control report is stale or malformed")
    if failures:
        raise LinuxControlEvidenceError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.write:
            write_report(git_revision(arguments.source_revision))
        check_report()
    except (OSError, UnicodeError, LinuxControlEvidenceError, subprocess.SubprocessError) as error:
        print(f"Linux control evidence failed: {error}", file=sys.stderr)
        return 1
    print("Fedora IPC, Bubblewrap, seccomp, cgroup, and attack evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
